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
//! TODO: push init, print_menu, match_usb_serial_buf, and write_serial into a module
//! TODO: Menu task spawn
//! TODO: SPI/UART command abstraction and data types 
//!

use defmt_rtt as _;
use panic_halt as _;

mod fmt;

#[rtic::app(device = rp2040_hal::pac, peripherals = true)]
mod app {

    use embedded_hal::blocking::spi::Transfer;

    use rp2040_hal as hal;
    use hal::clocks::Clock;
    use hal::uart::{UartConfig, DataBits, StopBits};
    use hal::gpio::{pin::bank0::*, Pin, FunctionUart};
    use hal::pac as pac;
    use fugit::RateExtU32;

    // USB Device support 
    use usb_device::{class_prelude::*, prelude::*};
    // USB Communications Class Device support
    use usbd_serial::SerialPort;


    type UartTx = Pin<Gpio0, FunctionUart>;
    type UartRx = Pin<Gpio1, FunctionUart>;

    /// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
    /// if your board has a different frequency
    const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    // Blink time 5 seconds
    const SCAN_TIME_US: u32 = 5000000; //  200000; // 5000000;  // 1000000; // 200000;

    pub struct Counter {
        counter: u32,
        enable: bool,
    }

    impl Counter {
        fn new() -> Self {
            Counter {
                counter: 0_u32,
                enable: true,
            }
        }

        fn get(&self) -> u32 {
            self.counter
        }

        fn reset(&mut self) {
            self.counter = 0_u32;
        }

        fn increment(&mut self) {
            self.counter += 1;
        }

        fn enable(&mut self, state: bool) {
            self.enable = state;
        }
    }


    #[shared]
    struct Shared {
        
        serial: SerialPort<'static, hal::usb::UsbBus>,
        usb_dev: usb_device::device::UsbDevice<'static, hal::usb::UsbBus>,

        counter: Counter,
    }

    #[local]
    struct Local {
        spi_dev: rp2040_hal::Spi<hal::spi::Enabled, pac::SPI0, 16>,
        uart_dev: rp2040_hal::uart::UartPeripheral<hal::uart::Enabled, pac::UART0, (UartTx, UartRx)>,
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
        let _spi_cs = pins.gpio8.into_mode::<hal::gpio::FunctionSpi>();
        let spi = hal::Spi::<_, _, 16>::new(c.device.SPI0);
        
        // Exchange the uninitialised SPI driver for an initialised one
        let spi_dev = spi.init(
            &mut resets,
            clocks.peripheral_clock.freq(),
            16.MHz(),
            &embedded_hal::spi::MODE_0,
            true,
        );

        let uart_pins = (
            // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
            pins.gpio0.into_mode::<hal::gpio::FunctionUart>(),
            // UART RX (characters received by RP2040) on pin 2 (GPIO1)
            pins.gpio1.into_mode::<hal::gpio::FunctionUart>(),
        );

        let mut uart = hal::uart::UartPeripheral::new(c.device.UART0, uart_pins, &mut resets)
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
    

        // Set core to sleep
        c.core.SCB.set_sleepdeep();
        //********
        // Return the Shared variables struct, the Local variables struct and the XPTO Monitonics
        (
            Shared {
                serial,
                usb_dev,

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
    #[link_section = ".data.bar"]
    #[task(binds=SPI0_IRQ, priority=3, local=[spi_dev])]
    fn spi0_irq(cx: spi0_irq::Context) {
        let mut tx_buf = [1_u16, 2, 3, 4, 5, 6];
        let mut _rx_buf = [0_u16; 6];
        let _t = cx.local.spi_dev.transfer(&mut tx_buf);
    }

    // USB interrupt handler hardware task. Runs every time host requests new data
    #[task(binds = USBCTRL_IRQ, priority = 3, shared = [serial, usb_dev, counter])]
    fn usb_rx(cx: usb_rx::Context) {
        let usb_dev = cx.shared.usb_dev;
        let serial = cx.shared.serial;
        let counter = cx.shared.counter;

        (usb_dev, serial, counter).lock(
            |usb_dev_a, serial_a, counter_a| {
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
                            let _ = serial_a.write(b"Didn't received data.");
                            let _ = serial_a.flush();
                        }
                        // TODO Add OK(_count) response
                        Ok(_count) => {
                            match_usb_serial_buf(&buf, serial_a, counter_a);   
                        // }
                    }
                    }
                }
                }
            )
        }

    // Task with least priority that only runs when nothing else is running.
    #[idle(local = [x: u32 = 0])]
    fn idle(cx: idle::Context) -> ! {
        // Locals in idle have lifetime 'static
        let x: &'static mut u32 = cx.local.x;

        //hprintln!("idle").unwrap();

        loop {
            // Now Wait For Interrupt is used instead of a busy-wait loop
            // to allow MCU to sleep between interrupts
            // https://developer.arm.com/documentation/ddi0406/c/Application-Level-Architecture/Instruction-Details/Alphabetical-list-of-instructions/WFI
            rtic::export::wfi()
        }
    }

    /* New Tasks */

    // Helper function to ensure all data is written across the serial interface
    fn write_serial(serial: &mut SerialPort<'static, hal::usb::UsbBus>, buf: &str, block: bool) {
        let write_ptr = buf.as_bytes();

        // Because the buffer is of constant size and initialized to zero (0) we 
        //  add a test to determine the size that's really occupied by the str that we
        // want to send. From index zero to first byte that is as the zero byte value
        let mut index = 0;
        while index < write_ptr.len() && write_ptr[index] != 0 {
            index += 1;
        }
        let mut write_ptr = &write_ptr[0..index];

        while !write_ptr.is_empty() {
            match serial.write(write_ptr) {
                Ok(len) => write_ptr = &write_ptr[len..],
                // Meaning the USB write buffer is full
                Err(UsbError::WouldBlock) => {
                    if !block {
                        break;
                    }
                }
                // On error, just drop unwritten data
                Err(_) => break,
            }
        }
        let _ = serial.flush();
    }

    // Match the Serial Input commands to a hardware/software request
    fn match_usb_serial_buf(
        buf: &[u8; 64],
        // Add any accessed 'static peripherals (PIO, SPI, etc) that will be controlled by host
        serial: &mut SerialPort<'static, hal::usb::UsbBus>,
        counter: &mut Counter,
    ) {
        let _buf_len = buf.len();
        match buf[0] {
            // Print Menu
            b'M' | b'm' => {
                write_serial(serial, "M - Print Menu\n", false);
                print_menu(serial);
            }
            // 0 - Reset counter
            b'0' => {
            write_serial(serial, "M - Print Menu\n", false);
                counter.reset();
            }
            // 1 - Increment counter
            b'1' => {
                write_serial(serial, "1 - Increment counter\n", false);
                counter.increment();
            }
            // 2 - Start continues counter
            b'2' => {
                write_serial(serial, "2 - Start continues counter\n", false);
                counter.enable(true);
            }
            // 3 - Stop continues counter
            b'3' => {
                write_serial(serial, "3 - Stop continues counter\n", false);
                counter.enable(false);
            }
            _ => {
                write_serial(
                    serial,
                    unsafe { core::str::from_utf8_unchecked(buf) },
                    false,
                );
                write_serial(serial, "Invalid option!\n", false);
            }
        }
    }

    fn print_menu(serial: &mut SerialPort<'static, hal::usb::UsbBus>){
        let mut _buf = [0u8; 273];
        // Create the Menu.
        let menu_str = "*****************
*  Menu:
*
*  M / m - Print menu
*    0   - Reset counter
*    1   - Increment counter
*    2   - Start continues counter
*    3   - Stop continues counter
*****************
Enter option: ";

        write_serial(serial, menu_str, true);
    }
}
