#[doc = r" The RTIC application module"] pub mod app
{
    #[doc =
    r" Always include the device crate which contains the vector table"] use
    rp_pico :: pac as
    you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml ; use
    embedded_hal :: digital :: v2 :: OutputPin ; use embedded_hal :: blocking
    :: spi :: { Transfer, Write } ; use embedded_hal :: spi :: FullDuplex ;
    use fugit :: HertzU32 ; use rp_pico :: XOSC_CRYSTAL_FREQ ; use rp_pico ::
    pac :: Interrupt ; use rp_pico :: hal as hal ; use rp_pico :: pac ; use
    heapless :: { String, spsc :: { Consumer, Producer, Queue } } ; use hal ::
    {
        clocks :: Clock, uart :: { UartConfig, DataBits, StopBits }, gpio ::
        { pin :: bank0 :: *, Pin, FunctionUart }, pio ::
        { PIOExt, ShiftDirection, PIOBuilder, SM0, PinDir, },
    } ; use cortex_m :: peripheral :: NVIC ; use usb_device ::
    { class_prelude :: *, prelude :: * } ; use usbd_serial :: SerialPort ; use
    fugit :: RateExtU32 ; use crate :: fmt :: Wrapper ; use crate :: serial ::
    { match_usb_serial_buf, write_serial } ; use crate :: protocol ::
    {
        Send, ValidHostInterfaces, host ::
        { HostRequest, Clean, ValidOps, ValidInterfaces, }, slave ::
        { NotReady, SlaveResponse }
    } ; use core :: str ; #[doc = r" User code from within the module"] const
    UART0_ICR : * mut u32 = 0x4003_4044 as * mut u32 ; const SPI0_ICR : * mut
    u32 = 0x4003_c020 as * mut u32 ; const PIO0_IRQE : * mut u32 = 0x5020012c
    as * mut u32 ; const PIO0_IRQC : * mut u32 = 0x50200030 as * mut u32 ;
    #[doc = " Clock divider for the PIO SM"] const SMI_DEFAULT_CLKDIV : u16 =
    1 ; const PIO_CLK_DIV_FRAQ : u8 = 1 ; type UartTx = Pin < Gpio0,
    FunctionUart > ; type UartRx = Pin < Gpio1, FunctionUart > ;
    #[doc = r" User code end"] #[inline(always)] #[allow(non_snake_case)] fn
    init(c : init :: Context) -> (Shared, Local, init :: Monotonics)
    {
        unsafe { hal :: sio :: spinlock_reset() ; } let mut core = c.core ;
        let mut p = c.device ; let mut watchdog = hal :: watchdog :: Watchdog
        :: new(p.WATCHDOG) ;
        p.VREG_AND_CHIP_RESET.vreg.write(| w | unsafe { w.vsel().bits(14) }) ;
        let xosc = hal :: xosc ::
        setup_xosc_blocking(p.XOSC, rp_pico ::
        XOSC_CRYSTAL_FREQ.Hz()).map_err(| _x | false).unwrap() ;
        watchdog.enable_tick_generation((XOSC_CRYSTAL_FREQ / 1_000_000) as u8)
        ; let mut clocks = hal :: clocks :: ClocksManager :: new(p.CLOCKS) ;
        let pll_sys = hal :: pll ::
        setup_pll_blocking(p.PLL_SYS, xosc.operating_frequency(), hal :: pll
        :: PLLConfig
        {
            vco_freq : HertzU32 :: MHz(800), refdiv : 1, post_div1 : 3,
            post_div2 : 2,
        }, & mut clocks, & mut p.RESETS,).map_err(| _x | false).unwrap() ; let
        pll_usb = hal :: pll ::
        setup_pll_blocking(p.PLL_USB, xosc.operating_frequency(), hal :: pll
        :: common_configs :: PLL_USB_48MHZ, & mut clocks, & mut
        p.RESETS,).map_err(| _x | false).unwrap() ;
        clocks.init_default(& xosc, & pll_sys, &
        pll_usb).map_err(| _x | false).unwrap() ; let mut resets = p.RESETS ;
        let sio = hal :: Sio :: new(p.SIO) ; let pins = hal :: gpio :: Pins ::
        new(p.IO_BANK0, p.PADS_BANK0, sio.gpio_bank0, & mut resets,) ; let mut
        freepin = pins.gpio25.into_push_pull_output() ; let spi_sclk =
        pins.gpio18.into_mode :: < hal :: gpio :: FunctionSpi > () ; let
        spi_mosi = pins.gpio19.into_mode :: < hal :: gpio :: FunctionSpi > ()
        ; let spi_miso = pins.gpio16.into_mode :: < hal :: gpio :: FunctionSpi
        > () ; let spi_cs = pins.gpio17.into_mode :: < hal :: gpio ::
        FunctionSpi > () ; let mut spi_dev = hal :: Spi :: < _, _, 8 > ::
        new(p.SPI0) ; let mut spi_dev =
        spi_dev.init_slave(& mut resets, & embedded_hal :: spi :: MODE_0) ;
        let uart_pins =
        (pins.gpio0.into_mode :: < hal :: gpio :: FunctionUart > (),
        pins.gpio1.into_mode :: < hal :: gpio :: FunctionUart > (),) ; let mut
        uart_dev = hal :: uart :: UartPeripheral ::
        new(p.UART0, uart_pins, & mut
        resets).enable(UartConfig ::
        new(115200.Hz(), DataBits :: Eight, None, StopBits :: One),
        clocks.peripheral_clock.freq(),).unwrap() ;
        uart_dev.enable_rx_interrupt() ;
        uart_dev.set_rx_watermark(hal :: uart :: FifoWatermark :: Bytes28) ;
        let usb_bus : & 'static _ =
        c.local.usb_bus.insert(UsbBusAllocator ::
        new(hal :: usb :: UsbBus ::
        new(p.USBCTRL_REGS, p.USBCTRL_DPRAM, clocks.usb_clock, true, & mut
        resets,))) ; let serial = SerialPort :: new(usb_bus) ; let usb_dev =
        UsbDeviceBuilder ::
        new(usb_bus,
        UsbVidPid(0x16c0,
        0x27dd)).manufacturer("Validation").product("Serial port").serial_number("TEST").device_class(2).build()
        ; let _mdio_pin = pins.gpio8.into_mode :: < hal :: gpio ::
        FunctionPio0 > () ; let _mdc_pin = pins.gpio9.into_mode :: < hal ::
        gpio :: FunctionPio0 > () ; let program = pio_proc :: pio_asm!
        ("
        .side_set 1", ".wrap_target", "set pins, 0   side 0",
        "start:", "pull block side 0", "set pindirs, 1 side 0",
        "set x, 31 side 0", "preamble:", "set pins, 1 side 1      [4]",
        "set pins 1  side 0      [2]", "jmp x-- preamble side 0 [2]",
        "set pins, 0  side 1     [4]", "nop side 0 [2]",
        "set y, 11 side 0 [2]", "set pins, 1 side 1 [4]", "nop side 0 [1]",
        "addr:", "set x, 15 side 0 [3]", "out pins, 1   side 1 [4]",
        "jmp y-- addr side 0 [1]", "set pins, 0 side 0  [3]",
        "out y 1 side 1 [4]", "nop side 0 [4]", "nop side 1 [4]",
        "jmp y-- write_data    side 0 [2]", "set pindirs, 0 side 0 [2]",
        "read_data:", "in pins 1 side 1 [4]", "jmp x-- read_data side 0 [4]",
        "push side 0", "out null 19 side 0" "jmp start side 0", "write_data:",
        "nop side 0 [1]", "out pins, 1 side 1 [4]",
        "jmp x-- write_data side 0 [3]", "set pins 0 side 0",
        "out null 32 side 0", ".wrap",) ; let(mut pio0, sm0, _, _, _,) =
        p.PIO0.split(& mut resets) ; let installed =
        pio0.install(& program.program).unwrap() ; let(mut sm, smi_rx, smi_tx)
        = PIOBuilder ::
        from_program(installed).out_pins(5,
        1).side_set_pin_base(6).out_sticky(false).clock_divisor_fixed_point(SMI_DEFAULT_CLKDIV,
        PIO_CLK_DIV_FRAQ).out_shift_direction(ShiftDirection ::
        Right).in_shift_direction(ShiftDirection ::
        Left).autopush(true).autopull(false).set_pins(5,
        1).in_pin_base(5).build(sm0) ; sm.set_pindirs([(5, PinDir :: Output)])
        ; sm.set_pindirs([(6, PinDir :: Output)]) ; let smi_master =
        sm.start() ; let serial_buf = [0_u8 ; 64] ; let _spi_tx_buf =
        [0_u16 ; 9] ; let(mut spi_tx_producer, spi_tx_consumer) =
        c.local.spi_q.split() ; spi_tx_producer.enqueue([0_u8 ; 18]).unwrap()
        ; freepin.set_low().unwrap() ; let(producer, consumer) =
        c.local.q.split() ; let(host_producer, host_consumer) =
        c.local.host_q.split() ; unsafe
        { core :: ptr :: write_volatile(PIO0_IRQE, 0x1) ; }
        core.SCB.set_sleepdeep() ;
        (Shared
        {
            serial, usb_dev, pio0, smi_master, smi_tx, smi_rx, serial_buf,
            _spi_tx_buf, host_producer, freepin, spi_dev : spi_dev,
        }, Local
        {
            uart_dev : uart_dev, spi_tx_producer, spi_tx_consumer,
            host_consumer, producer, consumer,
        }, init :: Monotonics(),)
    } #[allow(non_snake_case)] fn idle(_cx : idle :: Context) ->!
    {
        use rtic :: Mutex as _ ; use rtic :: mutex_prelude :: * ; loop
        { rtic :: export :: wfi() }
    } #[allow(non_snake_case)] fn uart0(cx : uart0 :: Context)
    {
        use rtic :: Mutex as _ ; use rtic :: mutex_prelude :: * ; let uart =
        cx.local.uart_dev ; let host_producer = cx.shared.host_producer ; let
        mut buffer = [0_u8 ; 64] ; let serial = cx.shared.serial ;
    } #[inline(never)] #[link_section = ".data.bar"] #[allow(non_snake_case)]
    fn spi0(cx : spi0 :: Context)
    {
        use rtic :: Mutex as _ ; use rtic :: mutex_prelude :: * ; let mut
        serial = cx.shared.serial ; let mut freepin = cx.shared.freepin ;
        (serial,
        freepin).lock(| serial, freepin |
        { freepin.set_high() ; write_serial(serial, "RX IRQ\n\r", false) ; })
        ;
    } #[inline(never)] #[link_section = ".data.bar"] #[allow(non_snake_case)]
    fn usb_rx(cx : usb_rx :: Context)
    {
        use rtic :: Mutex as _ ; use rtic :: mutex_prelude :: * ; let usb_dev
        = cx.shared.usb_dev ; let serial = cx.shared.serial ; let serial_buf =
        cx.shared.serial_buf ; let freepin = cx.shared.freepin ; let
        host_producer = cx.shared.host_producer ;
        (usb_dev, serial, serial_buf, freepin,
        host_producer).lock(| usb_dev_a, serial_a, serial_buf, freepin,
        host_producer |
        {
            if usb_dev_a.poll(& mut [serial_a])
            {
                let mut buf = [0u8 ; 64] ; match serial_a.read(& mut buf)
                {
                    Err(_e) => {} Ok(0) =>
                    {
                        let _ = serial_a.write(b"Didn't received data.\n\r") ; let _
                        = serial_a.flush() ;
                    } Ok(_count) =>
                    {
                        match buf [0]
                        {
                            b'\r' =>
                            {
                                let first_zero = serial_buf.iter().position(| & x | x == 0)
                                ; match first_zero
                                {
                                    Some(Index) => { serial_buf [Index] = b' ' ; } None =>
                                    { for elem in serial_buf.iter_mut() { * elem = 0 ; } }
                                } match match_usb_serial_buf(serial_buf, serial_a)
                                {
                                    Ok(hr) =>
                                    {
                                        let clean = hr.init_clean() ; match clean
                                        {
                                            Ok(hr) =>
                                            {
                                                match host_producer.enqueue(hr)
                                                {
                                                    Ok(..) => {} Err(..) =>
                                                    {
                                                        write_serial(serial_a,
                                                        "Error Pushing Host Request to queue\n\r", false) ;
                                                    }
                                                } ; send_out :: spawn().unwrap() ;
                                            } Err(err) => { write_serial(serial_a, err, false) ; }
                                        }
                                    } Err("Ok") => {} Err(err) =>
                                    { write_serial(serial_a, err, false) ; }
                                } for elem in serial_buf.iter_mut() { * elem = 0 ; }
                            } _ =>
                            {
                                let first_zero = serial_buf.iter().position(| & x | x == 0)
                                ; match first_zero
                                {
                                    Some(Index) => { serial_buf [Index] = buf [0] ; } None =>
                                    { for elem in serial_buf.iter_mut() { * elem = 0 ; } }
                                } let command = str ::
                                from_utf8(& mut buf [0 .. 1]).unwrap() ;
                                write_serial(serial_a, command, false) ;
                            }
                        }
                    }
                }
            }
        })
    } #[allow(non_snake_case)] fn pio_sm_rx(cx : pio_sm_rx :: Context)
    {
        use rtic :: Mutex as _ ; use rtic :: mutex_prelude :: * ; unsafe
        {
            core :: ptr :: write_volatile(PIO0_IRQC, 0xFFF) ; NVIC ::
            unpend(pac :: Interrupt :: PIO0_IRQ_0) ;
        } let mut serial = cx.shared.serial ;
        (serial).lock(| serial | { write_serial(serial, "fired", false) ; }) ;
        if let Some(mut slave_response) = cx.local.consumer.dequeue()
        {
            let pio0 = cx.shared.pio0 ; let rx = cx.shared.smi_rx ;
            (pio0, rx,
            serial).lock(| pio0, rx_a, serial, |
            {
                match rx_a.read()
                {
                    Some(word) => { slave_response.set_payload(word) ; } _ => {}
                } pio0.clear_irq(0xF) ; match slave_response.init_ready()
                {
                    Ok(sr) =>
                    {
                        write_serial(serial, "Hi", false) ; respond_to_host ::
                        spawn(sr) ;
                    } Err(err) => { write_serial(serial, err, false) ; }
                }
            })
        } else {}
    } #[allow(non_snake_case)] fn send_out(cx : send_out :: Context)
    {
        use rtic :: Mutex as _ ; use rtic :: mutex_prelude :: * ; let mut
        slave_response = false ; let freepin = cx.shared.freepin ; let smi_tx
        = cx.shared.smi_tx ; let smi_rx = cx.shared.smi_rx ; let smi_master =
        cx.shared.smi_master ; let serial = cx.shared.serial ; let producer =
        cx.local.producer ; let mut return_string = "\n\r->" ; let mut hr =
        cx.local.host_consumer.dequeue() ; match hr
        {
            Some(mut hr) =>
            {
                (freepin, smi_tx, smi_rx, smi_master,
                serial).lock(| freepin, smi_tx, smi_rx, smi_master, serial |
                {
                    match hr.interface
                    {
                        ValidInterfaces :: SMI =>
                        {
                            smi_tx.write(hr.payload [0]) ; smi_rx.read() ;
                            slave_response = true ;
                        } ValidInterfaces :: Config =>
                        {
                            if hr.operation == ValidOps :: SmiSet
                            {
                                if hr.payload [0] == 25
                                {
                                    smi_master.set_clock_divisor(4.56640625) ; return_string =
                                    "\n\rSMI Clock Rate Set 2.5Mhz\n\r->" ;
                                } else if hr.payload [0] == 10
                                {
                                    smi_master.clock_divisor_fixed_point(1, 145) ; return_string
                                    = "\n\rSMI Clock Rate Set 10Mhz\n\r->" ;
                                } else
                                {
                                    smi_master.clock_divisor_fixed_point(hr.payload [0] as u16,
                                    0) ;
                                }
                            }
                        } ValidInterfaces :: GPIO =>
                        {
                            if hr.payload [0] != 0 { freepin.set_high().unwrap() ; }
                            else { freepin.set_low().unwrap() ; }
                        } _ => {}
                    } write_serial(serial, return_string, false) ; if
                    slave_response
                    {
                        let slave_response = hr.exchange_for_slave_response() ;
                        match slave_response
                        {
                            Ok(val) =>
                            {
                                match producer.enqueue(val)
                                {
                                    Ok(sr) => {} Err(err) =>
                                    { write_serial(serial, "Consumer queue is full", false) ; }
                                }
                            } Err(_err) => {}
                        }
                    }
                }) ;
            } None => {}
        }
    } #[allow(non_snake_case)] fn
    respond_to_host(cx : respond_to_host :: Context, sr : SlaveResponse <
    crate :: protocol :: slave :: Ready >)
    {
        use rtic :: Mutex as _ ; use rtic :: mutex_prelude :: * ; let mut
        serial = cx.shared.serial ; if sr.host_config == ValidHostInterfaces
        :: SPI
        {
            let mut tx_buf = [0_u8 ; 18] ; tx_buf [0] = sr.proc_id as u8 ; let
            split_u32_to_u8 = | word : u32 | -> (u8, u8, u8, u8)
            {
                (((word >> 24) & 0xff) as u8, ((word >> 16) & 0xff) as u8,
                ((word >> 8) & 0xff) as u8, (word & 0xff) as u8)
            } ; (tx_buf [1], tx_buf [2], tx_buf [3], tx_buf [4]) =
            split_u32_to_u8(sr.payload) ;
            cx.local.spi_tx_producer.enqueue(tx_buf).unwrap() ;
        } else if sr.host_config == ValidHostInterfaces :: Serial
        {
            (serial).lock(| serial |
            {
                let mut data = String :: < 32 > :: new() ; let mut buf =
                [0 as u8, 20] ; write!
                (Wrapper :: new(& mut buf), "{}",
                sr.payload).expect("Can't Write") ;
                write_serial(serial, out, false) ;
            }) ;
        }
    } struct Shared
    {
        serial : SerialPort < 'static, hal :: usb :: UsbBus >, usb_dev :
        usb_device :: device :: UsbDevice < 'static, hal :: usb :: UsbBus >,
        pio0 : hal :: pio :: PIO < pac :: PIO0 >, smi_master : hal :: pio ::
        StateMachine < (pac :: PIO0, SM0), hal :: pio :: Running >, smi_tx :
        hal :: pio :: Tx < (pac :: PIO0, SM0) >, smi_rx : hal :: pio :: Rx <
        (pac :: PIO0, SM0) >, serial_buf : [u8 ; 64], host_producer : Producer
        < 'static, HostRequest < Clean >, 3 >, _spi_tx_buf : [u16 ; 9],
        freepin : Pin < Gpio25, hal :: gpio :: Output < hal :: gpio ::
        PushPull > >, spi_dev : hal :: Spi < hal :: spi :: Enabled, pac ::
        SPI0, 8 >,
    } struct Local
    {
        uart_dev : hal :: uart :: UartPeripheral < hal :: uart :: Enabled, pac
        :: UART0, (UartTx, UartRx) >, spi_tx_producer : Producer < 'static,
        [u8 ; 18], 3 >, spi_tx_consumer : Consumer < 'static, [u8 ; 18], 3 >,
        host_consumer : Consumer < 'static, HostRequest < Clean >, 3 >,
        producer : Producer < 'static, SlaveResponse < NotReady >, 3 >,
        consumer : Consumer < 'static, SlaveResponse < NotReady >, 3 >,
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `init` has access to"] pub struct
    __rtic_internal_initLocalResources < >
    {
        pub usb_bus : & 'static mut Option < usb_device :: bus ::
        UsbBusAllocator < hal :: usb :: UsbBus > >, pub spi_q : & 'static mut
        Queue < [u8 ; 18], 3 >, pub q : & 'static mut Queue < SlaveResponse <
        NotReady >, 3 >, pub host_q : & 'static mut Queue < HostRequest <
        Clean >, 3 >,
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
    #[doc = "Shared resources `idle` has access to"] pub struct
    __rtic_internal_idleSharedResources < 'a >
    { pub spi_dev : shared_resources :: spi_dev < 'a >, }
    #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_idle_Context <
    'a >
    {
        #[doc = r" Shared Resources this task has access to"] pub shared :
        idle :: SharedResources < 'a >,
    } impl < 'a > __rtic_internal_idle_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_idle_Context
            { shared : idle :: SharedResources :: new(priority), }
        }
    } #[allow(non_snake_case)] #[doc = "Idle loop"] pub mod idle
    {
        #[doc(inline)] pub use super :: __rtic_internal_idleSharedResources as
        SharedResources ; pub use super :: __rtic_internal_idle_Context as
        Context ;
    } mod shared_resources
    {
        use rtic :: export :: Priority ; #[doc(hidden)]
        #[allow(non_camel_case_types)] pub struct serial < 'a >
        { priority : & 'a Priority, } impl < 'a > serial < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { serial { priority } } #[inline(always)] pub unsafe fn
            priority(& self) -> & Priority { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct usb_dev <
        'a > { priority : & 'a Priority, } impl < 'a > usb_dev < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { usb_dev { priority } } #[inline(always)] pub unsafe fn
            priority(& self) -> & Priority { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct pio0 < 'a >
        { priority : & 'a Priority, } impl < 'a > pio0 < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { pio0 { priority } } #[inline(always)] pub unsafe fn
            priority(& self) -> & Priority { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct smi_master
        < 'a > { priority : & 'a Priority, } impl < 'a > smi_master < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { smi_master { priority } } #[inline(always)] pub unsafe fn
            priority(& self) -> & Priority { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct smi_tx < 'a
        > { priority : & 'a Priority, } impl < 'a > smi_tx < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { smi_tx { priority } } #[inline(always)] pub unsafe fn
            priority(& self) -> & Priority { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct smi_rx < 'a
        > { priority : & 'a Priority, } impl < 'a > smi_rx < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { smi_rx { priority } } #[inline(always)] pub unsafe fn
            priority(& self) -> & Priority { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct serial_buf
        < 'a > { priority : & 'a Priority, } impl < 'a > serial_buf < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { serial_buf { priority } } #[inline(always)] pub unsafe fn
            priority(& self) -> & Priority { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct
        host_producer < 'a > { priority : & 'a Priority, } impl < 'a >
        host_producer < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { host_producer { priority } } #[inline(always)] pub unsafe
            fn priority(& self) -> & Priority { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct freepin <
        'a > { priority : & 'a Priority, } impl < 'a > freepin < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { freepin { priority } } #[inline(always)] pub unsafe fn
            priority(& self) -> & Priority { self.priority }
        } #[doc(hidden)] #[allow(non_camel_case_types)] pub struct spi_dev <
        'a > { priority : & 'a Priority, } impl < 'a > spi_dev < 'a >
        {
            #[inline(always)] pub unsafe fn new(priority : & 'a Priority) ->
            Self { spi_dev { priority } } #[inline(always)] pub unsafe fn
            priority(& self) -> & Priority { self.priority }
        }
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `uart0` has access to"] pub struct
    __rtic_internal_uart0LocalResources < 'a >
    {
        pub uart_dev : & 'a mut hal :: uart :: UartPeripheral < hal :: uart ::
        Enabled, pac :: UART0, (UartTx, UartRx) >,
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Shared resources `uart0` has access to"] pub struct
    __rtic_internal_uart0SharedResources < 'a >
    {
        pub serial : shared_resources :: serial < 'a >, pub host_producer :
        shared_resources :: host_producer < 'a >,
    } #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_uart0_Context <
    'a >
    {
        #[doc = r" Local Resources this task has access to"] pub local : uart0
        :: LocalResources < 'a >,
        #[doc = r" Shared Resources this task has access to"] pub shared :
        uart0 :: SharedResources < 'a >,
    } impl < 'a > __rtic_internal_uart0_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_uart0_Context
            {
                local : uart0 :: LocalResources :: new(), shared : uart0 ::
                SharedResources :: new(priority),
            }
        }
    } #[allow(non_snake_case)] #[doc = "Hardware task"] pub mod uart0
    {
        #[doc(inline)] pub use super :: __rtic_internal_uart0LocalResources as
        LocalResources ; #[doc(inline)] pub use super ::
        __rtic_internal_uart0SharedResources as SharedResources ; pub use
        super :: __rtic_internal_uart0_Context as Context ;
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `spi0` has access to"] pub struct
    __rtic_internal_spi0LocalResources < 'a >
    { pub spi_tx_consumer : & 'a mut Consumer < 'static, [u8 ; 18], 3 >, }
    #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Shared resources `spi0` has access to"] pub struct
    __rtic_internal_spi0SharedResources < 'a >
    {
        pub spi_dev : shared_resources :: spi_dev < 'a >, pub serial :
        shared_resources :: serial < 'a >, pub host_producer :
        shared_resources :: host_producer < 'a >, pub freepin :
        shared_resources :: freepin < 'a >,
    } #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_spi0_Context <
    'a >
    {
        #[doc = r" Local Resources this task has access to"] pub local : spi0
        :: LocalResources < 'a >,
        #[doc = r" Shared Resources this task has access to"] pub shared :
        spi0 :: SharedResources < 'a >,
    } impl < 'a > __rtic_internal_spi0_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_spi0_Context
            {
                local : spi0 :: LocalResources :: new(), shared : spi0 ::
                SharedResources :: new(priority),
            }
        }
    } #[allow(non_snake_case)] #[doc = "Hardware task"] pub mod spi0
    {
        #[doc(inline)] pub use super :: __rtic_internal_spi0LocalResources as
        LocalResources ; #[doc(inline)] pub use super ::
        __rtic_internal_spi0SharedResources as SharedResources ; pub use super
        :: __rtic_internal_spi0_Context as Context ;
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Shared resources `usb_rx` has access to"] pub struct
    __rtic_internal_usb_rxSharedResources < 'a >
    {
        pub serial : shared_resources :: serial < 'a >, pub usb_dev :
        shared_resources :: usb_dev < 'a >, pub serial_buf : shared_resources
        :: serial_buf < 'a >, pub freepin : shared_resources :: freepin < 'a
        >, pub host_producer : shared_resources :: host_producer < 'a >,
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
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `pio_sm_rx` has access to"] pub struct
    __rtic_internal_pio_sm_rxLocalResources < 'a >
    {
        pub consumer : & 'a mut Consumer < 'static, SlaveResponse < NotReady
        >, 3 >,
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Shared resources `pio_sm_rx` has access to"] pub struct
    __rtic_internal_pio_sm_rxSharedResources < 'a >
    {
        pub serial : shared_resources :: serial < 'a >, pub pio0 :
        shared_resources :: pio0 < 'a >, pub smi_rx : shared_resources ::
        smi_rx < 'a >,
    } #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct
    __rtic_internal_pio_sm_rx_Context < 'a >
    {
        #[doc = r" Local Resources this task has access to"] pub local :
        pio_sm_rx :: LocalResources < 'a >,
        #[doc = r" Shared Resources this task has access to"] pub shared :
        pio_sm_rx :: SharedResources < 'a >,
    } impl < 'a > __rtic_internal_pio_sm_rx_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_pio_sm_rx_Context
            {
                local : pio_sm_rx :: LocalResources :: new(), shared :
                pio_sm_rx :: SharedResources :: new(priority),
            }
        }
    } #[allow(non_snake_case)] #[doc = "Hardware task"] pub mod pio_sm_rx
    {
        #[doc(inline)] pub use super ::
        __rtic_internal_pio_sm_rxLocalResources as LocalResources ;
        #[doc(inline)] pub use super ::
        __rtic_internal_pio_sm_rxSharedResources as SharedResources ; pub use
        super :: __rtic_internal_pio_sm_rx_Context as Context ;
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `send_out` has access to"] pub struct
    __rtic_internal_send_outLocalResources < 'a >
    {
        pub producer : & 'a mut Producer < 'static, SlaveResponse < NotReady
        >, 3 >, pub host_consumer : & 'a mut Consumer < 'static, HostRequest <
        Clean >, 3 >,
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Shared resources `send_out` has access to"] pub struct
    __rtic_internal_send_outSharedResources < 'a >
    {
        pub serial : shared_resources :: serial < 'a >, pub smi_master :
        shared_resources :: smi_master < 'a >, pub smi_tx : shared_resources
        :: smi_tx < 'a >, pub smi_rx : shared_resources :: smi_rx < 'a >, pub
        freepin : shared_resources :: freepin < 'a >,
    } #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_send_out_Context
    < 'a >
    {
        #[doc = r" Local Resources this task has access to"] pub local :
        send_out :: LocalResources < 'a >,
        #[doc = r" Shared Resources this task has access to"] pub shared :
        send_out :: SharedResources < 'a >,
    } impl < 'a > __rtic_internal_send_out_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_send_out_Context
            {
                local : send_out :: LocalResources :: new(), shared : send_out
                :: SharedResources :: new(priority),
            }
        }
    } #[doc = r" Spawns the task directly"] pub fn
    __rtic_internal_send_out_spawn() -> Result < (), () >
    {
        let input = () ; unsafe
        {
            if let Some(index) = rtic :: export :: interrupt ::
            free(| _ |
            (& mut * __rtic_internal_send_out_FQ.get_mut()).dequeue())
            {
                (& mut *
                __rtic_internal_send_out_INPUTS.get_mut()).get_unchecked_mut(usize
                :: from(index)).as_mut_ptr().write(input) ; rtic :: export ::
                interrupt ::
                free(| _ |
                {
                    (& mut *
                    __rtic_internal_P3_RQ.get_mut()).enqueue_unchecked((P3_T ::
                    send_out, index)) ;
                }) ; rtic :: pend(rp_pico :: pac :: interrupt :: PWM_IRQ_WRAP)
                ; Ok(())
            } else { Err(input) }
        }
    } #[allow(non_snake_case)] #[doc = "Software task"] pub mod send_out
    {
        #[doc(inline)] pub use super :: __rtic_internal_send_outLocalResources
        as LocalResources ; #[doc(inline)] pub use super ::
        __rtic_internal_send_outSharedResources as SharedResources ; pub use
        super :: __rtic_internal_send_out_Context as Context ; pub use super
        :: __rtic_internal_send_out_spawn as spawn ;
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `respond_to_host` has access to"] pub struct
    __rtic_internal_respond_to_hostLocalResources < 'a >
    { pub spi_tx_producer : & 'a mut Producer < 'static, [u8 ; 18], 3 >, }
    #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Shared resources `respond_to_host` has access to"] pub struct
    __rtic_internal_respond_to_hostSharedResources < 'a >
    { pub serial : shared_resources :: serial < 'a >, }
    #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct
    __rtic_internal_respond_to_host_Context < 'a >
    {
        #[doc = r" Local Resources this task has access to"] pub local :
        respond_to_host :: LocalResources < 'a >,
        #[doc = r" Shared Resources this task has access to"] pub shared :
        respond_to_host :: SharedResources < 'a >,
    } impl < 'a > __rtic_internal_respond_to_host_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_respond_to_host_Context
            {
                local : respond_to_host :: LocalResources :: new(), shared :
                respond_to_host :: SharedResources :: new(priority),
            }
        }
    } #[doc = r" Spawns the task directly"] pub fn
    __rtic_internal_respond_to_host_spawn(_0 : SlaveResponse < crate ::
    protocol :: slave :: Ready >,) -> Result < (), SlaveResponse < crate ::
    protocol :: slave :: Ready > >
    {
        let input = _0 ; unsafe
        {
            if let Some(index) = rtic :: export :: interrupt ::
            free(| _ |
            (& mut * __rtic_internal_respond_to_host_FQ.get_mut()).dequeue())
            {
                (& mut *
                __rtic_internal_respond_to_host_INPUTS.get_mut()).get_unchecked_mut(usize
                :: from(index)).as_mut_ptr().write(input) ; rtic :: export ::
                interrupt ::
                free(| _ |
                {
                    (& mut *
                    __rtic_internal_P3_RQ.get_mut()).enqueue_unchecked((P3_T ::
                    respond_to_host, index)) ;
                }) ; rtic :: pend(rp_pico :: pac :: interrupt :: PWM_IRQ_WRAP)
                ; Ok(())
            } else { Err(input) }
        }
    } #[allow(non_snake_case)] #[doc = "Software task"] pub mod
    respond_to_host
    {
        #[doc(inline)] pub use super ::
        __rtic_internal_respond_to_hostLocalResources as LocalResources ;
        #[doc(inline)] pub use super ::
        __rtic_internal_respond_to_hostSharedResources as SharedResources ;
        pub use super :: __rtic_internal_respond_to_host_Context as Context ;
        pub use super :: __rtic_internal_respond_to_host_spawn as spawn ;
    } #[doc = r" app module"] impl < > __rtic_internal_initLocalResources < >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_initLocalResources
            {
                usb_bus : & mut *
                __rtic_internal_local_init_usb_bus.get_mut(), spi_q : & mut *
                __rtic_internal_local_init_spi_q.get_mut(), q : & mut *
                __rtic_internal_local_init_q.get_mut(), host_q : & mut *
                __rtic_internal_local_init_host_q.get_mut(),
            }
        }
    } impl < 'a > __rtic_internal_idleSharedResources < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_idleSharedResources
            { spi_dev : shared_resources :: spi_dev :: new(priority), }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic0"] static
    __rtic_internal_shared_resource_serial : rtic :: RacyCell < core :: mem ::
    MaybeUninit < SerialPort < 'static, hal :: usb :: UsbBus > >> = rtic ::
    RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic
    :: Mutex for shared_resources :: serial < 'a >
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
                f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic1"] static
    __rtic_internal_shared_resource_usb_dev : rtic :: RacyCell < core :: mem
    :: MaybeUninit < usb_device :: device :: UsbDevice < 'static, hal :: usb
    :: UsbBus > >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic :: Mutex
    for shared_resources :: usb_dev < 'a >
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
                NVIC_PRIO_BITS, f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic2"] static
    __rtic_internal_shared_resource_pio0 : rtic :: RacyCell < core :: mem ::
    MaybeUninit < hal :: pio :: PIO < pac :: PIO0 > >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic :: Mutex
    for shared_resources :: pio0 < 'a >
    {
        type T = hal :: pio :: PIO < pac :: PIO0 > ; #[inline(always)] fn lock
        < RTIC_INTERNAL_R >
        (& mut self, f : impl FnOnce(& mut hal :: pio :: PIO < pac :: PIO0 >)
        -> RTIC_INTERNAL_R) -> RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_pio0.get_mut() as * mut
                _, self.priority(), CEILING, rp_pico :: pac :: NVIC_PRIO_BITS,
                f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic3"] static
    __rtic_internal_shared_resource_smi_master : rtic :: RacyCell < core ::
    mem :: MaybeUninit < hal :: pio :: StateMachine < (pac :: PIO0, SM0), hal
    :: pio :: Running > >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic :: Mutex
    for shared_resources :: smi_master < 'a >
    {
        type T = hal :: pio :: StateMachine < (pac :: PIO0, SM0), hal :: pio
        :: Running > ; #[inline(always)] fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl
        FnOnce(& mut hal :: pio :: StateMachine < (pac :: PIO0, SM0), hal ::
        pio :: Running >) -> RTIC_INTERNAL_R) -> RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_smi_master.get_mut() as *
                mut _, self.priority(), CEILING, rp_pico :: pac ::
                NVIC_PRIO_BITS, f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic4"] static
    __rtic_internal_shared_resource_smi_tx : rtic :: RacyCell < core :: mem ::
    MaybeUninit < hal :: pio :: Tx < (pac :: PIO0, SM0) > >> = rtic ::
    RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic
    :: Mutex for shared_resources :: smi_tx < 'a >
    {
        type T = hal :: pio :: Tx < (pac :: PIO0, SM0) > ; #[inline(always)]
        fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl
        FnOnce(& mut hal :: pio :: Tx < (pac :: PIO0, SM0) >) ->
        RTIC_INTERNAL_R) -> RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_smi_tx.get_mut() as * mut
                _, self.priority(), CEILING, rp_pico :: pac :: NVIC_PRIO_BITS,
                f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic5"] static
    __rtic_internal_shared_resource_smi_rx : rtic :: RacyCell < core :: mem ::
    MaybeUninit < hal :: pio :: Rx < (pac :: PIO0, SM0) > >> = rtic ::
    RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic
    :: Mutex for shared_resources :: smi_rx < 'a >
    {
        type T = hal :: pio :: Rx < (pac :: PIO0, SM0) > ; #[inline(always)]
        fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl
        FnOnce(& mut hal :: pio :: Rx < (pac :: PIO0, SM0) >) ->
        RTIC_INTERNAL_R) -> RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_smi_rx.get_mut() as * mut
                _, self.priority(), CEILING, rp_pico :: pac :: NVIC_PRIO_BITS,
                f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic6"] static
    __rtic_internal_shared_resource_serial_buf : rtic :: RacyCell < core ::
    mem :: MaybeUninit < [u8 ; 64] >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic :: Mutex
    for shared_resources :: serial_buf < 'a >
    {
        type T = [u8 ; 64] ; #[inline(always)] fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl FnOnce(& mut [u8 ; 64]) -> RTIC_INTERNAL_R) ->
        RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_serial_buf.get_mut() as *
                mut _, self.priority(), CEILING, rp_pico :: pac ::
                NVIC_PRIO_BITS, f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic7"] static
    __rtic_internal_shared_resource_host_producer : rtic :: RacyCell < core ::
    mem :: MaybeUninit < Producer < 'static, HostRequest < Clean >, 3 > >> =
    rtic :: RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ; impl <
    'a > rtic :: Mutex for shared_resources :: host_producer < 'a >
    {
        type T = Producer < 'static, HostRequest < Clean >, 3 > ;
        #[inline(always)] fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl
        FnOnce(& mut Producer < 'static, HostRequest < Clean >, 3 >) ->
        RTIC_INTERNAL_R) -> RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_host_producer.get_mut()
                as * mut _, self.priority(), CEILING, rp_pico :: pac ::
                NVIC_PRIO_BITS, f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic8"] static
    __rtic_internal_shared_resource__spi_tx_buf : rtic :: RacyCell < core ::
    mem :: MaybeUninit < [u16 ; 9] >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic9"] static
    __rtic_internal_shared_resource_freepin : rtic :: RacyCell < core :: mem
    :: MaybeUninit < Pin < Gpio25, hal :: gpio :: Output < hal :: gpio ::
    PushPull > > >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ; impl < 'a > rtic :: Mutex
    for shared_resources :: freepin < 'a >
    {
        type T = Pin < Gpio25, hal :: gpio :: Output < hal :: gpio :: PushPull
        > > ; #[inline(always)] fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl
        FnOnce(& mut Pin < Gpio25, hal :: gpio :: Output < hal :: gpio ::
        PushPull > >) -> RTIC_INTERNAL_R) -> RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 3u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_freepin.get_mut() as *
                mut _, self.priority(), CEILING, rp_pico :: pac ::
                NVIC_PRIO_BITS, f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic10"] static
    __rtic_internal_shared_resource_spi_dev : rtic :: RacyCell < core :: mem
    :: MaybeUninit < hal :: Spi < hal :: spi :: Enabled, pac :: SPI0, 8 > >> =
    rtic :: RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ; impl <
    'a > rtic :: Mutex for shared_resources :: spi_dev < 'a >
    {
        type T = hal :: Spi < hal :: spi :: Enabled, pac :: SPI0, 8 > ;
        #[inline(always)] fn lock < RTIC_INTERNAL_R >
        (& mut self, f : impl
        FnOnce(& mut hal :: Spi < hal :: spi :: Enabled, pac :: SPI0, 8 >) ->
        RTIC_INTERNAL_R) -> RTIC_INTERNAL_R
        {
            #[doc = r" Priority ceiling"] const CEILING : u8 = 2u8 ; unsafe
            {
                rtic :: export ::
                lock(__rtic_internal_shared_resource_spi_dev.get_mut() as *
                mut _, self.priority(), CEILING, rp_pico :: pac ::
                NVIC_PRIO_BITS, f,)
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic11"] static
    __rtic_internal_local_resource_uart_dev : rtic :: RacyCell < core :: mem
    :: MaybeUninit < hal :: uart :: UartPeripheral < hal :: uart :: Enabled,
    pac :: UART0, (UartTx, UartRx) > >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic12"] static
    __rtic_internal_local_resource_spi_tx_producer : rtic :: RacyCell < core
    :: mem :: MaybeUninit < Producer < 'static, [u8 ; 18], 3 > >> = rtic ::
    RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic13"] static
    __rtic_internal_local_resource_spi_tx_consumer : rtic :: RacyCell < core
    :: mem :: MaybeUninit < Consumer < 'static, [u8 ; 18], 3 > >> = rtic ::
    RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic14"] static
    __rtic_internal_local_resource_host_consumer : rtic :: RacyCell < core ::
    mem :: MaybeUninit < Consumer < 'static, HostRequest < Clean >, 3 > >> =
    rtic :: RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic15"] static
    __rtic_internal_local_resource_producer : rtic :: RacyCell < core :: mem
    :: MaybeUninit < Producer < 'static, SlaveResponse < NotReady >, 3 > >> =
    rtic :: RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic16"] static
    __rtic_internal_local_resource_consumer : rtic :: RacyCell < core :: mem
    :: MaybeUninit < Consumer < 'static, SlaveResponse < NotReady >, 3 > >> =
    rtic :: RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_local_init_usb_bus : rtic ::
    RacyCell < Option < usb_device :: bus :: UsbBusAllocator < hal :: usb ::
    UsbBus > > > = rtic :: RacyCell :: new(None) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_local_init_spi_q : rtic :: RacyCell
    < Queue < [u8 ; 18], 3 > > = rtic :: RacyCell :: new(Queue :: new()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_local_init_q : rtic :: RacyCell <
    Queue < SlaveResponse < NotReady >, 3 > > = rtic :: RacyCell ::
    new(Queue :: new()) ; #[allow(non_camel_case_types)]
    #[allow(non_upper_case_globals)] #[doc(hidden)] static
    __rtic_internal_local_init_host_q : rtic :: RacyCell < Queue < HostRequest
    < Clean >, 3 > > = rtic :: RacyCell :: new(Queue :: new()) ;
    #[allow(non_snake_case)] #[no_mangle] unsafe fn UART0_IRQ()
    {
        const PRIORITY : u8 = 3u8 ; rtic :: export ::
        run(PRIORITY, ||
        {
            uart0(uart0 :: Context ::
            new(& rtic :: export :: Priority :: new(PRIORITY)))
        }) ;
    } impl < 'a > __rtic_internal_uart0LocalResources < 'a >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_uart0LocalResources
            {
                uart_dev : & mut *
                (& mut *
                __rtic_internal_local_resource_uart_dev.get_mut()).as_mut_ptr(),
            }
        }
    } impl < 'a > __rtic_internal_uart0SharedResources < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_uart0SharedResources
            {
                serial : shared_resources :: serial :: new(priority),
                host_producer : shared_resources :: host_producer ::
                new(priority),
            }
        }
    } #[allow(non_snake_case)] #[no_mangle] #[inline(never)]
    #[link_section = ".data.bar"] unsafe fn SPI0_IRQ()
    {
        const PRIORITY : u8 = 2u8 ; rtic :: export ::
        run(PRIORITY, ||
        {
            spi0(spi0 :: Context ::
            new(& rtic :: export :: Priority :: new(PRIORITY)))
        }) ;
    } impl < 'a > __rtic_internal_spi0LocalResources < 'a >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_spi0LocalResources
            {
                spi_tx_consumer : & mut *
                (& mut *
                __rtic_internal_local_resource_spi_tx_consumer.get_mut()).as_mut_ptr(),
            }
        }
    } impl < 'a > __rtic_internal_spi0SharedResources < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_spi0SharedResources
            {
                spi_dev : shared_resources :: spi_dev :: new(priority), serial
                : shared_resources :: serial :: new(priority), host_producer :
                shared_resources :: host_producer :: new(priority), freepin :
                shared_resources :: freepin :: new(priority),
            }
        }
    } #[allow(non_snake_case)] #[no_mangle] #[inline(never)]
    #[link_section = ".data.bar"] unsafe fn USBCTRL_IRQ()
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
                serial : shared_resources :: serial :: new(priority), usb_dev
                : shared_resources :: usb_dev :: new(priority), serial_buf :
                shared_resources :: serial_buf :: new(priority), freepin :
                shared_resources :: freepin :: new(priority), host_producer :
                shared_resources :: host_producer :: new(priority),
            }
        }
    } #[allow(non_snake_case)] #[no_mangle] unsafe fn PIO0_IRQ_0()
    {
        const PRIORITY : u8 = 3u8 ; rtic :: export ::
        run(PRIORITY, ||
        {
            pio_sm_rx(pio_sm_rx :: Context ::
            new(& rtic :: export :: Priority :: new(PRIORITY)))
        }) ;
    } impl < 'a > __rtic_internal_pio_sm_rxLocalResources < 'a >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_pio_sm_rxLocalResources
            {
                consumer : & mut *
                (& mut *
                __rtic_internal_local_resource_consumer.get_mut()).as_mut_ptr(),
            }
        }
    } impl < 'a > __rtic_internal_pio_sm_rxSharedResources < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_pio_sm_rxSharedResources
            {
                serial : shared_resources :: serial :: new(priority), pio0 :
                shared_resources :: pio0 :: new(priority), smi_rx :
                shared_resources :: smi_rx :: new(priority),
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_send_out_FQ : rtic :: RacyCell <
    rtic :: export :: SCFQ < 2 > > = rtic :: RacyCell ::
    new(rtic :: export :: Queue :: new()) ; #[link_section = ".uninit.rtic17"]
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_send_out_INPUTS : rtic :: RacyCell <
    [core :: mem :: MaybeUninit < () > ; 1] > = rtic :: RacyCell ::
    new([core :: mem :: MaybeUninit :: uninit(),]) ; impl < 'a >
    __rtic_internal_send_outLocalResources < 'a >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_send_outLocalResources
            {
                producer : & mut *
                (& mut *
                __rtic_internal_local_resource_producer.get_mut()).as_mut_ptr(),
                host_consumer : & mut *
                (& mut *
                __rtic_internal_local_resource_host_consumer.get_mut()).as_mut_ptr(),
            }
        }
    } impl < 'a > __rtic_internal_send_outSharedResources < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_send_outSharedResources
            {
                serial : shared_resources :: serial :: new(priority),
                smi_master : shared_resources :: smi_master :: new(priority),
                smi_tx : shared_resources :: smi_tx :: new(priority), smi_rx :
                shared_resources :: smi_rx :: new(priority), freepin :
                shared_resources :: freepin :: new(priority),
            }
        }
    } #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_respond_to_host_FQ : rtic ::
    RacyCell < rtic :: export :: SCFQ < 2 > > = rtic :: RacyCell ::
    new(rtic :: export :: Queue :: new()) ; #[link_section = ".uninit.rtic18"]
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_respond_to_host_INPUTS : rtic ::
    RacyCell <
    [core :: mem :: MaybeUninit < SlaveResponse < crate :: protocol :: slave
    :: Ready > > ; 1] > = rtic :: RacyCell ::
    new([core :: mem :: MaybeUninit :: uninit(),]) ; impl < 'a >
    __rtic_internal_respond_to_hostLocalResources < 'a >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_respond_to_hostLocalResources
            {
                spi_tx_producer : & mut *
                (& mut *
                __rtic_internal_local_resource_spi_tx_producer.get_mut()).as_mut_ptr(),
            }
        }
    } impl < 'a > __rtic_internal_respond_to_hostSharedResources < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(priority : & 'a rtic :: export :: Priority) -> Self
        {
            __rtic_internal_respond_to_hostSharedResources
            { serial : shared_resources :: serial :: new(priority), }
        }
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[derive(Clone, Copy)] #[doc(hidden)] pub enum P3_T
    { respond_to_host, send_out, } #[doc(hidden)]
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)] static
    __rtic_internal_P3_RQ : rtic :: RacyCell < rtic :: export :: SCRQ < P3_T,
    3 > > = rtic :: RacyCell :: new(rtic :: export :: Queue :: new()) ;
    #[allow(non_snake_case)]
    #[doc = "Interrupt handler to dispatch tasks at priority 3"] #[no_mangle]
    unsafe fn PWM_IRQ_WRAP()
    {
        #[doc = r" The priority of this interrupt handler"] const PRIORITY :
        u8 = 3u8 ; rtic :: export ::
        run(PRIORITY, ||
        {
            while let Some((task, index)) =
            (& mut * __rtic_internal_P3_RQ.get_mut()).split().1.dequeue()
            {
                match task
                {
                    P3_T :: respond_to_host =>
                    {
                        let _0 =
                        (& *
                        __rtic_internal_respond_to_host_INPUTS.get()).get_unchecked(usize
                        :: from(index)).as_ptr().read() ;
                        (& mut *
                        __rtic_internal_respond_to_host_FQ.get_mut()).split().0.enqueue_unchecked(index)
                        ; let priority = & rtic :: export :: Priority ::
                        new(PRIORITY) ;
                        respond_to_host(respond_to_host :: Context :: new(priority),
                        _0)
                    } P3_T :: send_out =>
                    {
                        let() =
                        (& *
                        __rtic_internal_send_out_INPUTS.get()).get_unchecked(usize
                        :: from(index)).as_ptr().read() ;
                        (& mut *
                        __rtic_internal_send_out_FQ.get_mut()).split().0.enqueue_unchecked(index)
                        ; let priority = & rtic :: export :: Priority ::
                        new(PRIORITY) ;
                        send_out(send_out :: Context :: new(priority))
                    }
                }
            }
        }) ;
    } #[doc(hidden)] mod rtic_ext
    {
        use super :: * ; #[no_mangle] unsafe extern "C" fn main() ->!
        {
            rtic :: export :: assert_send :: < SerialPort < 'static, hal ::
            usb :: UsbBus > > () ; rtic :: export :: assert_send :: <
            usb_device :: device :: UsbDevice < 'static, hal :: usb :: UsbBus
            > > () ; rtic :: export :: assert_send :: < hal :: pio :: PIO <
            pac :: PIO0 > > () ; rtic :: export :: assert_send :: < hal :: pio
            :: StateMachine < (pac :: PIO0, SM0), hal :: pio :: Running > > ()
            ; rtic :: export :: assert_send :: < hal :: pio :: Tx <
            (pac :: PIO0, SM0) > > () ; rtic :: export :: assert_send :: < hal
            :: pio :: Rx < (pac :: PIO0, SM0) > > () ; rtic :: export ::
            assert_send :: < [u8 ; 64] > () ; rtic :: export :: assert_send ::
            < Producer < 'static, HostRequest < Clean >, 3 > > () ; rtic ::
            export :: assert_send :: < Pin < Gpio25, hal :: gpio :: Output <
            hal :: gpio :: PushPull > > > () ; rtic :: export :: assert_send
            :: < hal :: Spi < hal :: spi :: Enabled, pac :: SPI0, 8 > > () ;
            rtic :: export :: assert_send :: < hal :: uart :: UartPeripheral <
            hal :: uart :: Enabled, pac :: UART0, (UartTx, UartRx) > > () ;
            rtic :: export :: assert_send :: < Producer < 'static, [u8 ; 18],
            3 > > () ; rtic :: export :: assert_send :: < Consumer < 'static,
            [u8 ; 18], 3 > > () ; rtic :: export :: assert_send :: < Consumer
            < 'static, HostRequest < Clean >, 3 > > () ; rtic :: export ::
            assert_send :: < Producer < 'static, SlaveResponse < NotReady >, 3
            > > () ; rtic :: export :: assert_send :: < Consumer < 'static,
            SlaveResponse < NotReady >, 3 > > () ; rtic :: export ::
            assert_send :: < SlaveResponse < crate :: protocol :: slave ::
            Ready > > () ; rtic :: export :: interrupt :: disable() ;
            (0 ..
            1u8).for_each(| i |
            (& mut *
            __rtic_internal_send_out_FQ.get_mut()).enqueue_unchecked(i)) ;
            (0 ..
            1u8).for_each(| i |
            (& mut *
            __rtic_internal_respond_to_host_FQ.get_mut()).enqueue_unchecked(i))
            ; let mut core : rtic :: export :: Peripherals = rtic :: export ::
            Peripherals :: steal().into() ; let _ =
            you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml ::
            interrupt :: PWM_IRQ_WRAP ; let _ =
            you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml ::
            interrupt :: SIO_IRQ_PROC0 ; let _ =
            you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml ::
            interrupt :: SIO_IRQ_PROC1 ; let _ =
            you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml ::
            interrupt :: UART1_IRQ ; let _ =
            [() ; ((1 << rp_pico :: pac :: NVIC_PRIO_BITS) - 3u8 as usize)] ;
            core.NVIC.set_priority(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: PWM_IRQ_WRAP, rtic :: export ::
            logical2hw(3u8, rp_pico :: pac :: NVIC_PRIO_BITS),) ; rtic ::
            export :: NVIC ::
            unmask(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: PWM_IRQ_WRAP) ; let _ =
            [() ; ((1 << rp_pico :: pac :: NVIC_PRIO_BITS) - 3u8 as usize)] ;
            core.NVIC.set_priority(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: UART0_IRQ, rtic :: export ::
            logical2hw(3u8, rp_pico :: pac :: NVIC_PRIO_BITS),) ; rtic ::
            export :: NVIC ::
            unmask(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: UART0_IRQ) ; let _ =
            [() ; ((1 << rp_pico :: pac :: NVIC_PRIO_BITS) - 2u8 as usize)] ;
            core.NVIC.set_priority(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: SPI0_IRQ, rtic :: export ::
            logical2hw(2u8, rp_pico :: pac :: NVIC_PRIO_BITS),) ; rtic ::
            export :: NVIC ::
            unmask(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: SPI0_IRQ) ; let _ =
            [() ; ((1 << rp_pico :: pac :: NVIC_PRIO_BITS) - 3u8 as usize)] ;
            core.NVIC.set_priority(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: USBCTRL_IRQ, rtic :: export ::
            logical2hw(3u8, rp_pico :: pac :: NVIC_PRIO_BITS),) ; rtic ::
            export :: NVIC ::
            unmask(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: USBCTRL_IRQ) ; let _ =
            [() ; ((1 << rp_pico :: pac :: NVIC_PRIO_BITS) - 3u8 as usize)] ;
            core.NVIC.set_priority(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: PIO0_IRQ_0, rtic :: export ::
            logical2hw(3u8, rp_pico :: pac :: NVIC_PRIO_BITS),) ; rtic ::
            export :: NVIC ::
            unmask(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: PIO0_IRQ_0) ; #[inline(never)] fn
            __rtic_init_resources < F > (f : F) where F : FnOnce() { f() ; }
            __rtic_init_resources(||
            {
                let(shared_resources, local_resources, mut monotonics) =
                init(init :: Context :: new(core.into())) ;
                __rtic_internal_shared_resource_serial.get_mut().write(core ::
                mem :: MaybeUninit :: new(shared_resources.serial)) ;
                __rtic_internal_shared_resource_usb_dev.get_mut().write(core
                :: mem :: MaybeUninit :: new(shared_resources.usb_dev)) ;
                __rtic_internal_shared_resource_pio0.get_mut().write(core ::
                mem :: MaybeUninit :: new(shared_resources.pio0)) ;
                __rtic_internal_shared_resource_smi_master.get_mut().write(core
                :: mem :: MaybeUninit :: new(shared_resources.smi_master)) ;
                __rtic_internal_shared_resource_smi_tx.get_mut().write(core ::
                mem :: MaybeUninit :: new(shared_resources.smi_tx)) ;
                __rtic_internal_shared_resource_smi_rx.get_mut().write(core ::
                mem :: MaybeUninit :: new(shared_resources.smi_rx)) ;
                __rtic_internal_shared_resource_serial_buf.get_mut().write(core
                :: mem :: MaybeUninit :: new(shared_resources.serial_buf)) ;
                __rtic_internal_shared_resource_host_producer.get_mut().write(core
                :: mem :: MaybeUninit :: new(shared_resources.host_producer))
                ;
                __rtic_internal_shared_resource_freepin.get_mut().write(core
                :: mem :: MaybeUninit :: new(shared_resources.freepin)) ;
                __rtic_internal_shared_resource_spi_dev.get_mut().write(core
                :: mem :: MaybeUninit :: new(shared_resources.spi_dev)) ;
                __rtic_internal_local_resource_uart_dev.get_mut().write(core
                :: mem :: MaybeUninit :: new(local_resources.uart_dev)) ;
                __rtic_internal_local_resource_spi_tx_producer.get_mut().write(core
                :: mem :: MaybeUninit :: new(local_resources.spi_tx_producer))
                ;
                __rtic_internal_local_resource_spi_tx_consumer.get_mut().write(core
                :: mem :: MaybeUninit :: new(local_resources.spi_tx_consumer))
                ;
                __rtic_internal_local_resource_host_consumer.get_mut().write(core
                :: mem :: MaybeUninit :: new(local_resources.host_consumer)) ;
                __rtic_internal_local_resource_producer.get_mut().write(core
                :: mem :: MaybeUninit :: new(local_resources.producer)) ;
                __rtic_internal_local_resource_consumer.get_mut().write(core
                :: mem :: MaybeUninit :: new(local_resources.consumer)) ; rtic
                :: export :: interrupt :: enable() ;
            }) ;
            idle(idle :: Context ::
            new(& rtic :: export :: Priority :: new(0)))
        }
    }
}