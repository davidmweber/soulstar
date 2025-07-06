# Soulstar

This is a wearable art piece that lets you know when your friends are close at events like festivals. It directly
inspired by the [HiveMind proximity detector](https://cpbotha.net/2024/08/10/afrikaburn-2018-hivemind-proximity/) that
was built by two close friends and tested at [AfrikaBurn](https://www.afrikaburn.org/) in 2018. Tech has moved on a bit since then so this is
effectively V2. You can see the original project source code [here](https://typst.app/docs/reference/model/bibliography/)
courtesy of cpbotha.

## Some disclaimers
I started the project using the Espressif IDF (a powerful tool indeed) but switched over to 
[Rust](https://www.rust-lang.org) for several reasons:
- I like Rust. 
- The embedded ecosystem is surprisingly strong. I particularly like [Embassy](https://embassy.dev/) with its great
  support for ESP32 hardware and its async capabilities. In particular, the Embassy tasks and connector tools make
  dealing with asynchronous events safe and easy.
- I really wanted to get a feel for how mature the embedded ecosystem is with a view to another commercial product
  I am involved with.
- Time is something I on my hands right now and am happy to struggle a bit, possibly even contribute back to the 
  ecosystem.
- I chose the `#[no_std]` (i.e. no Espressif IDF) option, mostly to feel out the ecosystem in a pure Rust world. This
  is an art piece after all.
- Async has some significant advantages, particularly with stack management and efficient concurrency, so I have
  made some effort to use async interfaces wherever possible.

## Hardware Requirements
This project is based around the ESP32 family of embedded microprocessors with some basic requirements and a standard
LS2812 LED strip.
- ESP32C6 dev board. I chose the ESP32C6 because it has a v5.0 BlueTooth stack that includes BLE as well as an 802.15
  stack which supports the ZigBee proximity detection used in [HiveMind](https://cpbotha.net/2024/08/10/afrikaburn-2018-hivemind-proximity/).
  I also prefer the [RISC-V](https://en.wikipedia.org/wiki/RISC-V) instruction set over Xtensa mostly because RISC-V is
  directly supported by LLVM and the Rust compiler. You can use pretty much any ESP32 processor that supports RMT and 
  BLE. In particular, the ESP32-S3 and ESP32-H2 support DMA for the RMI interface used to drive the LED string which
  is a better option for long LED strings.
- WS2812 LED strip. These strips are ubiquitous and have great driver support. I used a ring LED strip for development.
- Logic level shifter. The ESP32 devices are 3.3V while the led strips come in 5V or 12V so some logic level shifting
  is required. As cpbotha correctly reports, this gets fiddly and can be brittle in the field.

## Software requirements
As this is a `#[no_std]` project, you do not need the Espressif IDF environment installed. You just need Rust and 
compilers for your targeted hardware. The tools you will need are:
- [A Rust development environment](https://www.rust-lang.org/learn/get-started). Just install and move on.
- A suitable toolchain. Install this for the RISC-V targets using:
  ```shell
  rustup target add riscv32imac-unknown-none-elf
  ```
  If you have an Xtensa device, you need to follow [these instructions](https://docs.esp-rs.org/book/installation/riscv-and-xtensa.html)
  to install the required targets as they are not (yet) part of the supported targets in Rust..
- [probe_rs](https://probe.rs/). This is used to flash and debug the device. Install using 
  ```shell
  cargo install probe-rs-tools
  ```
- [Wokwi](https://wokwi.com/) is a useful way to test without flashing a device all the time. There is a fully working
  Wokwi setup in the repo. You will need to get an account and set up your IDE to use it. There are plugins for VSCode
  and Jetbrains.

## Building, configuration and running
Builds are mostly managed by cargo, but we use the awesome [just](https://github.com/casey/just) tool to automate
some of the builds. Running `just --list` will show all the available tasks.

The devices are custom flashed per user from the configuration file [souls.toml]. For this example:
```toml
[[device]]
id = "nefario"
bt_name = "Dr Nefario"
color = [0xFF, 0x00, 0x00]

[[device]]
id = "strange"
bt_name = "Dr Strange"
color = [0x00, 0xFF, 0x00]

[[device]]
id = "who"
bt_name = "Dr Who"
color = [0x00, 0x00, 0xFF]

```
we have three souls that have an ID, bluetooth advertisement name and a desired colour. You configure the device by
setting the `SOUL_ID` environment variables to one of the id's above which will generate [src/soul_config.rs] which
hardcodes the details into the build. The easiest way to flash a device for a specific person is to use `just`:
```shell
just flash peter # Will flash the device with a bluetooth name "Dr Krekel" and colour blue. 
```
You must specify the environment variable `SOUL_ID` at compile time else the compilation will fail.

## Useful links

- [ESP32-C6 esp_hal documention](https://docs.esp-rs.org/esp-hal/esp-hal/0.23.1/esp32c6/esp_hal/)
- [Rust on ESP book](https://docs.esp-rs.org/book/)

# TODO
- [x] Set up some device configuration from a file so we can easily set up stuff like GPIO pins for the string,
      BLE advertisement transmitter power and stuff like that.
- [x] Personalise the device name using a flash partition (I used another scheme)
- [x] Change the logging to use defmt
- [x] Fix the Wokwi emulator


# Learnings
 - The Rust embedded ecosystem is potent but immature. That being said, it is actually really nice to work with and
   is rapidly evolving.
 - Make sure that you set suitable interval and window value for the BLE scanner, especially if you are 
   advertising. In particular, the *interval* value must be greater than *window* else the stack just crashes
   at some point. 