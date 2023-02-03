<!-- Title -->
<p align="center">
  <img width=15% src="https://www.svgrepo.com/show/68860/microchip.svg">
  <h1 align="center">pico-bridge</h1>
</p>

The **pico-bridge** is a project that implements an embedded RPC for interface bridging utilizing the programmable I/O peripherals on the RP2040. 
The application for the [`rp2040`][1] is based on the [`RTIC`][2] Real Time Interrupt-Driven Concurrency environment for [Rust][3].


### What is Interface Bridging?
Interface bridging in this context is abstracting away the use of each DUT (Device) interface or protocol into the interaction between commands/data and TX/RX FIFOs. The enabling technology for the RP2040 to perform this service are the Programmable I/O State machines that allow high speed, extensible, and customizable "interfaces" to be interacted with as if they were simply hardware drivers. For example, an SMI (Serial Management Interface) state machine has been provided that by writing the Phy Address and Register Address + (data) to its TX FIFO, the system can write and read to the register space of an Ethernet Phy with precise timing. This state machine clocks out an MDC pin as well as reading and writing to the MDIO in the format of the [SMI Clause-22 Specification][29]. 

### Programmable I/O Architecture
There are 2 identical PIO blocks in RP2040. Each PIO block has dedicated connections to the bus fabric, GPIO and
interrupt controller. Each PIO block contains 4 independent state machines that can all be running simultaneously with different programs loaded. A total of 8 state machines can be running at any given time on the RP2040 MCU.

<img width="459" alt="image" src="https://user-images.githubusercontent.com/68623356/216333483-515f476e-c5cc-4484-92e4-1c4c3eac3a3b.png">

#### **Benefits**
When we want to send data to a pin via a state machine, we first push the data to the FIFO input. When the state machine is ready to process the data, it will pull it from the queue and perform the instruction.

The key benefit here is that we can decouple the need for the central CPU from the execution of the instruction, since it has been “delegated” to the PIO’s state machine.

Although each FIFO can only hold up to four words of data (each of 32 bits), we can link them with direct memory access (DMA) to transmit larger amounts. This way, we can once again free up the CPU from having to “babysit” the process.

#### **Instruction Set**
* **JMP**: This ‘jump’ instruction can be a conditional or a non-conditional statement. In this, it transfers the flow of execution by changing the instruction pointer register. In simple words, with ‘jmp’ statement the flow of execution goes to another part of the code.
* **WAIT**: This instruction stalls the execution of the code. Each instruction takes one cycle unless it is stalled (using the WAIT instructions).
* **OUT**: This instruction shifts data from the output shift register to other destinations, 1…32 bits at a time.
* **PULL**: This instruction pops 32-bit words from TX 
 FIFO into the output shift register.
* **IN**: This instruction shift 1…32 bits at a time into the register.
* **PUSH**: This instruction to write the ISR content to the RX FIFO.

#### **Interaction**
Each state machine driver will be designed to block until a word of data has been written to its TX FIFO.
```
.wrap_target
pull block
.wrap
```
The above program will sit in a loop and wait until a word is ready, and then pull it onto the OSR (Output Shift Register). 

A driver will generally take this data that is expected to be in a certain format and clock it out to meet the specification of its interface. For example, the SMI_Master driver provided expects the first two bits in the word provided to be the Op-Code and the next 16 to include both the PHY Address and Register Address. 

When the driver is finished with its transaction, it will set an IRQ flag to indicate a word has been pushed to its respective RX FIFO. On an operation like a read, this could be the register contents, or for a write it could be simply a status bit that indicates the write successfully completed.

### Configurable
* Over the same transport layer between the host and Pico, commands can dynamically set, and read the State Machine configurations such as Clock Rate, Pin Assignments, and disable/enable

### Extensible
* Interface State Machines can be dynamically loaded and unloaded, depending on the application requirements. 
* Users can implement additional interfaces as needed, ie: [An awesome RMII PIO implementation][26].
* Multiple transport layers are all supported depending on host, (USB Serial, UART/SPI, Standalone SPI)
* Greatly simplifies device peripheral designs, as all interfaces can be managed through a single port. Removes the need for separate board headers for each interface. 

### Robust, Performant, and Low-Power
* A purely Rust application, static analysis at compile time guarantees memory safety and thread safe code.
* Built on the RTIC (Real Time Interrupt Driven Concurrency) Framework, tasks are bound to hardware interrupts managed by the ARM Cortex NVIC, with no RTOS kernel overhead. This makes the already rapid Rust application deterministic and linear in its transaction turnaround. See: [`Benchmarks and Profiling`][23]
* This interrupt-driven application enters a low power sleep state when idle. Will reach 6 mW when in its sleep mode.

### Remote Procedure Call
* RPC System Architecture
<img width="716" alt="image" src="https://user-images.githubusercontent.com/68623356/215372195-838c0ac2-9e39-4127-b480-fc2aa33086c0.png">

### DUT Interfaces
* SPI Master: 4 Modes, Multiple CS, up to system frequency (133 MHz)
* SMI Master: up to 30 MHz
* JTAG: TMS, TDO, TDI, and TCK synchronization and TAP state machine traversal precomputed. up to 30 MHz
* I2C: up to 133 MHz

### Host Interfaces
* Serial USB (Using RP2040 built in USB 1.1 Phy and controller stack) Up to 12Mbps. 
* Serial UART and SPI slave combination. Command specified over UART and associated data transmitted over SPI
* Multi Packet SPI Slave: protocol based SPI 


## Table of Contents
1. [Requirements](#requirements)
2. [Setup](#setup)
    1. [System Setup](#system-setup)
    2. [Hardware Setup](#hardware-setup)
3. [Usage](#usage)
4. [RPC Requests](#RPC-Requests)
5. [Host Configurations](#Host-Configurations)
    1. [Standalone SPI](#Standalone-Spi)
    2. [UART/SPI](#UART/SPI)
6. [Interface Defaults](#Interface-Defaults)
7. [Testing](#Testing)
8. [Appendix](#appendix)

## Requirements
* Raspberry Pi Pico
* Application Release .uf2 file 

## Setup
### System Setup


### Hardware Setup
#### Connecting the Raspberry Pi Pico Debug Probe

```
Pico A GND -> Pico B GND
Pico A GP2 -> Pico B SWCLK
Pico A GP3 -> Pico B SWDIO
Pico A GP4/UART1 TX -> Pico B GP1/UART0 RX
Pico A GP5/UART1 RX -> Pico B GP0/UART0 TX
Pico A VSYS -> Pico B VSYS
```

## Usage
#### Running
To run the firmware in debug mode:
```shell
$ cargo run
```

To run the firmware in release mode:
```shell
$ cargo run --release
```

#### Programming the Pico
1. While holding down the BOOTSEL button, plug the Pico into a USB port.
2. The Pico will appear as a mass storage device in your file navigator.
3. Drag-and-drop the .uf2 to the Pico, as you would if you were 
moving a file to a flash drive.
4. The Pico will automatically reboot, and begin the application. It will appear as a Serial USB device which can be accessed with a terminal emulator as well 
as UART and SPI

### Programming the RP2040

## RPC Requests
TODO add the menu and possible commands that can be called and how to use them across each host transport
## Host Configurations

A host in this architecture can be anything that is capable of making requests to and receiving from the pico-bridge with one or more of the supported host-facing interfaces (Serial, UART, or SPI).

In order to successfully call the functions the pico-bridge supports, it is necessary for the host to understand the protocol that is implemented. This host/pico-bridge agreed protocol is the architectural key that can successfully abstract an interface and platform agnostic way to interact with device side hardware. 

To implement this, a Request/Receive message protocol is created which specifies the required and optional fields a request and response must have to ensure data integrity, correctness, and consistency.

### ****Request/Receive Protocol****

To begin a request, the host-side driver or user will construct a HostRequest message that will be packaged with all the necessary fields for the given RPC command. These required fields may be different for various RPC commands, and the way they are transmitted can be different across the host facing interfaces. 

For example, using the serial interface of the pico-bridge,
The entry below will suffice for the rpc-bridge to understand that this is a SMI interface request with a read operation on the Phy Address 9 to Register 1 of the general SMI register bank:
```
smi r 0x9 0x1 
```

If using the standalone SPI host facing interface, a data structure must be created with extra fields. This data structure is then broken into bytes and pushed onto the SPI bus where the pico-bridge will read in its contents and construct a HostRequest message as seen below: 

 [src/protocol.rs]
```
    pub struct HostRequest<S: State> {
        state: PhantomData<S>,
        proc_id: u8,
        interface: ValidInterfaces,
        operation: ValidOps,
        checksum: u8,         // Wrapping checksum
        size: u8,             // A value between 0 and 4
        payload: [u8; 4],     // Max payload size over SPI 
    }
```
pico-bridge will instantiate this data structure with the type:
```
let mut new_message: HostRequest<Unclean> = HostRequest::new();
```
This has an associated type of 'Unclean' which implies that it is not yet ready to be sent to its destination. Before sending, this type enforces at compile time that the application must initialize all fields of the message before being cleaned. 
It will perform validation of the parameters and verify the checksum by using the trait function: 'init_clean()' which returns a new HostRequest with an associated type <Clean>. 

This is an example of a zero cost abstraction where using Rust's rich type system can create effective compile time checks that behavior is followed correctly. These associated types have zero size, and are essentially enforcing labels.
```
new_message.init_clean(); // HostRequest<Clean>
```
***It is still a design question what to do with a message that fails to transition to the Clean type, for whatever reason. Some potential options are to simply drop the message or, and whether or not to notify the host.***

A host driver that constructs the SPI message to be sent can look like: 
```
header = ValidInterface::SMI >> 3 | ValidOp::Read >> 5 | Proc_id # Enumerated values
data_tag = size | calculate_checksum(payload) >> 2 # Size is 0x2 bytes
request = [header, data_tag, 0x9, 0x1]

spi.transfer(request)
```

In end functionality, both the SPI and Serial messages will invoke the same functions on the pico-bridge. They will be constructed the same way when received. The type HostRequest<Clean> will implement the trait 'Send' which will define the generic behavior of interacting with the TX FIFOS or performing the RPC command:

```
impl Send for HostRequest<Clean> {
    pub fn send_out(&self) -> Result<SlaveResponse<NotReady>, SlaveErr>
    {
        ...
    }
}
```
The send_out() function will match on the interface specified and push the payload to the TX FIFO of that interface. If it is a Config command, the handler task that built the message will spawn the config function associated and pass the parameters specified. 

At this point, keep in mind it has already been validated that the fields are valid because it has been checked before transitioning. 

The return type of the send_out() function is an Option<> which can be unpacked as either a valid SlaveResponse data structure or an enumeration of a Slave Error that occured when performing the command. 

the SlaveResponse data structure is similar in functionality to the HostRequest we saw earlier:
```
    pub struct SlaveResponse<S: State> {
        state: PhantomData<S>,
        proc_id: u8,
        host_config: ValidHostInterface,
        checksum: u8,          // Wrapping checksum
        response: [u8; 4],     // Max response size
    }
```
Like before, when first initialized, it will be of the type: 
```
SlaveResponse<NotReady>
```
Initialized with the same proc_id as the preceding HostRequest.
Upon the event of the response data being ready whether from Config function succeeding or the PIO IRQ asserting a read on the RX FIFO,
the response field will be filled, allowing the type to transition to:
```
SlaveResponse<Ready>
```
Which implements the Respond Trait:
```
impl Respond for SlaveResponse<Ready> {
    pub fn respond_to_host(&self) -> Option<(), app::Error> {
        ...
    }
}
```
It is noted that the SlaveResponse data structure includes a field called host_config. This is an enumerated type of ValidHostInterfaces. This is important because the SlaveResponse also looks different when sending back to the host.

For example, with USB Serial and UART, the respond_to_host function can simply write to the respective buffer and that is a completed transaction. In Standalone SPI however, the pico-bridge is configured as a slave, and can not initiate communication with the Host (SPI Master). It is the function of the Host to re-request the SlaveResponse, which will be clocked out on the MISO line of the SPI bus over the next transactions. 

****Another Design decision is whether or not to continuously advertise slave status codes. If the Host tries to clock in a response too early, the slave can simply shift out a SLAVE_BUSY code until SlaveResponse is ready. If not, the Response is written on the next transfer, but the host will not know exactly when that Response will come. Also it should not be necessary to create a new message simply to read the response, so maybe the host should send a ReadREQ code that states it is not a message but a transfer to read from MISO the Response.****

### **Optimizations**
One assumption that can be integrated into this design is that a host or application will typically make the same host commands repeatedly. For example, a Device may be being configured with a JTAG interface, so repeated writes and reads to the same interface will be done.

****Another design decision is to include a static state that stores the most recent interface used as the default interface in a LIFO fashion to remove the need to always specify the interface in every transaction. This may cause issues with creating more states, and it is only removing 3 bits.****








### UART/SPI
How to setup communication between the Pico and Host for each interface

## Command List
## Interface Defaults
Clock rates, pin assignments, etc...
## Testing

Test timing and latency over 1000 transactions with each interface. 
Total Average Latency 
Maximum Latency
Minimum Latency
Power consumption

* **Transaction Latency Test Architecture**
<img width="707" alt="image" src="https://user-images.githubusercontent.com/68623356/216362037-a6015805-e16b-463c-aaf2-1500c493aa4e.png">
  
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

Test across all host-facing interfaces
  
    * Invalid Interface
    * Invalid Size
    * Buffer Overflow
    * Invalid Checksum
    * Missing Payload
   
    * Force PIO Timeout
    * PIO TX FIFO Full
   

## Appendix
#### Documentation
* [Raspberry Pi Pico][1]
* [Rust][3]
* [Cargo][8]
* [Rustup][15]
* [Embassy][2]
* [Knurling-RS `defmt`][5]
* [Knurling-RS `flip-link`][6]
* [Knurling-RS `probe-run`][7]
* [Probe-RS `cargo-embed`][10]
* [Probe-RS `probe-rs-debugger`][11]
* [Raspberry Pi Pico `elf2uf2`][12]
* [Raspberry Pi Pico `picotool`][13]
* [CMSIS-DAP Firmware `DapperMime`][16]

#### Resources
* [SMI Protocol Specification][29]
* [Rust Embedded Book][20]
* [Awesome Embedded Rust][21]
* [Getting Started with Raspberry Pi Pico][22]
* [Ferrous Systems Embedded Training][23]
* [Ferrous Systems Embedded Teaching Material][24]
* [RP-RS App Template][25]
* [RP-RS Alternative Debug Probes][17]
* [RP-RS Alternative Runners][14]
* [Knurling-RS App Template][4]
* [Probe-RS Probe Setup][9]
* [Raspberry Pi Pico Dev Board][19]


<!-- Reference -->
[1]: https://www.raspberrypi.com/documentation/microcontrollers/rp2040.html
[2]: https://rtic.rs/1/book/en/preface.html
[3]: https://www.rust-lang.org/
[4]: https://github.com/knurling-rs/app-template
[5]: https://github.com/knurling-rs/defmt
[6]: https://github.com/knurling-rs/flip-link
[7]: https://github.com/knurling-rs/probe-run
[8]: https://doc.rust-lang.org/cargo/
[9]: https://probe.rs/docs/getting-started/probe-setup/
[10]: https://github.com/probe-rs/cargo-embed
[11]: https://github.com/probe-rs/vscode
[12]: https://github.com/JoNil/elf2uf2-rs
[13]: https://github.com/raspberrypi/picotool
[14]: https://github.com/rp-rs/rp2040-project-template#alternative-runners
[15]: https://rustup.rs/
[16]: https://github.com/majbthrd/DapperMime
[17]: https://github.com/rp-rs/rp2040-project-template/blob/main/debug_probes.md
[18]: https://datasheets.raspberrypi.com/pico/getting-started-with-pico.pdf#picoprobe-wiring-section
[19]: https://timsavage.github.io/rpi-pico-devboard/
[20]: https://docs.rust-embedded.org/book/
[21]: https://github.com/rust-embedded/awesome-embedded-rust
[22]: https://datasheets.raspberrypi.com/pico/getting-started-with-pico.pdf
[23]: https://embedded-trainings.ferrous-systems.com/
[24]: https://github.com/ferrous-systems/teaching-material
[25]: https://github.com/rp-rs/rp2040-project-template
[26]: https://github.com/sandeepmistry/pico-rmii-ethernet
[29]: https://prodigytechno.com/mdio-management-data-input-output/
