#![no_std]
#![no_main]

//! Pico-RPC-RTIC Application for MCHP Hardware Interface Bridging
//! Opens USB Device Serial Port, SPI slave, and Serial UART Host-to-Device transport layers to handle reqeusts 
//! RTIC assigns hardware tasks for these peripheral interrupts in the RTIC domain to handle asynchronous host requests
//! 
//! Author: Dmitri Lyalikov
//! Version: 0.0.1
//! 
//! TODO: Load, Configure, Start PIO state machines (SMI,JTAG)
//! TODO: PIO receive/write task
//! TODO: push init, print_menu, match_usb_serial_buf, and write_serial into a module
//! TODO: Menu task spawn
//! TODO: SPI/UART command abstraction and data types 
//! 
//! 
//! TODO: Assert IRQ on SMI reads
//! TODO: create shared string, make match_serial_buf take a string argument and match on those strings
//! TODO: Clear SPI/UART interrupts

use defmt_rtt as _;
use panic_halt as _;

mod fmt;
mod setup;
mod protocol;

/// Clock divider for the PIO SM
const PIO_CLK_DIV_INT: u16 = 1;
const PIO_CLK_DIV_FRAQ: u8 = 255;

#[rtic::app(device = rp_pico::pac, peripherals = true)]
mod app {

    use embedded_hal::blocking::spi::Transfer;
    use rp_pico::hal as hal;
    use rp_pico::pac;

    use hal::{clocks::Clock,
        spi,
        uart::{UartConfig, DataBits, StopBits},
        gpio::{pin::bank0::*, Pin, FunctionUart},
        pio::{PIOExt, ShiftDirection,PIOBuilder, Tx, SM0, PinDir,}
        };

    use usb_device::control::Request;
    // USB Device support 
    use usb_device::{class_prelude::*, prelude::*};
    // USB Communications Class Device support
    use usbd_serial::SerialPort;

    use fugit::RateExtU32;

    use crate::fmt::Wrapper;
    use crate::setup::{Counter, match_usb_serial_buf, write_serial, print_menu};
    use crate::protocol::HostRequest;
    use core::str;
    use core::str::from_utf8;


    /// Clock divider for the PIO SM
    const PIO_CLK_DIV_INT: u16 = 1;
    const PIO_CLK_DIV_FRAQ: u8 = 255;

    type UartTx = Pin<Gpio0, FunctionUart>;
    type UartRx = Pin<Gpio1, FunctionUart>;

    /// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
    /// if your board has a different frequency
    const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    #[shared]
    struct Shared {
        
        serial: SerialPort<'static, hal::usb::UsbBus>,
        usb_dev: usb_device::device::UsbDevice<'static, hal::usb::UsbBus>,

        // SMI PIO StateMachine Instance
        smi_master: hal::pio::StateMachine<(pac::PIO0, SM0), hal::pio::Running>,
        // SMI PIO TX FIFO
        smi_tx: hal::pio::Tx<(pac::PIO0, SM0)>,
        // SMI PIO RX FIFO
        smi_rx: hal::pio::Rx<(pac::PIO0, SM0)>,

        // A single Message struct that is constructed when a command is given
        // TODO: Place in queue
        message: HostRequest<crate::protocol::Clean>,

        serial_buf: [u8; 64],
        // String command that will be received over serial and must be matched
        // Used for internal logic in USB_IRQ to count characters.
        counter: Counter,
    }

    #[local]
    struct Local {
        spi_dev: hal::Spi<hal::spi::Enabled, pac::SPI0, 16>,
        uart_dev: hal::uart::UartPeripheral<hal::uart::Enabled, pac::UART0, (UartTx, UartRx)>,
    }

    #[init(local = [usb_bus: Option<usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>> = None])]
    fn init(mut c: init::Context) -> (Shared, Local, init::Monotonics) {
        //*******
        // Initialization of the system clock.
        let mut resets = c.device.RESETS;
        let mut watchdog = hal::watchdog::Watchdog::new(c.device.WATCHDOG);

        // Configure the clocks
        let clocks = hal::clocks::init_clocks_and_plls(
            XTAL_FREQ_HZ,
            c.device.XOSC,
            c.device.CLOCKS,
            c.device.PLL_SYS,
            c.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let mut spi_message = HostRequest::new().init_clean();
        
    
        // The single-cycle I/O block controls our GPIO pins
        let sio = hal::Sio::new(c.device.SIO);

        // Set the pins to their default state
        let pins = hal::gpio::Pins::new(
            c.device.IO_BANK0,
            c.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        // These are implicitly used by the spi driver if they are in the correct mode
        let _spi_sclk = pins.gpio6.into_mode::<hal::gpio::FunctionSpi>();
        let _spi_mosi = pins.gpio7.into_mode::<hal::gpio::FunctionSpi>();
        let _spi_miso = pins.gpio4.into_mode::<hal::gpio::FunctionSpi>();
        let _spi_cs = pins.gpio5.into_mode::<hal::gpio::FunctionSpi>();
        let spi = hal::Spi::<_, _, 16>::new(c.device.SPI0);
        // Exchange the uninitialized spi device for an enabled slave
        let spi_dev = spi.init_slave(&mut resets, &embedded_hal::spi::MODE_0);
          
        let uart_pins = (
            // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
            pins.gpio0.into_mode::<hal::gpio::FunctionUart>(),
            // UART RX (characters received by RP2040) on pin 2 (GPIO1)
            pins.gpio1.into_mode::<hal::gpio::FunctionUart>(),
        );

        let uart = hal::uart::UartPeripheral::new(c.device.UART0, uart_pins, &mut resets)
        .enable(
            UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

        //*****
        // Initialization of the USB and Serial and USB Device ID

        // USB
        // 
        // Set up the USB Driver
        // The bus that is used to manage the device and class below
        
        let usb_bus: &'static _ = 
            c.local
                .usb_bus
                .insert(UsbBusAllocator::new(hal::usb::UsbBus::new(
                    c.device.USBCTRL_REGS,
                    c.device.USBCTRL_DPRAM,
                    clocks.usb_clock,
                    true,
                    &mut resets,
                )));

        // Set up the USB Communication Class Device Driver
        let serial = SerialPort::new(usb_bus);

        // Create a USB device with a VID and PID
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Validation")
                .product("Serial port")
                .serial_number("TEST")
                .device_class(2) // from https://www.usb.org/defined-class-codes
                .build();

        // Reset the counter
        let counter = Counter::new();
        // let mut command = str::from_utf8(&[0]).unwrap();
        let mut command = "hello";
        

        let _mdio_pin = pins.gpio15.into_mode::<hal::gpio::FunctionPio0>();
        let program = pio_proc::pio_asm!( 
        "
        .side_set 1",
        ".wrap_target",
        "set pins, 0   side 0",
    "start:",
        "pull block side 0",
        "set pindirs, 1 side 0",
        "set x, 31 side 0",
    "preamble:",
        "set pins, 1 side 1      [4]",
        "set pins 1  side 0      [2]",
        "jmp x-- preamble side 0 [2]",
        "set pins, 0  side 1     [4]",
        "nop side 0 [2]",
        "set y, 11 side 0 [2]",
        "set pins, 1 side 1 [4]",
        "nop side 0 [1]",
    "addr:",
        "set x, 15 side 0 [3]",
        "out pins, 1   side 1 [4]",
        "jmp y-- addr side 0 [1]",
        "out null 20 side 0  [3]",  // Discard remaining 20 bits of 32 bit word (we wrote first 12 which are OP/PHY/REG fields)
        "nop side 1 [4]",
        "nop side 0 [4]",
        "nop side 1 [4]",
        "jmp !osre write_data    side 0 [2]", // If Autopull pulled in another word from our TX FIFO, we have data to write
        "set pindirs, 0 side 0 [2]",
    "read_data:",
        "in pins 1 side 1 [4]",
        "jmp x-- read_data side 0 [4]",
        "push side 0",
        "jmp start side 0",
    "write_data:",
        "nop side 0 [1]",
        "out pins, 1 side 1 [4]",
        "jmp x-- write_data side 0 [3]",
        ".wrap",
        ); 
            
        let (mut pio, sm0, _, _, _,) = c.device.PIO0.split(&mut resets);
        let installed = pio.install(&program.program).unwrap();
        let (mut sm, smi_rx, smi_tx) = PIOBuilder::from_program(installed)
            .out_pins(1, 1)
            .side_set_pin_base(2)
            .out_sticky(false)
            .clock_divisor_fixed_point(PIO_CLK_DIV_INT, PIO_CLK_DIV_FRAQ)
            .out_shift_direction(ShiftDirection::Right)
            .in_shift_direction(ShiftDirection::Left)
            .autopull(true)
            .pull_threshold(0)  // TEST Designed to autofill when OSRE completely empty, maybe 32 is valid. 
            .set_pins(1, 1)
            .in_pin_base(1)
            .build(sm0);
        sm.set_pindirs([(1, PinDir::Output)]);
        let smi_master = sm.start();

        let mut serial_buf = [0_u8; 64];
               
        // Set core to sleep
        c.core.SCB.set_sleepdeep();

        //********
        // Return the Shared variables struct, the Local variables struct and the XPTO Monitonics
        (
            Shared {
                serial,
                usb_dev,

                smi_master,   // SMI PIO State Machine 
                smi_tx,       // SMI TX FIFO
                smi_rx,       // SMI RX FIFO

                message: spi_message,

                serial_buf,

                //command,

                counter,
            },
            Local {
                spi_dev: spi_dev,
                uart_dev: uart, 
            },
            init::Monotonics(),
        )
    }

    // Task that binds to the SPI0 IRQ and handles requests. This will execute from RAM
    // This takes a mutable reference to the SPI bus writes immediately from tx_buffer while reading 
    // into the rx_buffer
    #[inline(never)]
    #[link_section = ".data.bar"] // Execute from IRAM
    #[task(binds=SPI0_IRQ, priority=3, local=[spi_dev])]
    fn spi0_irq(cx: spi0_irq::Context) {
        let mut tx_buf = [1_u16, 2, 3, 4, 5, 6];
        let mut _rx_buf = [0_u8; 6];
        let _t = cx.local.spi_dev.transfer(&mut tx_buf);
        
        
    }

    // USB interrupt handler hardware task. Runs every time host requests new data
    #[task(binds = USBCTRL_IRQ, priority = 3, shared = [serial, usb_dev, counter, serial_buf])]
    fn usb_rx(cx: usb_rx::Context) {
        let usb_dev = cx.shared.usb_dev;
        let serial = cx.shared.serial;
        let counter = cx.shared.counter;
        let serial_buf = cx.shared.serial_buf;

        (usb_dev, serial, counter, serial_buf).lock(
            |usb_dev_a, serial_a, counter_a, serial_buf| {
                // Check for new data
                if  usb_dev_a.poll(&mut [serial_a]) {
                    let mut buf = [0u8; 64];
                    match serial_a.read(&mut buf) {
                        Err(_e) => {
                            // Do nothing
                            // let _ = serial_a.write(b"Error Reading in Data");
                            // let _ = serial_a.flush();
                        }
                        Ok(0) => {
                            // Do nothing
                            let _ = serial_a.write(b"Didn't received data.\n\r");
                            let _ = serial_a.flush();
                        }
                        // TODO Add OK(_count) response
                        Ok(_count) => {
                            let index = counter_a.get() as usize;
                            match buf[0] {
                                // Check if return key was given \n, if so a command was given.
                                b'\r' => { 
                                    // let command = str::from_utf8(serial_buf).unwrap();
                                    // write_serial(serial_a, command, false);
                                    
                                    match_usb_serial_buf(serial_buf, serial_a, index); 
                                    // Reset the buffer
                                   // print_menu(serial_a);
                                    // Reset serial buffer
                                    for elem in serial_buf.iter_mut() {
                                        *elem = 0;
                                    }
                                    counter_a.reset(); 
                                }

                                _ => {
                                    let first_zero = serial_buf.iter().position(|&x| x == 0);
                                    match(first_zero) {
                                        Some(Index) => { serial_buf[Index] = buf[0]; }
                                        
                                        _ => { 
                                            for elem in serial_buf.iter_mut() {
                                                *elem = 0;
                                            }
                                        }
                                    }

                                    let command = str::from_utf8(&mut buf[0..1]).unwrap();
                                    write_serial(serial_a, command, false);
                                }
                            }
                    } } }
                } 
            )
        }

    // Hardware task associated with PIO0_IRQ_0
    // Takes control of shared state machine and rx fifo of PIO_0 SM_0 
    // Reads rx fifo into buffer and pushed to queue, spawn software task to return value
    #[task(binds = PIO0_IRQ_0, priority = 3, shared = [smi_master, smi_rx])]
    fn pio_sm_rx(cx: pio_sm_rx::Context) {
        let smi = cx.shared.smi_master;
        let rx = cx.shared.smi_rx;

        (smi, rx).lock(
            |smi_a, rx_a| {
                rx_a.read();
            }
        )
    }
    // Task with least priority that only runs when nothing else is running.
    #[idle(local = [])]
    fn idle(_cx: idle::Context) -> ! {
        // Locals in idle have lifetime 'static
        loop {
            // Now Wait For Interrupt is used instead of a busy-wait loop
            // to allow MCU to sleep between interrupts
            // https://developer.arm.com/documentation/ddi0406/c/Application-Level-Architecture/Instruction-Details/Alphabetical-list-of-instructions/WFI
            rtic::export::wfi()
        }
    }
}
