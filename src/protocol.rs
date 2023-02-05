// Check if this must implement send and sync
    use core::{marker::PhantomData};
    use core::result::Result;
    use core::{mem, slice};
    use self::Slave::{SlaveResponse, NotReady, SlaveErr, HostErr};

    pub trait Send{
        fn send_out(&mut self) -> Result<SlaveResponse<NotReady>, &'static str> {
            // Match on the device facing interface and send payload to its TX FIFO
            // Return the constructed SlaveResponse
            Ok (SlaveResponse::new())
        }
    }

    pub trait Respond {
        fn respond_to_host(&self) -> HostErr {
            // Match on host_interface and send payload back on that channel
            // This needs to be done on a task that has access to all host facing interfaces
            HostErr::None
        }
    }

    #[derive(Copy, Clone)]
    pub enum ValidHostInterfaces {
        Serial = 0b00,
        UART = 0b01,
        SPI = 0b10,
        None = 0b11,
    }

pub mod Host {
    use core::fmt::Write;
    use core::{marker::PhantomData};
    use super::Send;
    use super::{SlaveResponse, ValidHostInterfaces};

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

    #[derive(PartialEq)]
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
        pub  interface: ValidInterfaces,
        host_config: ValidHostInterfaces,
        operation: ValidOps,
        checksum: u8,         // Wrapping checksum
        pub size: u8,             // A value between 0 and 4
        pub payload: [u32; 4],     // Max payload size over SPI is 4 bytes 

    }

    impl <S: State> HostRequest<S>{
        fn transition<To: State>(self, _: To) -> Result<HostRequest<To>, &'static str> {
           Ok(HostRequest {
                state: PhantomData,
                proc_id: self.proc_id,
                interface: self.interface,
                host_config: self.host_config,
                operation: self.operation,
                checksum: self.checksum,
                size: self.size,       
                payload: self.payload,
            })
        }
    }
    
    impl Send for HostRequest<Clean> {
        fn send_out(&mut self) -> Result<super::Slave::SlaveResponse<super::Slave::NotReady>, &'static str> {
            let mut SR = SlaveResponse::new();
            SR.set_host_config( self.host_config);
            SR.set_proc_id(self.proc_id);
            Ok(SR)
        }
    }
    impl HostRequest<Unclean> {
        pub fn new() -> HostRequest<Unclean> {
            HostRequest {
                state: PhantomData,
                proc_id: 0_u8,
                interface: ValidInterfaces::None,
                host_config: ValidHostInterfaces::None,
                operation: ValidOps::None,
                checksum: 0_u8,
                size: 0_u8,           
                payload: [0_u32; 4],
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

        pub fn set_host_config(&mut self, cfg: ValidHostInterfaces) {
            self.host_config = cfg
        }

        pub fn set_payload(&mut self, payload: [u32; 4]) {
            self.payload =  payload;
        }

        pub fn set_checksum(&mut self, checksum: u8) {
            self.checksum =  checksum;
        }

        pub fn set_interface(&mut self, interface: ValidInterfaces) {
            self.interface = interface;
        }

        // This will validate all the interface rules for our HostRequest
        pub fn init_clean(mut self) -> Result<HostRequest<Clean>, &'static str> {
            // if let valid_packet = checksum(self.checksum) ...
            // Any other kind of packet sanitizing

            match self.interface {
                ValidInterfaces::SMI => {
                    // If it is SMI Read, we need PHY address and REG address
                    if self.operation == ValidOps::Read {
                        if self.size != 2 {return Err("Invalid Arguments for SMI: Read\n\r")}
                        // Opcode    PhyAddr               RedAddr
                        self.payload[0] = 2 | self.payload[0] << 2 | self.payload[1] << 7;
                        self.size = 1;
                    }
                    // If it is SMI Write, we need PHY address and REG address + Data
                    else if self.operation == ValidOps::Write && self.size != 3 {
                        if self.size != 3 {return Err("Invalid Arguments for SMI: Write\n\r")}
                        self.payload[0] = 1 | self.payload[0] << 2 | self.payload[1] << 7;
                        self.payload[1] = self.payload[2];
                        self.size = 2;
                    }
                }
                ValidInterfaces::Config => {
                }

                ValidInterfaces::None => {
                    return Err("No Interface Selected\n\r")
                }
                _ => {

                }
            }
            self.transition(Clean {__private: () })
        } 
    }
}

pub mod Slave {
    use core::{marker::PhantomData};
    use super::{Respond, ValidHostInterfaces};

        // State of the request
    pub trait State {}
    // The response is ready to go back to the host
    pub struct Ready {
        __private: (),
    }
    // The response is not ready to send back to host
    pub struct NotReady {
        __private: (),
    }

    impl State for NotReady {}
    impl State for Ready {}

    pub struct SlaveResponse<S: State> {
        state: PhantomData<S>,
        proc_id: u8,
        host_config: ValidHostInterfaces,
        size: u8,             // A value between 0 and 4
        payload: [u32; 4],     // Max payload size over SPI is 4 bytes 
    }

    impl <S: State> SlaveResponse<S>{
        fn transition<To: State>(self, _: To) -> Result<SlaveResponse<To>, &'static str> {
            Ok(SlaveResponse {
                state: PhantomData,
                proc_id: self.proc_id,
                host_config: self.host_config,
                size: self.size,       
                payload: self.payload,
            })
        }
    }

    impl Respond for SlaveResponse<Ready> {
        fn respond_to_host(&self) -> HostErr {
            match self.host_config {
                ValidHostInterfaces::Serial => {
                    // Serial_Respond_Task.spawn(size, payload)
                    HostErr::None
                }
                ValidHostInterfaces::UART => {
                    // UART_Respond_Task.spawn(size, payload)
                    HostErr::None
                }
                ValidHostInterfaces::SPI => {
                    // SPI_Respond_Task.spawn(size, payload)
                    HostErr::None
                }
                _ => {
                    // Should never happen
                    HostErr::Timeout
                }
            }
        }
    }

    impl SlaveResponse<NotReady> {
        pub fn new() -> SlaveResponse<NotReady> {
            SlaveResponse { 
                state: PhantomData,
                proc_id: 0_u8,
                host_config: ValidHostInterfaces::None,
                size: 0_u8,       
                payload: [0_u32; 4],
            }
        }

        pub fn set_proc_id(&mut self, proc_id: u8) {
            self.proc_id = proc_id;
        }

        pub fn set_host_config(&mut self, interface: ValidHostInterfaces) {
            self.host_config = interface;
        }

        pub fn set_size(&mut self, size: u8) {
            self.size = size;
        }

        pub fn set_payload(&mut self, payload: [u32; 4]) {
            self.payload = payload;
        }
    
        pub fn init_ready(mut self) -> Result<SlaveResponse<Ready>, &'static str> {
            // if let valid_packet = checksum(self.checksum) ...
            // This will validate all the interface rules for our HostRequest, 
            // Any other kind of packet sanitizing
            self.transition(Ready {__private: () })
        }
    }
    pub enum SlaveErr {
        Timeout,
        None,
    }

    pub enum HostErr {
        Timeout,
        Overflow,
        None,
    }
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

