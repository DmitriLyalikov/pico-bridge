#[doc = r" The RTIC application module"] pub mod app
{
    #[doc =
    r" Always include the device crate which contains the vector table"] use
    rp_pico :: pac as
    you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml ; use
    embedded_hal :: blocking :: spi :: Transfer ; use embedded_time ::
    fixed_point :: FixedPoint ; use rp_pico :: hal as hal ; use rp_pico :: pac
    ; use hal :: clocks :: Clock ; use hal :: spi ; use hal :: uart ::
    { UartConfig, DataBits, StopBits } ; use hal :: gpio ::
    { pin :: bank0 :: *, Pin, FunctionUart } ; use hal :: pio ::
    { PIOExt, ShiftDirection, PIOBuilder, Tx, SM0, PinDir, } ; use pac ::
    { SPI0, Interrupt } ; use rp_pico :: XOSC_CRYSTAL_FREQ ; use fugit ::
    RateExtU32 ; use crate :: setup :: Counter ; use usb_device ::
    { class_prelude :: *, prelude :: * } ; use usbd_serial :: SerialPort ;
    #[doc = r" User code from within the module"] type UartTx = Pin < Gpio0,
    FunctionUart > ; type UartRx = Pin < Gpio1, FunctionUart > ;
    #[doc =
    " External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust"]
    #[doc = " if your board has a different frequency"] const XTAL_FREQ_HZ :
    u32 = 12_000_000u32 ; fn
    write_serial(serial : & mut SerialPort < 'static, hal :: usb :: UsbBus >,
    buf : & str, block : bool)
    {
        let write_ptr = buf.as_bytes() ; let mut index = 0 ; while index <
        write_ptr.len() && write_ptr [index] != 0 { index += 1 ; } let mut
        write_ptr = & write_ptr [0 .. index] ; while! write_ptr.is_empty()
        {
            match serial.write(write_ptr)
            {
                Ok(len) => write_ptr = & write_ptr [len ..],
                Err(UsbError :: WouldBlock) => { if! block { break ; } }
                Err(_) => break,
            }
        } let _ = serial.flush() ;
    } fn
    match_usb_serial_buf(buf : & [u8 ; 64], serial : & mut SerialPort <
    'static, hal :: usb :: UsbBus >, counter : & mut Counter,)
    {
        let _buf_len = buf.len() ; match buf [0]
        {
            b'M' | b'm' =>
            {
                write_serial(serial, "M - Print Menu\n", false) ;
                print_menu(serial) ;
            } b'0' =>
            {
                write_serial(serial, "M - Print Menu\n", false) ;
                counter.reset() ;
            } b'1' =>
            {
                write_serial(serial, "1 - Increment counter\n", false) ;
                counter.increment() ;
            } b'2' =>
            {
                write_serial(serial, "2 - Start continues counter\n", false) ;
                counter.enable(true) ;
            } b'3' =>
            {
                write_serial(serial, "3 - Stop continues counter\n", false) ;
                counter.enable(false) ;
            } _ =>
            {
                write_serial(serial, unsafe
                { core :: str :: from_utf8_unchecked(buf) }, false,) ;
                write_serial(serial, "Invalid option!\n", false) ;
            }
        }
    } fn
    print_menu(serial : & mut SerialPort < 'static, hal :: usb :: UsbBus >)
    {
        let mut _buf = [0u8 ; 273] ; let menu_str =
        "*****************
*  Menu:
*
*  M / m - Print menu
*    0   - Reset counter
*    1   - Increment counter
*    2   - Start continues counter
*    3   - Stop continues counter
*****************
Enter option: "
        ; write_serial(serial, menu_str, true) ;
    } #[doc = r" User code end"] #[inline(always)] #[allow(non_snake_case)] fn
    init(mut c : init :: Context) -> (Shared, Local, init :: Monotonics)
    {
        let mut resets = c.device.RESETS ; let mut watchdog = hal :: watchdog
        :: Watchdog :: new(c.device.WATCHDOG) ; let clocks = hal :: clocks ::
        init_clocks_and_plls(XTAL_FREQ_HZ, c.device.XOSC, c.device.CLOCKS,
        c.device.PLL_SYS, c.device.PLL_USB, & mut resets, & mut
        watchdog,).ok().unwrap() ; let sio = hal :: Sio :: new(c.device.SIO) ;
        let pins = hal :: gpio :: Pins ::
        new(c.device.IO_BANK0, c.device.PADS_BANK0, sio.gpio_bank0, & mut
        resets,) ; let _spi_sclk = pins.gpio6.into_mode :: < hal :: gpio ::
        FunctionSpi > () ; let _spi_mosi = pins.gpio7.into_mode :: < hal ::
        gpio :: FunctionSpi > () ; let _spi_miso = pins.gpio4.into_mode :: <
        hal :: gpio :: FunctionSpi > () ; let _spi_cs = pins.gpio5.into_mode
        :: < hal :: gpio :: FunctionSpi > () ; let spi = hal :: Spi :: < _, _,
        16 > :: new(c.device.SPI0) ; let spi_dev =
        spi.init_slave(& mut resets, & embedded_hal :: spi :: MODE_0) ; let
        uart_pins =
        (pins.gpio0.into_mode :: < hal :: gpio :: FunctionUart > (),
        pins.gpio1.into_mode :: < hal :: gpio :: FunctionUart > (),) ; let mut
        uart = hal :: uart :: UartPeripheral ::
        new(c.device.UART0, uart_pins, & mut
        resets).enable(UartConfig ::
        new(9600.Hz(), DataBits :: Eight, None, StopBits :: One),
        clocks.peripheral_clock.freq(),).unwrap() ; let usb_bus : & 'static _
        =
        c.local.usb_bus.insert(UsbBusAllocator ::
        new(hal :: usb :: UsbBus ::
        new(c.device.USBCTRL_REGS, c.device.USBCTRL_DPRAM, clocks.usb_clock,
        true, & mut resets,))) ; let serial = SerialPort :: new(usb_bus) ; let
        usb_dev = UsbDeviceBuilder ::
        new(usb_bus,
        UsbVidPid(0x16c0,
        0x27dd)).manufacturer("Validation").product("Serial port").serial_number("TEST").device_class(2).build()
        ; let counter = Counter :: new() ; c.core.SCB.set_sleepdeep() ;
        (Shared { serial, usb_dev, counter, }, Local
        { spi_dev : spi_dev, uart_dev : uart, }, init :: Monotonics(),)
    } #[allow(non_snake_case)] fn idle(cx : idle :: Context) ->!
    {
        use rtic :: Mutex as _ ; use rtic :: mutex :: prelude :: * ; let x : &
        'static mut u32 = cx.local.x ; loop { rtic :: export :: wfi() }
    } #[allow(non_snake_case)] fn spi0_irq(cx : spi0_irq :: Context)
    {
        use rtic :: Mutex as _ ; use rtic :: mutex :: prelude :: * ; let mut
        tx_buf = [1_u16, 2, 3, 4, 5, 6] ; let mut _rx_buf = [0_u16 ; 6] ; let
        _t = cx.local.spi_dev.transfer(& mut tx_buf) ;
    } #[allow(non_snake_case)] fn usb_rx(cx : usb_rx :: Context)
    {
        use rtic :: Mutex as _ ; use rtic :: mutex :: prelude :: * ; let
        usb_dev = cx.shared.usb_dev ; let serial = cx.shared.serial ; let
        counter = cx.shared.counter ;
        (usb_dev, serial,
        counter).lock(| usb_dev_a, serial_a, counter_a |
        {
            if usb_dev_a.poll(& mut [serial_a])
            {
                let mut buf = [0u8 ; 64] ; match serial_a.read(& mut buf)
                {
                    Err(_e) => {} Ok(0) =>
                    {
                        let _ = serial_a.write(b"Didn't received data.") ; let _ =
                        serial_a.flush() ;
                    } Ok(_count) =>
                    { match_usb_serial_buf(& buf, serial_a, counter_a) ; }
                }
            }
        })
    } struct Shared
    {
        serial : SerialPort < 'static, hal :: usb :: UsbBus >, usb_dev :
        usb_device :: device :: UsbDevice < 'static, hal :: usb :: UsbBus >,
        counter : Counter,
    } struct Local
    {
        spi_dev : hal :: Spi < hal :: spi :: Enabled, pac :: SPI0, 16 >,
        uart_dev : hal :: uart :: UartPeripheral < hal :: uart :: Enabled, pac
        :: UART0, (UartTx, UartRx) >,
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `init` has access to"] pub struct
    __rtic_internal_initLocalResources < >
    {
        pub usb_bus : & 'static mut Option < usb_device :: bus ::
        UsbBusAllocator < hal :: usb :: UsbBus > >,
    } #[doc = r" Monotonics used by the system"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_Monotonics() ;
    #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_init_Context <
    'a >
    {
        #[doc = r" Core (Cortex-M) peripherals"] pub core : rtic :: export ::
        Peripherals, #[doc = r" Device peripherals"] pub device : rp_pico ::
        pac :: Peripherals, #[doc = r" Critical section token for init"] pub
        cs : rtic :: export :: CriticalSection < 'a >,
        #[doc = r" Local Resources this task has access to"] pub local : init
        :: LocalResources < >,
    } impl < 'a > __rtic_internal_init_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(core : rtic :: export :: Peripherals,) -> Self
        {
            __rtic_internal_init_Context
            {
                device : rp_pico :: pac :: Peripherals :: steal(), cs : rtic
                :: export :: CriticalSection :: new(), core, local : init ::
                LocalResources :: new(),
            }
        }
    } #[allow(non_snake_case)] #[doc = "Initialization function"] pub mod init
    {
        #[doc(inline)] pub use super :: __rtic_internal_initLocalResources as
        LocalResources ; pub use super :: __rtic_internal_Monotonics as
        Monotonics ; pub use super :: __rtic_internal_init_Context as Context
        ;
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `idle` has access to"] pub struct
    __rtic_internal_idleLocalResources < > { pub x : & 'static mut u32, }
    #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_idle_Context < >
    {
        #[doc = r" Local Resources this task has access to"] pub local : idle
        :: LocalResources < >,
    } impl < > __rtic_internal_idle_Context < >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & rtic :: export :: Priority) -> Self
        {
            __rtic_internal_idle_Context
            { local : idle :: LocalResources :: new(), }
        }
    } #[allow(non_snake_case)] #[doc = "Idle loop"] pub mod idle
    {
        #[doc(inline)] pub use super :: __rtic_internal_idleLocalResources as
        LocalResources ; pub use super :: __rtic_internal_idle_Context as
        Context ;
    } mod shared_resources
    {
        use rtic :: export :: Priority ; #[doc(hidden)]
        #[allow(non_camel_case_types)] pub struct
        serial_that_needs_to_be_locked < 'a > { priority : & 'a Priority, }
        impl < 'a > serial_that_needs_to_be_locked < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { serial_that_needs_to_be_locked { priority } }
            #[inline(always)] pub unsafe fn priority(& self) -> & Priority
            { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct
        usb_dev_that_needs_to_be_locked < 'a > { priority : & 'a Priority, }
        impl < 'a > usb_dev_that_needs_to_be_locked < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { usb_dev_that_needs_to_be_locked { priority } }
            #[inline(always)] pub unsafe fn priority(& self) -> & Priority
            { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct
        counter_that_needs_to_be_locked < 'a > { priority : & 'a Priority, }
        impl < 'a > counter_that_needs_to_be_locked < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { counter_that_needs_to_be_locked { priority } }
            #[inline(always)] pub unsafe fn priority(& self) -> & Priority
            { self.priority }
        }
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `spi0_irq` has access to"] pub struct
    __rtic_internal_spi0_irqLocalResources < 'a >
    {
        pub spi_dev : & 'a mut hal :: Spi < hal :: spi :: Enabled, pac ::
        SPI0, 16 >,
    } #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_spi0_irq_Context
    < 'a >
    {
        #[doc = r" Local Resources this task has access to"] pub local :
        spi0_irq :: LocalResources < 'a >,
    } impl < 'a > __rtic_internal_spi0_irq_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_spi0_irq_Context
            { local : spi0_irq :: LocalResources :: new(), }
        }
    } #[allow(non_snake_case)] #[doc = "Hardware task"] pub mod spi0_irq
    {
        #[doc(inline)] pub use super :: __rtic_internal_spi0_irqLocalResources
        as LocalResources ; pub use super :: __rtic_internal_spi0_irq_Context
        as Context ;
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Shared resources `usb_rx` has access to"] pub struct
    __rtic_internal_usb_rxSharedResources < 'a >
    {
        pub serial : shared_resources :: serial_that_needs_to_be_locked < 'a
        >, pub usb_dev : shared_resources :: usb_dev_that_needs_to_be_locked <
        'a >, pub counter : shared_resources ::
        counter_that_needs_to_be_locked < 'a >,
    } #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_usb_rx_Context <
    'a >
    {
        #[doc = r" Shared Resources this task has access to"] pub shared :
        usb_rx :: SharedResources < 'a >,
    } impl < 'a > __rtic_internal_usb_rx_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_usb_rx_Context
            { shared : usb_rx :: SharedResources :: new(priority), }
        }
    } #[allow(non_snake_case)] #[doc = "Hardware task"] pub mod usb_rx
    {
        #[doc(inline)] pub use super :: __rtic_internal_usb_rxSharedResources
        as SharedResources ; pub use super :: __rtic_internal_usb_rx_Context
        as Context ;
    } #[doc = r" app module"] impl < > __rtic_internal_initLocalResources < >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_initLocalResources
            {
                usb_bus : & mut *
                __rtic_internal_local_init_usb_bus.get_mut(),
            }
        }
    } impl < > __rtic_internal_idleLocalResources < >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_idleLocalResources
            { x : & mut * __rtic_internal_local_idle_x.get_mut(), }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic0"] static
    __rtic_internal_shared_resource_serial : rtic :: RacyCell < core :: mem ::
    MaybeUninit < SerialPort < 'static, hal :: usb :: UsbBus > >> = rtic ::
    RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic
    :: Mutex for shared_resources :: serial_that_needs_to_be_locked < 'a >
    {
        type T = SerialPort < 'static, hal :: usb :: UsbBus > ;
        #[inline(always)] fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl
        FnOnce(& mut SerialPort < 'static, hal :: usb :: UsbBus >) ->
        RTIC_INTERNAL_R) -> RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_serial.get_mut() as * mut
                _, self.priority(), CEILING, rp_pico :: pac :: NVIC_PRIO_BITS,
                & __rtic_internal_MASKS, f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic1"] static
    __rtic_internal_shared_resource_usb_dev : rtic :: RacyCell < core :: mem
    :: MaybeUninit < usb_device :: device :: UsbDevice < 'static, hal :: usb
    :: UsbBus > >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic :: Mutex
    for shared_resources :: usb_dev_that_needs_to_be_locked < 'a >
    {
        type T = usb_device :: device :: UsbDevice < 'static, hal :: usb ::
        UsbBus > ; #[inline(always)] fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl
        FnOnce(& mut usb_device :: device :: UsbDevice < 'static, hal :: usb
        :: UsbBus >) -> RTIC_INTERNAL_R) -> RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_usb_dev.get_mut() as *
                mut _, self.priority(), CEILING, rp_pico :: pac ::
                NVIC_PRIO_BITS, & __rtic_internal_MASKS, f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic2"] static
    __rtic_internal_shared_resource_counter : rtic :: RacyCell < core :: mem
    :: MaybeUninit < Counter >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic :: Mutex
    for shared_resources :: counter_that_needs_to_be_locked < 'a >
    {
        type T = Counter ; #[inline(always)] fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl FnOnce(& mut Counter) -> RTIC_INTERNAL_R) ->
        RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_counter.get_mut() as *
                mut _, self.priority(), CEILING, rp_pico :: pac ::
                NVIC_PRIO_BITS, & __rtic_internal_MASKS, f,)
            }
        }
    } #[doc(hidden)] #[allow(non_upper_case_globals)] const
    __rtic_internal_MASKS : [u32 ; 3] =
    [rtic :: export :: create_mask([]), rtic :: export :: create_mask([]),
    rtic :: export ::
    create_mask([rp_pico :: pac :: Interrupt :: SPI0_IRQ as u32, rp_pico ::
    pac :: Interrupt :: USBCTRL_IRQ as u32])] ; #[allow(non_camel_case_types)]
    #[allow(non_upper_case_globals)] #[doc(hidden)]
    #[link_section = ".uninit.rtic3"] static
    __rtic_internal_local_resource_spi_dev : rtic :: RacyCell < core :: mem ::
    MaybeUninit < hal :: Spi < hal :: spi :: Enabled, pac :: SPI0, 16 > >> =
    rtic :: RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic4"] static
    __rtic_internal_local_resource_uart_dev : rtic :: RacyCell < core :: mem
    :: MaybeUninit < hal :: uart :: UartPeripheral < hal :: uart :: Enabled,
    pac :: UART0, (UartTx, UartRx) > >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_local_init_usb_bus : rtic ::
    RacyCell < Option < usb_device :: bus :: UsbBusAllocator < hal :: usb ::
    UsbBus > > > = rtic :: RacyCell :: new(None) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_local_idle_x : rtic :: RacyCell <
    u32 > = rtic :: RacyCell :: new(0) ; #[allow(non_snake_case)] #[no_mangle]
    unsafe fn SPI0_IRQ()
    {
        const PRIORITY : u8 = 3u8 ; rtic :: export ::
        run(PRIORITY, ||
        {
            spi0_irq(spi0_irq :: Context ::
            new(& rtic :: export :: Priority :: new(PRIORITY)))
        }) ;
    } impl < 'a > __rtic_internal_spi0_irqLocalResources < 'a >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_spi0_irqLocalResources
            {
                spi_dev : & mut *
                (& mut *
                __rtic_internal_local_resource_spi_dev.get_mut()).as_mut_ptr(),
            }
        }
    } #[allow(non_snake_case)] #[no_mangle] unsafe fn USBCTRL_IRQ()
    {
        const PRIORITY : u8 = 3u8 ; rtic :: export ::
        run(PRIORITY, ||
        {
            usb_rx(usb_rx :: Context ::
            new(& rtic :: export :: Priority :: new(PRIORITY)))
        }) ;
    } impl < 'a > __rtic_internal_usb_rxSharedResources < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_usb_rxSharedResources
            {
                serial : shared_resources :: serial_that_needs_to_be_locked ::
                new(priority), usb_dev : shared_resources ::
                usb_dev_that_needs_to_be_locked :: new(priority), counter :
                shared_resources :: counter_that_needs_to_be_locked ::
                new(priority),
            }
        }
    } #[doc(hidden)] mod rtic_ext
    {
        use super :: * ; #[no_mangle] unsafe extern "C" fn main() ->!
        {
            rtic :: export :: assert_send :: < SerialPort < 'static, hal ::
            usb :: UsbBus > > () ; rtic :: export :: assert_send :: <
            usb_device :: device :: UsbDevice < 'static, hal :: usb :: UsbBus
            > > () ; rtic :: export :: assert_send :: < Counter > () ; rtic ::
            export :: assert_send :: < hal :: Spi < hal :: spi :: Enabled, pac
            :: SPI0, 16 > > () ; rtic :: export :: assert_send :: < hal ::
            uart :: UartPeripheral < hal :: uart :: Enabled, pac :: UART0,
            (UartTx, UartRx) > > () ; const _CONST_CHECK : () =
            {
                if rtic :: export :: is_armv6()
                {
                    if(rp_pico :: pac :: Interrupt :: SPI0_IRQ as u32) > 31
                    {
                        :: core :: panic!
                        ("An interrupt above value 31 is used while in armv6") ;
                    } if(rp_pico :: pac :: Interrupt :: USBCTRL_IRQ as u32) > 31
                    {
                        :: core :: panic!
                        ("An interrupt above value 31 is used while in armv6") ;
                    }
                } else {}
            } ; let _ = _CONST_CHECK ; rtic :: export :: interrupt ::
            disable() ; let mut core : rtic :: export :: Peripherals = rtic ::
            export :: Peripherals :: steal().into() ; const _ : () =
            if(1 << rp_pico :: pac :: NVIC_PRIO_BITS) < 3u8 as usize
            {
                :: core :: panic!
                ("Maximum priority used by interrupt vector 'SPI0_IRQ' is more than supported by hardware")
                ;
            } ;
            core.NVIC.set_priority(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: SPI0_IRQ, rtic :: export ::
            logical2hw(3u8, rp_pico :: pac :: NVIC_PRIO_BITS),) ; rtic ::
            export :: NVIC ::
            unmask(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: SPI0_IRQ) ; const _ : () =
            if(1 << rp_pico :: pac :: NVIC_PRIO_BITS) < 3u8 as usize
            {
                :: core :: panic!
                ("Maximum priority used by interrupt vector 'USBCTRL_IRQ' is more than supported by hardware")
                ;
            } ;
            core.NVIC.set_priority(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: USBCTRL_IRQ, rtic :: export ::
            logical2hw(3u8, rp_pico :: pac :: NVIC_PRIO_BITS),) ; rtic ::
            export :: NVIC ::
            unmask(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: USBCTRL_IRQ) ; #[inline(never)] fn
            __rtic_init_resources < F > (f : F) where F : FnOnce() { f() ; }
            __rtic_init_resources(||
            {
                let(shared_resources, local_resources, mut monotonics) =
                init(init :: Context :: new(core.into())) ;
                __rtic_internal_shared_resource_serial.get_mut().write(core ::
                mem :: MaybeUninit :: new(shared_resources.serial)) ;
                __rtic_internal_shared_resource_usb_dev.get_mut().write(core
                :: mem :: MaybeUninit :: new(shared_resources.usb_dev)) ;
                __rtic_internal_shared_resource_counter.get_mut().write(core
                :: mem :: MaybeUninit :: new(shared_resources.counter)) ;
                __rtic_internal_local_resource_spi_dev.get_mut().write(core ::
                mem :: MaybeUninit :: new(local_resources.spi_dev)) ; rtic ::
                export :: interrupt :: enable() ;
            }) ;
            idle(idle :: Context ::
            new(& rtic :: export :: Priority :: new(0)))
        }
    }
}