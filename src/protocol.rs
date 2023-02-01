// Check if this must implement send and sync
    use core::{marker::PhantomData};
    use core::{mem, slice};
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

    pub enum ValidOps  {
        None = 0b000,
        Read = 0b001,
        Write = 0b010, 
        SetClk = 0b011,
        GetClk = 0b100,
    }

    pub enum ValidInterfaces  {
        None = 0b000,
        SMI = 0b001,
        JTAG = 0b010, 
        I2C = 0b011,
        SPI = 0b100,
        Config = 0b101,
    }

    pub struct HostRequest<S: State> {
        state: PhantomData<S>,
        proc_id: u8,
        interface: ValidInterfaces,
        operation: ValidOps,
        checksum: u8,         // Wrapping checksum
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
                checksum: self.checksum,
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
                checksum: 0_u8,
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

        pub fn set_checksum(&mut self, checksum: u8) {
            self.checksum =  checksum;
        }

        pub fn set_interface(&mut self, interface: ValidInterfaces) {
            self.interface = interface;
        }

        pub fn init_clean(mut self) -> HostRequest<Clean> {
            // if let valid_packet = checksum(self.checksum) ...
            // Any other kind of packet sanitizing
            self.transition(Clean {__private: () })
        } 
    }

mod SlaveResponse {
    pub enum SlaveCode {
        NotReady,
        Ready,
        Sync,
    }
}

/// Sums all the bytes of a data structure
pub fn sum<T>(data: &T) -> u8 {
    let ptr = data as *const _ as *const u8;
    let len = mem::size_of::<T>();

    let data = unsafe { slice::from_raw_parts(ptr, len) };

    sum_slice(data)
}

/// Sums all the bytes in an array
pub fn sum_slice(data: &[u8]) -> u8 {
    data.iter().fold(0, |a, &b| a.wrapping_add(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_sum() {
        struct Simple(u32, u32);

        let simple = Simple(0xAA_00_BB_00, 0xAA_00_00_00);

        assert_eq!(sum(&simple), 15);
    }
}

