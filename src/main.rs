#![no_std]
#![no_main]

//! Pico-Bridge Application for Hardware Interface Bridging
//! Opens USB Device Serial Port, SPI slave, and Serial UART Host-to-Device transport layers to handle reqeusts 
//! RTIC assigns hardware tasks for these peripheral interrupts in the RTIC domain to handle asynchronous host requests
//! 
//! Author: Dmitri Lyalikov
//! Version: 0.0.1
//! 
//! TODO: PIO receive/write task
//! TODO: Assert IRQ on SMI reads
//! TODO: Clear SPI/UART interrupts
//! 
//! src/openocd -f interface/cmsis-dap.cfg -c "adapter speed 1000" -f target/rp2040.cfg -s tcl
//! gdb-multiarch -q -ex "target extended-remote :3333" target/thumbv6m-none-eabi/debug/pico-rpc-rtic


use defmt_rtt as _;
use panic_halt as _;
mod fmt;
mod serial;
mod protocol;


#[rtic::app(device = rp_pico::pac, peripherals = true, dispatchers= [PWM_IRQ_WRAP, SIO_IRQ_PROC0, SIO_IRQ_PROC1, UART1_IRQ])]
mod app {
    use embedded_hal::digital::v2::OutputPin;
    use embedded_hal::blocking::spi::{Transfer, Write};
    use embedded_hal::spi::FullDuplex;

    use fugit::HertzU32;
    use rp_pico::XOSC_CRYSTAL_FREQ;
    use rp_pico::pac::Interrupt;
    use rp_pico::hal as hal;
    use rp_pico::pac;
    use heapless::{String, spsc::{Consumer, Producer, Queue}};

    const UART0_ICR: *mut u32 = 0x4003_4044 as *mut u32;
    const SPI0_ICR: *mut u32 = 0x4003_c020 as *mut u32;
    const PIO0_IRQE: *mut u32 = 0x5020012c as *mut u32;
    const PIO0_IRQC: *mut u32 = 0x50200030 as *mut u32;
    
    //use embedded_hal::
    use hal::{clocks::Clock,
        uart::{UartConfig, DataBits, StopBits},
        gpio::{pin::bank0::*, Pin, FunctionUart},
        pio::{PIOExt, ShiftDirection,PIOBuilder, SM0, PinDir,},
        };

    use cortex_m::peripheral::NVIC;

    // USB Device support 
    use usb_device::{class_prelude::*, prelude::*};

    // USB Communications Class Device support
    use usbd_serial::SerialPort;
    use fugit::RateExtU32;

    use crate::fmt::Wrapper;
    use crate::serial::{match_usb_serial_buf, write_serial};
    use crate::protocol::{Send, ValidHostInterfaces,
        host::{HostRequest, Clean, ValidOps, ValidInterfaces,}, 
        slave::{NotReady, SlaveResponse}};

    use core::str;

    /// Clock divider for the PIO SM
    const SMI_DEFAULT_CLKDIV: u16 =  4;//4; // (133000000 / 2500000)
    const PIO_CLK_DIV_FRAQ: u8 =  145;//145;

    type UartTx = Pin<Gpio0, FunctionUart>;
    type UartRx = Pin<Gpio1, FunctionUart>;

    /// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
    /// if your board has a different frequency

    #[shared]
    struct Shared {
        
        serial: SerialPort<'static, hal::usb::UsbBus>,
        usb_dev: usb_device::device::UsbDevice<'static, hal::usb::UsbBus>,

        pio0: hal::pio::PIO<pac::PIO0>,
        // SMI PIO StateMachine Instance
        smi_master: hal::pio::StateMachine<(pac::PIO0, SM0), hal::pio::Running>,
        // SMI PIO TX FIFO
        smi_tx: hal::pio::Tx<(pac::PIO0, SM0)>,
        // SMI PIO RX FIFO
        smi_rx: hal::pio::Rx<(pac::PIO0, SM0)>,

        //spi_master: hal::Spi<hal::spi::Enabled, pac::SPI1, 8>,

        // String command that will be received over serial and must be matched
        serial_buf: [u8; 64],

        host_producer: Producer<'static, HostRequest<Clean>, 3>,

        #[lock_free]
        _spi_tx_buf: [u16; 9],

        // pin for interrupt testing, additional functions, etc..
        freepin: Pin<Gpio25, hal::gpio::Output<hal::gpio::PushPull>>,
        spi_dev: hal::Spi<hal::spi::Enabled, pac::SPI0, 8>,
    }

    #[local]
    struct Local {
        uart_dev: hal::uart::UartPeripheral<hal::uart::Enabled, pac::UART0, (UartTx, UartRx)>,

        spi_tx_producer: Producer<'static, [u8; 18], 3>,
        spi_tx_consumer: Consumer<'static, [u8; 18], 3>,

        host_consumer: Consumer<'static, HostRequest<Clean>, 3>,

        producer: Producer<'static, SlaveResponse<NotReady>, 3>,    // Statically allocated non-blocking, non critical section access to writng to queue
        consumer: Consumer<'static, SlaveResponse<NotReady>, 3>,    // Statically allocated non-blocking, non critical section access to read to queue 
    }

    #[init(local = [usb_bus: Option<usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>> = None,
        spi_q: Queue<[u8; 18], 3> = Queue::new(),
        q: Queue<SlaveResponse<NotReady>, 3> = Queue::new(),
        host_q: Queue<HostRequest<Clean>, 3> = Queue::new()])]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        unsafe {
            hal::sio::spinlock_reset();
        }
        let mut core = c.core;
        let mut p = c.device;
        // let mut resets = c.device.RESETS;
        //*******
        // Initialization of the system clock.
        let mut watchdog = hal::watchdog::Watchdog::new(p.WATCHDOG);

        // Try to set VREG to DVDD to 1.25V
        p.VREG_AND_CHIP_RESET.vreg.write(|w| unsafe {
            w.vsel().bits(14)
        });

        /** 
        // Step 1. Turn on the crystal.
	    let xosc = hal::xosc::setup_xosc_blocking(p.XOSC, rp_pico::XOSC_CRYSTAL_FREQ.Hz())
            .map_err(|_x| false)
            .unwrap();

        // Step 2. Configure the watchdog tick generation to tick over every microsecond
        watchdog.enable_tick_generation((XOSC_CRYSTAL_FREQ / 1_000_000) as u8);

        // Step 3. Create a clocks manager
        let mut clocks = hal::clocks::ClocksManager::new(p.CLOCKS);

        // Step 4. Set up the system PLL 
        // 
        // Take the Crystal Oscillator  (=12Mhz) with no divider, and x126 to 
        // give a FOUTVCO of 1512 MHz. This must be in range 750 Mhz - 1600 Mhz
        // THe factor of 126 is calcuated automatically given the desired FOUTVCO
        //
        // Next we ÷5 on the first post divider to give 302.4 MHz
        //
        // Finally we ÷2 on the second post divider to give 151.2 Mhz
        //
        let pll_sys = hal::pll::setup_pll_blocking(p.PLL_SYS,  xosc.operating_frequency(), hal::pll::PLLConfig {
                vco_freq: HertzU32::MHz(800),
                refdiv: 1,
                post_div1: 2,
                post_div2: 2,
            }, &mut clocks,
            &mut p.RESETS,
        )
        .map_err(|_x| false)
        .unwrap();
        
        // Step 5. Set up a 48 Mhz PLL for the USB system
        	// Step 5. Set up a 48 MHz PLL for the USB system.
	let pll_usb = hal::pll::setup_pll_blocking(
		p.PLL_USB,
		xosc.operating_frequency(),
		hal::pll::common_configs::PLL_USB_48MHZ,
		&mut clocks,
		&mut p.RESETS,
	)
	.map_err(|_x| false)
	.unwrap();

    // Step 6. Set the system to run from the PLLs we just configured
    clocks  
        .init_default(&xosc, &pll_sys, &pll_usb)
        .map_err(|_x| false)
        .unwrap();
        */ 
        let mut resets = p.RESETS;
        
        // The single-cycle I/O block controls our GPIO pins
        let sio = hal::Sio::new(p.SIO);
        // Set the pins to their default state
        let pins = hal::gpio::Pins::new(
            p.IO_BANK0,
            p.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );
        
        let clocks = hal::clocks::init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            p.XOSC,
            p.CLOCKS,
            p.PLL_SYS,
            p.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let mut freepin = pins.gpio25.into_push_pull_output();
        // SPI Pre-Init Reset State
        // DEBUG Breakpoint Here: 
        // Test points:
        //      NVIC_ISER_0xc000e180 Expect: bit 17 = 1. This means SPI0_IRQ is unmasked by "default"
        //      NVIC_ICPR_0xc000e280 Expect: bit 17 = 0. This meas SPI0_IRQ is not pending
        //      SPI0_SCR0_0x4003c000 Expect: 0x00, SPI is not enabled yet
        //      SPI0_SPSR_0x4003c00c Expect: 0x3, SPI TX FIFO is empty
        //      SPI0_SMIS_0x4003c014 Expect: 0x0, all Interrupt sources are masked
        // 
        // These are implicitly used by the spi driver if they are in the correct mode
        let spi_sclk = pins.gpio18.into_mode::<hal::gpio::FunctionSpi>();
        let spi_mosi = pins.gpio19.into_mode::<hal::gpio::FunctionSpi>();
        let spi_miso = pins.gpio16.into_mode::<hal::gpio::FunctionSpi>();
        let spi_cs = pins.gpio17.into_mode::<hal::gpio::FunctionSpi>();

        let spi_master_sclk = pins.gpio26.into_mode::<hal::gpio::FunctionSpi>();
        let spi_master_mosi = pins.gpio27.into_mode::<hal::gpio::FunctionSpi>();
        let spi_master_miso = pins.gpio28.into_mode::<hal::gpio::FunctionSpi>();
        let spi_master_cs = pins.gpio13.into_mode::<hal::gpio::FunctionSpi>();

        //let mut spi_master = hal::Spi::<_, _, 8>::new(p.SPI1);

        // Initialize spi master at 1Mhz SPI Mode 0
        // let mut spi_master = spi_master.init(&mut resets, )

        let mut spi_dev = hal::Spi::<_, _, 8>::new(p.SPI0);
        // Exchange the uninitialized spi device for an enabled slave
        // let mut spi_dev = spi_dev.init(resets, peri_frequency, baudrate, mode);
        let mut spi_dev = spi_dev.init_slave(&mut resets, &embedded_hal::spi::MODE_0);
        
        // SPI Enabled State
        // DEBUG Breakpoint Here: 
        // Test points:
        //      NVIC_ISER_0xc000e180 Expect: bit 17 = 1. This means SPI0_IRQ is unmasked by "default"
        //      NVIC_ICPR_0xc000e280 Expect: bit 17 = 0. This meas SPI0_IRQ is not pending
        //      SPI0_SCR0_0x4003c000 Expect: 0xc7, SPI is enabled, MODE_3
        //      SPI0_SCR1_0x4003c004 Expect: 0x6, slave mode, SOD = 0, enabled = 1, LBM = 0
        //      SPI0_SPSR_0x4003c00c Expect: 0x3, SPI TX FIFO is empty
        //      SPI0_SMIS_0x4003c014 Expect: 0xc, RX/TX IM are unmasked
    
        // Prime the tx FIFO with a SPI0_write 
        // if spi_dev.write(&[1,2,3,4,5]).is_ok() {
            // SPI write was succesful
        // };
        // Does Spi transfer/read/write block until its contents are used or when the write completes?
        // TEST: match on spi_dev..transfer and write and step into each arm
        // Insert Breakpoint
        // TX FIFO Prime state
        //      SPI0_SPSR_0x4003c00c Expect: 0x0, SPI TX FIFO is not empty

        let uart_pins = (
            // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
            pins.gpio0.into_mode::<hal::gpio::FunctionUart>(),
            // UART RX (characters received by RP2040) on pin 2 (GPIO1)
            pins.gpio1.into_mode::<hal::gpio::FunctionUart>(),
        );

        
        let mut uart_dev = hal::uart::UartPeripheral::new(p.UART0, uart_pins, &mut resets)
        .enable(
            UartConfig::new(115200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();
        uart_dev.enable_rx_interrupt();
        uart_dev.set_rx_watermark(hal::uart::FifoWatermark::Bytes28);
        // uart_dev.write_full_blocking(b"UART Alive\r\n");

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
                    p.USBCTRL_REGS,
                    p.USBCTRL_DPRAM,
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
         //*****
        // Initialization of the PIO0 and SMI state machine
        let _mdio_pin = pins.gpio6.into_mode::<hal::gpio::FunctionPio0>();
        let _mdc_pin = pins.gpio7.into_mode::<hal::gpio::FunctionPio0>();
        let program = pio_proc::pio_asm!( 
        "
        .side_set 1",
        ".wrap_target",
        "set pins, 0   side 0",
    "start:",
        "pull block side 0",
        "set x, 31 side 0",
        "set pindirs, 1 side 0",
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
        "set pins, 0 side 0  [3]",  
        "out y 1 side 1 [4]",
        "nop side 0 [4]",
        "nop side 1 [4]",
        "jmp y-- write_data    side 0 [2]", // If Autopull pulled in another word from our TX FIFO, we have data to write
        "set pindirs, 0 side 0 [2]",
    "read_data:",
        "in pins 1 side 1 [4]",
        "jmp x-- read_data side 0 [4]",
        "push side 0",        // Set IRQ flag with index 1 (State machine 1)
        "out null 19 side 0"   // // Discard remaining 19 bits of 32 bit word (we wrote first 12 which are OP/PHY/REG fields)
        "jmp start side 0",
    "write_data:",
        "nop side 0 [1]",
        "out pins, 1 side 1 [4]",
        "jmp x-- write_data side 0 [3]",
        "set pins 0 side 0",        // Set IRQ flag with index 1 (State machine 1)
        "out null 32 side 0",
        // "irq 1 side 0",     
        ".wrap",
        ); 

        
        let (mut pio0, sm0, _, _, _,) = p.PIO0.split(&mut resets);
        let installed = pio0.install(&program.program).unwrap();
        let (mut sm, smi_rx, smi_tx) = PIOBuilder::from_program(installed)
            .out_pins(6, 1)
            .side_set_pin_base(7)
            .out_sticky(false)
            .clock_divisor_fixed_point(SMI_DEFAULT_CLKDIV, PIO_CLK_DIV_FRAQ) // freq = 1 / (int + (frac/256))
            .out_shift_direction(ShiftDirection::Right)
            .in_shift_direction(ShiftDirection::Left)
            .autopush(true)
            .autopull(false)
            // .pull_threshold()  // TEST Designed to autofill when OSRE completely empty, maybe 32 is valid. 
            .set_pins(6, 1)
            .in_pin_base(6)
            .build(sm0);
        sm.set_pindirs([(6, PinDir::Output)]);
        sm.set_pindirs([(7, PinDir::Output)]);
        let smi_master = sm.start();
        let serial_buf = [0_u8; 64];
        let _spi_tx_buf = [0_u16; 9];

        let (mut spi_tx_producer, spi_tx_consumer) = c.local.spi_q.split();
        // initialize our first buffer
        spi_tx_producer.enqueue([0_u8; 18]).unwrap();

        freepin.set_low().unwrap();
        // q has 'static lifetime so after the split and return of 'init'
        // it will continue to exist and be allocated
        let (producer, consumer) = c.local.q.split();
        let (host_producer, host_consumer) = c.local.host_q.split();

        //spi_dev.write(&[1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 7_u8, 8_u8]).unwrap();
        // IRQ0_INTE enable 0x5020012c
        unsafe {
            core::ptr::write_volatile(PIO0_IRQE, 0x1);
        }
        
        // spi_dev.send(3_u8);
        // Set core to sleep
        core.SCB.set_sleepdeep();

        //********
        // Return the Shared variables struct, the Local variables struct and the XPTO Monitonics
        (
            Shared {
                serial,
                usb_dev,

                pio0,
                smi_master,      // SMI PIO State Machine 
                // smi_master_freq, // SMI State Machine frequency
                smi_tx,          // SMI TX FIFO
                smi_rx,          // SMI RX FIFO

                serial_buf,
                _spi_tx_buf,

                host_producer,
                freepin,
                spi_dev: spi_dev,
            },
            Local {
                uart_dev: uart_dev,

                spi_tx_producer,
                spi_tx_consumer,

                host_consumer,

                producer,
                consumer,
            },
            init::Monotonics(),
        )
    }

    #[task(binds=UART0_IRQ, priority=3, local=[uart_dev], shared=[serial, host_producer])]
    fn uart0(cx: uart0::Context) {
        let uart = cx.local.uart_dev;
        let host_producer = cx.shared.host_producer;
        // RX FIFO is 32 bytes deep
        let mut buffer = [0_u8; 64];
        let serial = cx.shared.serial;
        /*  
        (serial, host_producer).lock(|serial, host_producer| {
        match uart.read_raw(&mut buffer) {
            Err(_err) => {   
                    write_serial(serial, "Uart RX Error", false);  
            }
            _ => {
                match match_usb_serial_buf(&buffer, serial) {
                    Ok(hr) => { // Got a Host Request from the Serial Port
                        let clean = hr.init_clean(); // Validate it
                        match clean {
                            Ok(hr) => {
                                match host_producer.enqueue(hr) {
                                    Ok(..) => {

                                    }
                                    Err(..) => {
                                        write_serial(serial, "Error Pushing Host Request to queue\n\r", false);
                                    }
                                };
                                send_out::spawn().unwrap(); // Send our clean host request to its destination
                            }
                            Err(err) =>  {
                                write_serial(serial, err, false);
                            }
                        } 
                    }
                    Err("Ok") => { }// We processed a simple command without constructing a Host Request
                    Err(err) => {
                        write_serial(serial, err, false); // Print the error back to the Serial port
                    }
            }
        }
        }
    });
        
    unsafe {
        // Clear the UART0_IRQ in the NVIC
        NVIC::unpend(pac::Interrupt::UART0_IRQ);
        // Clear the UART0 ICR register to clear RX/TX Interrupts for peripheral
        core::ptr::write_volatile(UART0_ICR, 0x3);
    }
    */
    }

    // Task that binds to the SPI0 IRQ and handles requests. This will execute from RAM
    // This takes a mutable reference to the SPI bus writes immediately from tx_buffer while reading 
    // into the rx_buffer
    #[inline(never)]
    #[link_section = ".data.bar"] // Execute from IRAM
    #[task(binds=SPI0_IRQ, priority=2, local=[spi_tx_consumer], shared = [spi_dev, serial, host_producer, freepin])]
    fn spi0(cx: spi0::Context) {  
        let mut serial = cx.shared.serial;
        let mut freepin = cx.shared.freepin;
        // We need to lock these resources within ISR to access them because to use hardware resources requires mutable reference
        (serial, freepin).lock(|serial, freepin|
            {
                freepin.set_high();
                write_serial(serial, "RX IRQ\n\r", false);
            }); 
        /* 
         let spi = cx.local.spi_dev;
         let mut buffer = [0_u8; 16];
         match spi.transfer(&mut buffer) {
            Err(_err) => {

            }
            _ => {
                match HostRequest::new().build_from_8bit_spi(&buffer) {
                    Ok(hr) => { // Host Request is clean and ready to be sent out
                        let mut host_producer = cx.shared.host_producer;
                        host_producer.lock(|host_producer| {
                            match host_producer.enqueue(hr) {
                                Ok(..) => {
                                    send_out::spawn().unwrap();
                                }
                                Err(..) => {
                                    // implement spi error handling
                                }
                            }
                        })
                    }
                    Err(_err) => { // Print the error to serial port if Host Request is invalid
                    }
                }
            }
         } */
        // SPI0_IRQ State
        // Debug Breakpoint
        // Test points:
        //      NVIC_ISER_0xc000e180 Expect: bit 17 = 1. This means SPI0_IRQ is unmasked
        //      NVIC_ICPR_0xc000e280 Expect: bit 17 = 1. This meas SPI0_IRQ is pending
        //      SPI0_SCR0_0x4003c000 Expect: 0xc7, SPI is enabled, MODE_3
        //      SPI0_SCR1_0x4003c004 Expect: 0x6, slave mode, SOD = 0, enabled = 1, LBM = 0
        //      SPI0_SPSR_0x4003c00c Expect: 0x3, SPI TX FIFO is empty, is RX FIFO empty?
        //      SPI0_SMIS_0x4003c014 Expect: 0xc, RX/TX IM are unmasked
        //      SPI0_SRIS_0x4003c018: This will tell what interrupt source asserted SPI0_IRQ

        // let mut serial = cx.shared.serial;
        // let _spi_dev = cx.local.spi_dev;
        //serial.lock(|serial|
        // {
        //     write_serial(serial, "Assert IRQ", false);
        //     let _rx_buf = [0_u8; 1];
        // }); 
        

        //if spi_dev.ssm() {
        //    serial.lock(|serial|
        //        {
        //            write_serial(serial, "RX IRQ", false);
        //        }); 
        //}
            /*let mut tx_buf = [5u8; 18];
            // Write/Read words back to slave. Received words will replace contents in tx_buf
            if let Some(mut tx_buf) = cx.local.spi_tx_consumer.dequeue(){}
            match cx.local.spi_dev.transfer(&mut tx_buf) {
            Ok(tx_buf) => {

                      // Received words, Now build our HostRequest
                match HostRequest::new().build_from_8bit_spi(&tx_buf) {
                    Ok(hr) => { // Host Request is clean and ready to be sent out
                        send_out::spawn(hr);
                    }
                    Err(err) => { // Print the error to serial port if Host Request is invalid
                    }
                }
            }

            _ => {
                }
            } */
            
        //unsafe {
            // Clear the SPI0_IRQ in the NVIC
        //    NVIC::unpend(pac::Interrupt::SPI0_IRQ);
            // Clear the UART RX/TX interrupt on the peripheral
         //   core::ptr::write_volatile(SPI0_ICR, 0x3);
        // }
    }

    // USB interrupt handler hardware task. Runs every time host requests new data
    #[inline(never)]
    #[link_section = ".data.bar"] // Execute from IRAM
    #[task(binds = USBCTRL_IRQ, priority = 3, shared = [serial, usb_dev, serial_buf, freepin, host_producer])]
    fn usb_rx(cx: usb_rx::Context) {
        let usb_dev = cx.shared.usb_dev;
        let serial = cx.shared.serial;
        let serial_buf = cx.shared.serial_buf;
        let freepin = cx.shared.freepin;
        let host_producer = cx.shared.host_producer;

        (usb_dev, serial, serial_buf, freepin, host_producer).lock(
            |usb_dev_a, serial_a, serial_buf, freepin, host_producer| {
                // Check for new data
                if  usb_dev_a.poll(&mut [serial_a]) {
                    let mut buf = [0u8; 64];
                    match serial_a.read(&mut buf) {
                        Err(_e) => {
                            // Do nothing
                        }
                        Ok(0) => {
                            // Do nothing
                            let _ = serial_a.write(b"Didn't received data.\n\r");
                            let _ = serial_a.flush();
                        }
                        // TODO Add backspace function
                        Ok(_count) => {
                            match buf[0] {
                                // Check if return key was given \n, if so a command was given.
                                b'\r' => { 
                                    //freepin.set_high().unwrap();
                                    let first_zero = serial_buf.iter().position(|&x| x == 0);
                                    match first_zero {
                                        Some(Index) => { serial_buf[Index] = b' '; }
                                        None => { // This means buffer is completely full (should not happen)
                                            for elem in serial_buf.iter_mut() {
                                                *elem = 0;
                                            }
                                        }
                                    }
                                    match match_usb_serial_buf(serial_buf, serial_a) {
                                        Ok(hr) => { // Got a Host Request from the Serial Port
                                            let clean = hr.init_clean(); // Validate it
                                            match clean {
                                                Ok(hr) => {
                                                    match host_producer.enqueue(hr) {
                                                        Ok(..) => {

                                                        }
                                                        Err(..) => {
                                                            write_serial(serial_a, "Error Pushing Host Request to queue\n\r", false);
                                                        }
                                                    };
                                                    send_out::spawn().unwrap(); // Send our clean host request to its destination
                                                }
                                                Err(err) =>  {
                                                    write_serial(serial_a, err, false);
                                                }
                                            } 
                                        }
                                        Err("Ok") => { }// We processed a simple command without constructing a Host Request
                                        Err(err) => {
                                            write_serial(serial_a, err, false); // Print the error back to the Serial port
                                        }
                                    }
                                    // Reset serial buffer
                                    for elem in serial_buf.iter_mut() {
                                        *elem = 0;
                                    }
                                }

                                _ => {
                                    // Add the byte to the front of the serial_buf buffer, building the command
                                    let first_zero = serial_buf.iter().position(|&x| x == 0);
                                    match first_zero {
                                        Some(Index) => { serial_buf[Index] = buf[0]; }
                                        None => { // This means buffer is completely full (should not happen)
                                            for elem in serial_buf.iter_mut() {
                                                *elem = 0;
                                            }
                                        }
                                    }
                                    // Print the single byte that was written so user can see type
                                    let command = str::from_utf8(&mut buf[0..1]).unwrap();
                                    write_serial(serial_a, command, false);
                                }
                            }
                    } } }
                } 
            )
        }

    // Software task that sends clean HostRequest to its destination (SysConfig or state machine)
    // Must validate that Associated State Machine is available and ready before sending, if not, return an Err
    // Pushes a SlaveResponse<NotReady> to process queue, that PIO_IRQ will build when response is gotten from state machine  
    #[task(priority = 3, local = [producer, host_consumer], shared = [serial, smi_master, smi_tx, smi_rx, freepin])]
    fn send_out(cx: send_out::Context) {

        let mut slave_response = false;

        let freepin = cx.shared.freepin;
        let smi_tx = cx.shared.smi_tx;
        let smi_rx = cx.shared.smi_rx;
        let smi_master = cx.shared.smi_master;
        let serial = cx.shared.serial; 

        let producer = cx.local.producer;

        let mut return_string = "\n\r->";
        let mut hr = cx.local.host_consumer.dequeue();
        match hr  {
            Some(mut hr) => {
                (freepin, smi_tx, smi_rx, smi_master, serial).lock(|freepin, smi_tx, smi_rx, smi_master, serial| {
                match hr.interface {
                    // For each additional supported interface, add another match arm that sends to the interface
                    // Take handle of its TX FIFO and send payload word by word according to the size
                    ValidInterfaces::SMI => {
                        // Send 32 bit word of for either read or write to SMI TX FIFO
                        smi_tx.write(hr.payload[0]);
                        smi_rx.read(); // for now we will empty the RX FIFO
                        slave_response = true;
                    }
                    ValidInterfaces::Config => {
                        if hr.operation == ValidOps::SmiSet {
                            if hr.payload[0] == 25 {
                                    smi_master.set_clock_divisor(4.56640625);
                                    // smi_master.clock_divisor_fixed_point(4, 145);
                                    return_string = "\n\rSMI Clock Rate Set 2.5Mhz\n\r->";
                            }
                            else if hr.payload[0] == 10 {
                                smi_master.clock_divisor_fixed_point(1, 145);
                                return_string = "\n\rSMI Clock Rate Set 10Mhz\n\r->";
                            }
                            else {smi_master.clock_divisor_fixed_point(hr.payload[0] as u16, 0);}
                        }
                    }
                    ValidInterfaces::GPIO => {

                            if hr.payload[0] != 0 {freepin.set_high().unwrap();}
                            else {freepin.set_low().unwrap();}
                            // We do not do slave response on set/config commands
                    }
                    ValidInterfaces::SPI => {

                    }
                    _ => {}
                }
                write_serial(serial, return_string, false);
                
                if slave_response {
                    // Exchange our Host Request for slave response that needs to be ready
                    let slave_response = hr.exchange_for_slave_response();
                    match slave_response {
                        Ok(val) => {
                            // enqueue our new slave response
                            match producer.enqueue(val) {
                                Ok(sr) => {

                                }
                                Err(err) => {
                                    write_serial(serial, "Consumer queue is full", false);
                                }
                            }
                        }
                        Err(_err) => {
                        // This should never happen
                        }
                    }
                }
                });
            }
            None => {}
            }
    }

    // Hardware task associated with PIO0_IRQ_0
    // Takes control of shared state machine and rx fifo of PIO_0 SM_0 
    // Reads rx fifo into buffer and pushed to queue, spawn software task to return value
    #[task(binds = PIO0_IRQ_0, priority = 3, shared = [serial, pio0, smi_rx], local = [consumer])]
    fn pio_sm_rx(cx: pio_sm_rx::Context) {
        // All statemachines implement IRQ flags, of which the first 0-3 LSB 

        unsafe {
            core::ptr::write_volatile(PIO0_IRQC, 0xFFF);
            NVIC::unpend(pac::Interrupt::PIO0_IRQ_0);
        }
        let mut serial = cx.shared.serial;
        (serial).lock(|serial| {
            write_serial(serial, "fired", false);
        });
        if let Some(mut slave_response) = cx.local.consumer.dequeue() {

            let pio0 = cx.shared.pio0;
            let rx = cx.shared.smi_rx;
            // let serial = cx.shared.serial;

            // Eventually lock all implemented state machines and rx fifos
            (pio0, rx, serial).lock(
                |pio0, rx_a, serial,| {
                    // First, read the index of the state machine IRQ flag 
                    // This determines which state machine flagged an IRQ
                    match rx_a.read() {
                        Some(word) => {
                                    // We got a word from the SMI RX FIFO
                            slave_response.set_payload(word);
                        }
                        _ => {
                                    // No word received
                        }
                    }
                    // Clear all PIO0 IRQ flags
                    pio0.clear_irq(0xF);
                    // Exchange our NotReady Slave Response for a Ready one
                    // TODO Add match case for this
                    match slave_response.init_ready() {
                        Ok(sr) => {

                            write_serial(serial, "Hi", false);
                            respond_to_host::spawn(sr);
                        }
                        Err(err) => {
                                    write_serial(serial, err, false);
                        }
                    }
                }
            )
        }
        else { 
            // If this happens, our IRQ fired from State machine but no slave response object
            // print that we had no slave response ready
        }
    }

    // Software task that sends clean HostRequest to its destination (SysConfig or state machine)
    // Must validate that Associated State Machine is available and ready before sending, if not, return an Err
    // Pushes a SlaveResponse<NotReady> to process queue, that PIO_IRQ will build when response is gotten from state machine
    #[task(priority = 3, shared = [serial], local =[spi_tx_producer])]
    fn respond_to_host(cx: respond_to_host::Context, sr: SlaveResponse<crate::protocol::slave::Ready>) {
        // If Host Response was SPI, we need to update the slave TX Buffer
        // This slave response will go out when the Master requests it again.
         let mut serial = cx.shared.serial;
        if sr.host_config == ValidHostInterfaces::SPI {
            let mut tx_buf = [0_u8; 18];
            // Push the relevant slave response fields to the spi tx buffer
            tx_buf[0] = sr.proc_id as u8;

            //let split_u32_to_u16 = |word: u32| -> (u16, u16) {
            //        ((word >> 16) as u16, word as u16)
            //    };
            // Split the 32-bit word from the FIFO to 16 bits, we have 16-bit SPI
            //(tx_buf[1], tx_buf[2]) = split_u32_to_u16(sr.payload);

            let split_u32_to_u8 = |word: u32| -> (u8, u8, u8, u8) {
                (((word >> 24) & 0xff) as u8, ((word >> 16) & 0xff) as u8, ((word >> 8) & 0xff) as u8, (word & 0xff) as u8)
            };

            (tx_buf[1], tx_buf[2], tx_buf[3], tx_buf[4]) = split_u32_to_u8(sr.payload);

            cx.local.spi_tx_producer.enqueue(tx_buf).unwrap();
           }

        else if sr.host_config == ValidHostInterfaces::Serial {
            (serial).lock(|serial| {
                let mut data = String::<32>::new();
                let mut buf = [0 as u8, 20];
                // Convert sr.payload to string and write to write_serial


                write_serial(serial, "got here", false);
                                // write!(Wrapper::new(&mut buf), "{}", sr.payload).expect("Can't Write");
                // write_serial(serial, out, false);
            });
        } 
    }

    // Task with least priority that only runs when nothing else is running.
    #[idle(local = [], shared = [spi_dev])]
    fn idle(_cx: idle::Context) -> ! {
        // Locals in idle have lifetime 'static
        
        //unsafe {
        //    core::ptr::write_volatile(PIO0_IRQE, 0x10);
        // }
        loop {
            //rtic::pend(Interrupt::UART0_IRQ);
            //rtic::pend(Interrupt::SPI0_IRQ);
            // Now Wait For Interrupt is used instead of a busy-wait loop
            // to allow MCU to sleep between interrupts
            // https://developer.arm.com/documentation/ddi0406/c/Application-Level-Architecture/Instruction-Details/Alphabetical-list-of-instructions/WFI
            rtic::export::wfi()
        }
    }
}

