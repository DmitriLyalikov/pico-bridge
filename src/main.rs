#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_halt as _;



#[rtic::app(device = rp2040_hal::pac, peripherals = true)]
mod app {

    use embedded_hal::blocking::spi::Transfer;

    use rp2040_hal as hal;
    use hal::clocks::Clock;
    use hal::pac as pac;
    use fugit::RateExtU32;

    /// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
    /// if your board has a different frequency
    const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        spi_dev: rp2040_hal::Spi<hal::spi::Enabled, pac::SPI0, 16>,
    }

    #[init(local = [])]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
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

        //********
        // Return the Shared variables struct, the Local variables struct and the XPTO Monitonics
        (
            Shared {},
            Local {
                spi_dev: spi_dev,
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

    // Task with least priority that only runs when nothing else is running.
    #[idle(local = [x: u32 = 0])]
    fn idle(_cx: idle::Context) -> ! {
        // Locals in idle have lifetime 'static
        // let _x: &'static mut u32 = cx.local.x;

        //hprintln!("idle").unwrap();

        loop {
            cortex_m::asm::nop();
        }
    }
}
