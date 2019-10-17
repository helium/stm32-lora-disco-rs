
use crate::hal::prelude::*;
use hal::device;
use hal::gpio::*;
use hal::rcc::Rcc;
use hal::spi;
use hal::exti;
use hal::pac;
use longfi_device::{AntPinsMode, BoardBindings, Spi};
use nb::block;

#[allow(dead_code)]
pub struct LongFiBindings {
    pub bindings: BoardBindings
}

type Uninitialized = Input<Floating>;

pub type RadioIRQ = gpiob::PB4<Input<PullUp>>;

pub fn initializeIrq(pin: gpiob::PB4<Uninitialized>, syscfg: &mut hal::syscfg::SYSCFG, exti: &mut pac::EXTI) -> gpiob::PB4<Input<PullUp>> {

        let dio0 = pin.into_pull_up_input();

        exti.listen(
            syscfg,
            dio0.port,
            dio0.i,
            exti::TriggerEdge::Rising,
        );

        dio0
}

impl LongFiBindings {
    pub const fn new() -> LongFiBindings {
        LongFiBindings {
            bindings: BoardBindings {
                reset: Some(radio_reset),
                spi_in_out: Some(spi_in_out),
                spi_nss: Some(spi_nss),
                delay_ms: Some(delay_ms),
                get_random_bits: Some(get_random_bits),
                set_antenna_pins: Some(set_antenna_pins),
                set_board_tcxo: None,
                busy_pin_status: None,
                reduce_power: None,
            }
        }
    }

    pub fn init<T: embedded_hal::digital::v2::OutputPin>(
        &self,
        spi_peripheral: device::SPI1,
        rcc: &mut Rcc,
        spi_sck: gpiob::PB3<Uninitialized>,
        spi_miso: gpioa::PA6<Uninitialized>,
        spi_mosi: gpioa::PA7<Uninitialized>,
        spi_nss: gpioa::PA15<Uninitialized>,
        reset: gpioc::PC0<Uninitialized>,
        rx: gpioa::PA1<Uninitialized>,
        tx_rfo: gpioc::PC2<Uninitialized>,
        tx_boost: gpioc::PC1<Uninitialized>,
        tcxo_en: Option<T>
    ){
        // store all of the necessary pins and peripherals into statics
        // this is necessary as the extern C functions need access
        // this is safe, thanks to ownership and because these statics are private
        unsafe {
            SPI = Some(spi_peripheral.spi(
                (spi_sck, spi_miso, spi_mosi),
                spi::MODE_0,
                1_000_000.hz(),
                rcc,
            ));
            SPI_NSS = Some(spi_nss.into_push_pull_output());
            RESET = Some(reset.into_push_pull_output());
            ANT_SW = Some(AntennaSwitches::new(
                rx.into_push_pull_output(),
                tx_rfo.into_push_pull_output(),
                tx_boost.into_push_pull_output(),
                ));
        };
    }
}

type SpiPort = hal::spi::Spi<
    hal::pac::SPI1,
    (
        hal::gpio::gpiob::PB3<hal::gpio::Input<hal::gpio::Floating>>,
        hal::gpio::gpioa::PA6<hal::gpio::Input<hal::gpio::Floating>>,
        hal::gpio::gpioa::PA7<hal::gpio::Input<hal::gpio::Floating>>,
    ),
>;
static mut SPI: Option<SpiPort> = None;
#[no_mangle]
extern "C" fn spi_in_out(_s: *mut Spi, out_data: u8) -> u8 {
    unsafe {
        if let Some(spi) = &mut SPI {
            spi.send(out_data).unwrap();
            let in_data = block!(spi.read()).unwrap();
            in_data
        } else {
            0
        }
    }
}

static mut SPI_NSS: Option<gpioa::PA15<Output<PushPull>>> = None;
#[no_mangle]
extern "C" fn spi_nss(value: bool) {
    unsafe {
        if let Some(pin) = &mut SPI_NSS {
            if value {
                pin.set_high().unwrap();
            } else {
                pin.set_low().unwrap();
            }
        }
    }
}

static mut RESET: Option<gpioc::PC0<Output<PushPull>>> = None;
#[no_mangle]
extern "C" fn radio_reset(value: bool) {
    unsafe {
        if let Some(pin) = &mut RESET {
            if value {
                pin.set_low().unwrap();
            } else {
                pin.set_high().unwrap();
            }
        }
    }
}

#[no_mangle]
extern "C" fn delay_ms(ms: u32) {
    cortex_m::asm::delay(ms);
}

#[no_mangle]
extern "C" fn get_random_bits(_bits: u8) -> u32 {
    0x1
}

pub struct AntennaSwitches<Rx, TxRfo, TxBoost> {
    rx: Rx,
    tx_rfo: TxRfo,
    tx_boost: TxBoost,
}

impl<Rx, TxRfo, TxBoost> AntennaSwitches<Rx, TxRfo, TxBoost>
where
    Rx: embedded_hal::digital::v2::OutputPin,
    TxRfo: embedded_hal::digital::v2::OutputPin,
    TxBoost: embedded_hal::digital::v2::OutputPin,
{
    pub fn new(rx: Rx, tx_rfo: TxRfo, tx_boost: TxBoost) -> AntennaSwitches<Rx, TxRfo, TxBoost> {
        AntennaSwitches {
            rx,
            tx_rfo,
            tx_boost,
        }
    }

    pub fn set_sleep(&mut self) {
        self.rx.set_low();
        self.tx_rfo.set_low();
        self.tx_boost.set_low();
    }

    pub fn set_tx(&mut self) {
        self.rx.set_low();
        self.tx_rfo.set_low();
        self.tx_boost.set_high();
    }

    pub fn set_rx(&mut self) {
        self.rx.set_high();
        self.tx_rfo.set_low();
        self.tx_boost.set_low();
    }
}

type AntSw = AntennaSwitches<
    stm32l0xx_hal::gpio::gpioa::PA1<stm32l0xx_hal::gpio::Output<stm32l0xx_hal::gpio::PushPull>>,
    stm32l0xx_hal::gpio::gpioc::PC2<stm32l0xx_hal::gpio::Output<stm32l0xx_hal::gpio::PushPull>>,
    stm32l0xx_hal::gpio::gpioc::PC1<stm32l0xx_hal::gpio::Output<stm32l0xx_hal::gpio::PushPull>>,
>;

static mut ANT_SW: Option<AntSw> = None;

pub fn set_antenna_switch(pin: AntSw) {
    unsafe {
        ANT_SW = Some(pin);
    }
}

pub extern "C" fn set_antenna_pins(mode: AntPinsMode, power: u8) {
    unsafe {
        if let Some(ant_sw) = &mut ANT_SW {
            match mode {
                AntPinsMode::AntModeTx => {
                    ant_sw.set_tx();
                }
                AntPinsMode::AntModeRx => {
                    ant_sw.set_rx();
                }
                AntPinsMode::AntModeSleep => {
                    ant_sw.set_sleep();
                }
                _ => (),
            }
        }
    }
}