
pub mod protocol_spi {
    use core::{convert::Infallible, marker::PhantomData, ops::Deref};
    // State of the request
    pub trait State {}
    pub trait Interface {}
    // request has not been validated
    pub struct Unclean {
        __private: (),
    }

    // The request has been validated
    pub struct Clean {
        __private: (),
    }

    pub struct None {
        __private: (),
    }
    pub struct SMI {
        __private: (),
    }

    pub struct JTAG {
        __private: (),
    }
    pub struct Config {
        __private: (),
    }
    pub struct SPI {
        __private: (),
    }

    impl State for Unclean {}
    impl State for Clean {}

    impl Interface for None {}
    impl Interface for SMI {}
    impl Interface for JTAG {}
    impl Interface for Config {}
    impl Interface for SPI {}
    

    pub trait PIODriver {
        
    }
    pub enum ValidOps  {
        None,
        Read,
        Write, 
        SetClk,
        GetClk,
    }

    pub struct Request<S: State, I: Interface> {
        state: PhantomData<S>,
        interface: PhantomData<I>,
        proc_id: u8,
        operation: ValidOps,
        size: u8,             // A value between 0 and 4
        payload: [u8; 4],     // Max payload size over SPI is 4 bytes 
    }

    impl <S: State, I: Interface> Request<S, I>{
        fn transition<To: State>(self, _: To) -> Request<To, I> {
            Request {
                state: PhantomData,
                interface: PhantomData,
                proc_id: self.proc_id,
                operation: self.operation,
                size: self.size,       
                payload: self.payload,
            }
        }
    }

    impl <I: Interface> Request<Unclean, I> {
        pub fn new() -> Request<Unclean, I> {
            Request {
                state: PhantomData,
                interface: PhantomData,
                proc_id: 0_u8,
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

        pub fn init_clean(mut self) -> Request<Clean, I> {
            // if let valid_packet = check_crc(self.crc) ...
            // Any other kind of packet sanitizing
            self.transition(Clean {__private: () })
            
        }
    }


}
