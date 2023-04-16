<p align="center">
  <img width=15% src="https://www.svgrepo.com/show/68860/microchip.svg">
  <h1 align="center">pico-bridge</h1>
</p>

The **pico-board** is an evaluation board that will be used to characterize the electrical, timing, power, and performance of the pico-bridge system on the RP2040. 

## Table of Contents
1. [Purpose](#requirements)
2. [Modules](#setup)
    1. [Power](#power)
    2. [RP2040](#rp2040)
    3. [Interfaces](#interfaces)
    4. [Reset](#reset)
3. [Usage](#usage)
    1. [Bringup](#bringup)
    2. [Flashing](#Flashing)
7. [Testing](#Testing)
8. [Debugging](#Debugging)
9. [Appendix](#appendix)

## Purpose
The purpose of this board is to enable testability and usage of the interfaces that the **pico-bridge** provides as an application on the RP2040. The generic Raspberry Pi Pico board has several design flaws that limit ease of use for use in our lab. This includes bringing out the supported DUT-Facing interfaces like SMI, SPI, JTAG, and UART onto headers to mate with commonly used DUTs such as our EVBs, adding an external reset circuit/button, and supporting various power configurations. 

It should also support the various Host-Facing interfaces that this application supports such as USB-Serial, UART, and SPI-Slave.

It should have a flexible power requirements allowing multiple powering configurations. This should include USB. 
## Modules
### Power
### RP2040
### Interfaces
### Reset
The Raspberry Pi Pico Board does not have a reset button, which can be very useful. This is relatively easy to add, and a simple configuration can be seen below: 


## Usage
### Bringup
### Flashing
## Testing
* **Signal Integrity**
Capture Signal Integrity characteristics across different frequencies, all DUT facing interfaces
    * Setup Time minimum (tSU)
    * Hold Time minimum (tH)
    * Jitter 
    * Rise Time (ns)
    * Fall Time (ns)
    * Amplitude 
    * Signal Distortion %
    
* **Protocol Tests**   

    * Invalid Interface
    * Invalid Size
    * Buffer Overflow
    * Invalid Checksum
    * Missing Payload
   
    * Force PIO Timeout
    * PIO TX FIFO Full
## Debugging
## Appendix
* [Raspberry Pi Pico][1]
* [Probe-RS `probe-rs-debugger`][2]
* [Raspberry Pi Pico `elf2uf2`][3]
* [Raspberry Pi Pico `picotool`][4]
* [Example Pico Dev Board][5]


<!-- Reference -->
[1]: https://www.raspberrypi.com/documentation/microcontrollers/rp2040.html
[2]: https://github.com/probe-rs/vscode
[3]: https://github.com/JoNil/elf2uf2-rs
[4]: https://github.com/raspberrypi/picotool
[5]: https://oshwlab.com/tim.savage/pipico-devboard
