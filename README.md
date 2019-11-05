# stm32-lora-disco-rs 

## About

This is an unofficial Board Support Crate (BSC) for using the [STMicroelectronics B-L072Z-LRWAN1 Discovery](https://www.st.com/en/evaluation-tools/b-l072z-lrwan1.html) with the [Rust] programming language.

It is currently in its early stages. Documentation is sparse and features are basic. Pull requests welcome!

[Rust]: https://www.rust-lang.org/


## Usage

### ST-Link & OpenOCD

The easiest way to flash code to the LoRa Discovery board is to use the on-board ST-Link and OpenOCD
- Using an external STLINK programmer via [OpenOCD]

Compiling the most recent version of OpenOCD is helpful, as support for this part has improved and most distributions provide a rather old version of OpenOCD.

To start the debug server, from this project directory, do:
`openocd -f ./openocd.cfg`

### JLink & JLinkServer

If you have a preference for JLink, you can actually [turn the ST-Link into a JLink debugger](https://www.segger.com/products/debug-probes/j-link/models/other-j-links/st-link-on-board/).

Download the [JLink server utility](https://www.segger.com/products/debug-probes/j-link/tools/j-link-gdb-server/about-j-link-gdb-server/) if you don't have it.

To run JLink server:
`JLinkGDBServer -device STM32L072CZ -speed 4000 -if swd -AutoConnect -1 -port 3333`


Open `.cargo/config` and uncomment the runner that matches your preferred configuration (comment all other ones). Then you can flash an example program like this:

```
cargo run --example longfi
```

[OpenOCD]: http://openocd.org/
