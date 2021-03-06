//! Rust Board Support Crate (BSC) for the STM32 LoRa Discovery Board (B-L072Z-LRWAN1)
#![no_std]
pub use cmwx1zzabz::hal;

pub type DebugUsart = hal::serial::USART2;

pub use cmwx1zzabz::initialize_radio_irq;
pub use cmwx1zzabz::LongFiBindings;
pub use cmwx1zzabz::RadioIRQ;
pub use cmwx1zzabz::TcxoEn;
