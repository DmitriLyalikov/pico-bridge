// Check if this must implement send and sync
    use core::{marker::PhantomData};
    // State of the request
    pub trait State {}
    // request has not been validated
    pub struct Unclean {
        __private: (),
    }
    // The request has been validated
    pub struct Clean {
        __private: (),
    }

    impl State for Unclean {}
    impl State for Clean {}

    pub trait PIODriver {
        
    }
    pub enum ValidOps  {
        None,
        Read,
        Write, 
        SetClk,
        GetClk,
    }

    pub enum ValidInterfaces  {
        None,
        SMI,
        JTAG, 
        I2C,
        SPI,
        Config,
    }

    pub struct HostRequest<S: State> {
        state: PhantomData<S>,
        proc_id: u8,
        interface: ValidInterfaces,
        operation: ValidOps,
        size: u8,             // A value between 0 and 4
        payload: [u8; 4],     // Max payload size over SPI is 4 bytes 

    }

    impl <S: State> HostRequest<S>{
        fn transition<To: State>(self, _: To) -> HostRequest<To> {
            HostRequest {
                state: PhantomData,
                proc_id: self.proc_id,
                interface: self.interface,
                operation: self.operation,
                size: self.size,       
                payload: self.payload,
            }
        }

    }


    
    impl HostRequest<Unclean> {
        pub fn new() -> HostRequest<Unclean> {
            HostRequest {
                state: PhantomData,
                proc_id: 0_u8,
                interface: ValidInterfaces::None,
                operation: ValidOps::None,
                size: 0_u8,           
                payload: [0_u8; 4],
            }
        }
        pub fn set_proc_id(&mut self, proc_id: u8) {
            self.proc_id =  proc_id;
        }

        pub fn set_operation(&mut self, op: ValidOps) {
            self.operation =  op;
        }

        pub fn set_size(&mut self, size: u8) {
            if size > 4 {
                // Assert or log that data size too large
                self.size = 4;
            }
            self.size = size
        }

        pub fn set_payload(&mut self, payload: [u8; 4]) {
            self.payload =  payload;
        }

        pub fn set_interface(&mut self, interface: ValidInterfaces) {
            self.interface = interface;
        }

        pub fn init_clean(mut self) -> HostRequest<Clean> {
            // if let valid_packet = check_crc(self.crc) ...
            // Any other kind of packet sanitizing
            self.transition(Clean {__private: () })
        } 
    }

