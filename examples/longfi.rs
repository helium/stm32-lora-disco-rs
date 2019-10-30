// To use example, press any key in serial terminal
// Packet will send and "Transmit Done!" will print when radio is done sending packet

#![cfg_attr(not(test), no_std)]
#![no_main]

extern crate nb;
extern crate panic_halt;

use hal::{pac, prelude::*, rcc, rng::Rng, serial, syscfg};
use rtfm::app;
use stm32l0xx_hal as hal;

use longfi_device;
use longfi_device::{ClientEvent, Config, LongFi, RadioType, RfEvent};

use core::fmt::Write;
use stm32_lora_disco;

static mut PRESHARED_KEY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

pub extern "C" fn get_preshared_key() -> *mut u8 {
    unsafe { &mut PRESHARED_KEY[0] as *mut u8 }
}

#[app(device = stm32l0xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        int: pac::EXTI,
        radio_irq: stm32_lora_disco::RadioIRQ,
        debug_uart: serial::Tx<stm32_lora_disco::DebugUsart>,
        uart_rx: serial::Rx<stm32_lora_disco::DebugUsart>,
        #[init([0;512])]
        buffer: [u8; 512],
        #[init(0)]
        count: u8,
        longfi: LongFi,
    }

    #[init(spawn = [send_ping], resources = [buffer])]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut BINDINGS: Option<stm32_lora_disco::LongFiBindings> = None;
        let device = ctx.device;

        let mut rcc = device.RCC.freeze(rcc::Config::hsi16());
        let mut syscfg = syscfg::SYSCFG::new(device.SYSCFG_COMP, &mut rcc);

        let gpioa = device.GPIOA.split(&mut rcc);
        let gpiob = device.GPIOB.split(&mut rcc);
        let gpioc = device.GPIOC.split(&mut rcc);

        let (tx_pin, rx_pin, serial_peripheral) = (gpioa.pa2, gpioa.pa3, device.USART2);

        let mut serial = serial_peripheral
            .usart((tx_pin, rx_pin), serial::Config::default(), &mut rcc)
            .unwrap();

        // listen for incoming bytes which will trigger transmits
        serial.listen(serial::Event::Rxne);
        let (mut tx, rx) = serial.split();

        write!(tx, "LongFi Device Test\r\n").unwrap();

        let mut exti = device.EXTI;
        let rng = Rng::new(device.RNG, &mut rcc, &mut syscfg, device.CRS);
        let radio_irq = stm32_lora_disco::initialize_radio_irq(gpiob.pb4, &mut syscfg, &mut exti);

        *BINDINGS = Some(stm32_lora_disco::LongFiBindings::new(
            device.SPI1,
            &mut rcc,
            rng,
            gpiob.pb3,
            gpioa.pa6,
            gpioa.pa7,
            gpioa.pa15,
            gpioc.pc0,
            gpioa.pa1,
            gpioc.pc2,
            gpioc.pc1,
            None,
        ));

        let rf_config = Config {
            oui: 1234,
            device_id: 5678,
            auth_mode: longfi_device::AuthMode::PresharedKey128,
        };

        let mut longfi_radio;
        if let Some(bindings) = BINDINGS {
            longfi_radio = unsafe {
                LongFi::new(
                    RadioType::Sx1276,
                    &mut bindings.bindings,
                    rf_config,
                    &PRESHARED_KEY,
                )
                .unwrap()
            };
        } else {
            panic!("No bindings exist");
        }

        longfi_radio.set_buffer(ctx.resources.buffer);

        write!(tx, "Going to main loop\r\n").unwrap();

        // Return the initialised resources.
        init::LateResources {
            int: exti,
            radio_irq,
            debug_uart: tx,
            uart_rx: rx,
            longfi: longfi_radio,
        }
    }

    #[task(capacity = 4, priority = 2, resources = [debug_uart, buffer, longfi])]
    fn radio_event(ctx: radio_event::Context, event: RfEvent) {
        let longfi_radio = ctx.resources.longfi;
        let client_event = longfi_radio.handle_event(event);

        match client_event {
            ClientEvent::ClientEvent_TxDone => {
                write!(ctx.resources.debug_uart, "Transmit Done!\r\n").unwrap();
            }
            ClientEvent::ClientEvent_Rx => {
                // get receive buffer
                let rx_packet = longfi_radio.get_rx();
                write!(ctx.resources.debug_uart, "Received packet\r\n").unwrap();
                write!(
                    ctx.resources.debug_uart,
                    "  Length =  {}\r\n",
                    rx_packet.len
                )
                .unwrap();
                write!(
                    ctx.resources.debug_uart,
                    "  Rssi   = {}\r\n",
                    rx_packet.rssi
                )
                .unwrap();
                write!(
                    ctx.resources.debug_uart,
                    "  Snr    =  {}\r\n",
                    rx_packet.snr
                )
                .unwrap();
                unsafe {
                    for i in 0..rx_packet.len {
                        write!(
                            ctx.resources.debug_uart,
                            "{:X} ",
                            *rx_packet.buf.offset(i as isize)
                        )
                        .unwrap();
                    }
                    write!(ctx.resources.debug_uart, "\r\n").unwrap();
                }
                // give buffer back to library
                longfi_radio.set_buffer(ctx.resources.buffer);
            }
            ClientEvent::ClientEvent_None => {}
        }
    }

    #[task(capacity = 4, priority = 2, resources = [debug_uart, count, longfi])]
    fn send_ping(ctx: send_ping::Context) {
        write!(ctx.resources.debug_uart, "Sending Ping\r\n").unwrap();
        let packet: [u8; 72] = [
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            *ctx.resources.count,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xa1,
            0xa2,
            0xa3,
            0xa4,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            *ctx.resources.count,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xa1,
            0xa2,
            0xa3,
            0xa4,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xDE,
            0xAD,
            0xBE,
            0xEF,
            0xa1,
            0xa2,
            0xa3,
            0xa4,
            0xBE,
            0xEF,
            0xa1,
            0xa2,
            0xa3,
            0xa4,
        ];
        *ctx.resources.count += 1;
        ctx.resources.longfi.send(&packet);
    }

    #[task(binds = USART1, priority=1, resources = [uart_rx], spawn = [send_ping])]
    fn USART1(ctx: USART1::Context) {
        let rx = ctx.resources.uart_rx;
        rx.read().unwrap();
        ctx.spawn.send_ping().unwrap();
    }

    #[task(binds = EXTI4_15, priority = 1, resources = [radio_irq, int], spawn = [radio_event])]
    fn EXTI4_15(ctx: EXTI4_15::Context) {
        ctx.resources.int.clear_irq(ctx.resources.radio_irq.i);
        ctx.spawn.radio_event(RfEvent::DIO0).unwrap();
    }

    // Interrupt handlers used to dispatch software tasks
    extern "C" {
        fn USART4_USART5();
    }
};
