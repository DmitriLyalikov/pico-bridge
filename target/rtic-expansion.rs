#[doc = r" The RTIC application module"] pub mod app
{
    #[doc =
    r" Always include the device crate which contains the vector table"] use
    rp2040_hal :: pac as
    you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml ; use
    embedded_hal :: blocking :: spi :: Transfer ; use rp2040_hal as hal ; use
    hal :: clocks :: Clock ; use hal :: uart ::
    { UartConfig, DataBits, StopBits } ; use hal :: gpio ::
    { pin :: bank0 :: *, Pin, FunctionUart } ; use hal :: pac as pac ; use
    fugit :: RateExtU32 ; #[doc = r" User code from within the module"] type
    UartTx = Pin < Gpio0, FunctionUart > ; type UartRx = Pin < Gpio1,
    FunctionUart > ;
    #[doc =
    " External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust"]
    #[doc = " if your board has a different frequency"] const XTAL_FREQ_HZ :
    u32 = 12_000_000u32 ; #[doc = r" User code end"] #[inline(always)]
    #[allow(non_snake_case)] fn init(c : init :: Context) ->
    (Shared, Local, init :: Monotonics)
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
        hal :: gpio :: FunctionSpi > () ; let _spi_cs = pins.gpio8.into_mode
        :: < hal :: gpio :: FunctionSpi > () ; let spi = hal :: Spi :: < _, _,
        16 > :: new(c.device.SPI0) ; let spi_dev =
        spi.init(& mut resets, clocks.peripheral_clock.freq(), 16.MHz(), &
        embedded_hal :: spi :: MODE_0, true,) ; let uart_pins =
        (pins.gpio0.into_mode :: < hal :: gpio :: FunctionUart > (),
        pins.gpio1.into_mode :: < hal :: gpio :: FunctionUart > (),) ; let mut
        uart = hal :: uart :: UartPeripheral ::
        new(c.device.UART0, uart_pins, & mut
        resets).enable(UartConfig ::
        new(9600.Hz(), DataBits :: Eight, None, StopBits :: One),
        clocks.peripheral_clock.freq(),).unwrap() ;
        (Shared {}, Local { spi_dev : spi_dev, uart_dev : uart, }, init ::
        Monotonics(),)
    } #[allow(non_snake_case)] fn idle(_cx : idle :: Context) ->!
    {
        use rtic :: Mutex as _ ; use rtic :: mutex :: prelude :: * ; loop
        { cortex_m :: asm :: nop() ; }
    } #[inline(never)] #[link_section = ".data.bar"] #[allow(non_snake_case)]
    fn spi0_irq(cx : spi0_irq :: Context)
    {
        use rtic :: Mutex as _ ; use rtic :: mutex :: prelude :: * ; let mut
        tx_buf = [1_u16, 2, 3, 4, 5, 6] ; let mut _rx_buf = [0_u16 ; 6] ; let
        _t = cx.local.spi_dev.transfer(& mut tx_buf) ;
    } struct Shared {} struct Local
    {
        spi_dev : rp2040_hal :: Spi < hal :: spi :: Enabled, pac :: SPI0, 16
        >, uart_dev : rp2040_hal :: uart :: UartPeripheral < hal :: uart ::
        Enabled, pac :: UART0, (UartTx, UartRx) >,
    } #[doc = r" Monotonics used by the system"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_Monotonics() ;
    #[doc = r" Execution context"] #[allow(non_snake_case)]
    #[allow(non_camel_case_types)] pub struct __rtic_internal_init_Context <
    'a >
    {
        #[doc = r" Core (Cortex-M) peripherals"] pub core : rtic :: export ::
        Peripherals, #[doc = r" Device peripherals"] pub device : rp2040_hal
        :: pac :: Peripherals, #[doc = r" Critical section token for init"]
        pub cs : rtic :: export :: CriticalSection < 'a >,
    } impl < 'a > __rtic_internal_init_Context < 'a >
    {
        #[inline(always)] pub unsafe fn
        new(core : rtic :: export :: Peripherals,) -> Self
        {
            __rtic_internal_init_Context
            {
                device : rp2040_hal :: pac :: Peripherals :: steal(), cs :
                rtic :: export :: CriticalSection :: new(), core,
            }
        }
    } #[allow(non_snake_case)] #[doc = "Initialization function"] pub mod init
    {
        pub use super :: __rtic_internal_Monotonics as Monotonics ; pub use
        super :: __rtic_internal_init_Context as Context ;
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
    } #[allow(non_snake_case)] #[allow(non_camel_case_types)]
    #[doc = "Local resources `spi0_irq` has access to"] pub struct
    __rtic_internal_spi0_irqLocalResources < 'a >
    {
        pub spi_dev : & 'a mut rp2040_hal :: Spi < hal :: spi :: Enabled, pac
        :: SPI0, 16 >,
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
    } #[doc = r" app module"] impl < > __rtic_internal_idleLocalResources < >
    {
        #[inline(always)] pub unsafe fn new() -> Self
        {
            __rtic_internal_idleLocalResources
            { x : & mut * __rtic_internal_local_idle_x.get_mut(), }
        }
    } #[doc(hidden)] #[allow(non_upper_case_globals)] const
    __rtic_internal_MASKS : [u32 ; 3] =
    [rtic :: export :: create_mask([]), rtic :: export :: create_mask([]),
    rtic :: export ::
    create_mask([rp2040_hal :: pac :: Interrupt :: SPI0_IRQ as u32])] ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic0"] static
    __rtic_internal_local_resource_spi_dev : rtic :: RacyCell < core :: mem ::
    MaybeUninit < rp2040_hal :: Spi < hal :: spi :: Enabled, pac :: SPI0, 16 >
    >> = rtic :: RacyCell :: new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] #[link_section = ".uninit.rtic1"] static
    __rtic_internal_local_resource_uart_dev : rtic :: RacyCell < core :: mem
    :: MaybeUninit < rp2040_hal :: uart :: UartPeripheral < hal :: uart ::
    Enabled, pac :: UART0, (UartTx, UartRx) > >> = rtic :: RacyCell ::
    new(core :: mem :: MaybeUninit :: uninit()) ;
    #[allow(non_camel_case_types)] #[allow(non_upper_case_globals)]
    #[doc(hidden)] static __rtic_internal_local_idle_x : rtic :: RacyCell <
    u32 > = rtic :: RacyCell :: new(0) ; #[allow(non_snake_case)] #[no_mangle]
    #[inline(never)] #[link_section = ".data.bar"] unsafe fn SPI0_IRQ()
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
    } #[doc(hidden)] mod rtic_ext
    {
        use super :: * ; #[no_mangle] unsafe extern "C" fn main() ->!
        {
            rtic :: export :: assert_send :: < rp2040_hal :: Spi < hal :: spi
            :: Enabled, pac :: SPI0, 16 > > () ; rtic :: export :: assert_send
            :: < rp2040_hal :: uart :: UartPeripheral < hal :: uart ::
            Enabled, pac :: UART0, (UartTx, UartRx) > > () ; const
            _CONST_CHECK : () =
            {
                if rtic :: export :: is_armv6()
                {
                    if(rp2040_hal :: pac :: Interrupt :: SPI0_IRQ as u32) > 31
                    {
                        :: core :: panic!
                        ("An interrupt above value 31 is used while in armv6") ;
                    }
                } else {}
            } ; let _ = _CONST_CHECK ; rtic :: export :: interrupt ::
            disable() ; let mut core : rtic :: export :: Peripherals = rtic ::
            export :: Peripherals :: steal().into() ; const _ : () =
            if(1 << rp2040_hal :: pac :: NVIC_PRIO_BITS) < 3u8 as usize
            {
                :: core :: panic!
                ("Maximum priority used by interrupt vector 'SPI0_IRQ' is more than supported by hardware")
                ;
            } ;
            core.NVIC.set_priority(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: SPI0_IRQ, rtic :: export ::
            logical2hw(3u8, rp2040_hal :: pac :: NVIC_PRIO_BITS),) ; rtic ::
            export :: NVIC ::
            unmask(you_must_enable_the_rt_feature_for_the_pac_in_your_cargo_toml
            :: interrupt :: SPI0_IRQ) ; #[inline(never)] fn
            __rtic_init_resources < F > (f : F) where F : FnOnce() { f() ; }
            __rtic_init_resources(||
            {
                let(shared_resources, local_resources, mut monotonics) =
                init(init :: Context :: new(core.into())) ;
                __rtic_internal_local_resource_spi_dev.get_mut().write(core ::
                mem :: MaybeUninit :: new(local_resources.spi_dev)) ; rtic ::
                export :: interrupt :: enable() ;
            }) ;
            idle(idle :: Context ::
            new(& rtic :: export :: Priority :: new(0)))
        }
    }
}