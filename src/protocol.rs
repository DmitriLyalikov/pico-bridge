// Check if this must implement send and sync
    use core::result::Result;
    use self::slave::{SlaveResponse, NotReady, HostErr};

    pub trait Send{
        fn exchange_for_slave_response(&mut self) -> Result<SlaveResponse<NotReady>, &'static str> {
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
    
    #[derive(Copy, Clone, PartialEq, Debug)]
    pub enum ValidHostInterfaces {
        Serial = 0b00,
        UART = 0b01,
        SPI = 0b10,
        None = 0b11,
    }

pub mod host {
    use super::{combine_u16_to_u32, combine_u8_to_u32, reverse_first_16_bit, encode_smi_read};
    use core::{marker::PhantomData};
    use core::convert::TryFrom;
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
        None,
        Read,
        Write, 
        Set,
        Get,
    }

    impl TryFrom<u16> for ValidOps {
        type Error = ();
    
        fn try_from(num: u16) -> Result<Self, Self::Error> {
            match num {
                0 => Ok(ValidOps::None),
                1 => Ok(ValidOps::Read),
                2 => Ok(ValidOps::Write),
                3 => Ok(ValidOps::Set),
                4 =>  Ok(ValidOps::Get),
                // ... add more variants here
                _ => Err(()),
            }
        }
    }

    pub enum ValidInterfaces  {
        None,
        SMI,
        JTAG, 
        I2C,
        SPI,
        Config,
        GPIO,
    }

    impl TryFrom<u16> for ValidInterfaces {
        type Error = ();
    
        fn try_from(num: u16) -> Result<Self, Self::Error> {
            match num {
                0 => Ok(ValidInterfaces::None),
                1 => Ok(ValidInterfaces::SMI),
                2 => Ok(ValidInterfaces::JTAG),
                3 => Ok(ValidInterfaces::I2C),
                4 => Ok(ValidInterfaces::SPI),
                5 => Ok(ValidInterfaces::Config),
                6 => Ok(ValidInterfaces::GPIO),
                // ... add more variants here
                _ => Err(()),
            }
        }
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
        fn exchange_for_slave_response(&mut self) -> Result<super::slave::SlaveResponse<super::slave::NotReady>, &'static str> {
            let mut sr = SlaveResponse::new();
            sr.set_host_config( self.host_config);
            sr.set_proc_id(self.proc_id);
            Ok(sr)
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

        pub fn build_from_16bit_spi(mut self, buf: &[u16]) -> Result<HostRequest<Clean>, &'static str> {
            // Interface first 3 bits of Packet 1
            let interface = ((buf[0] >> 13) & 0b111) as u16;
            match ValidInterfaces::try_from(interface) {
                Ok(interface) => {
                    self.set_interface(interface);
                }
                _ => {
                    return Err("Invalid Interface");
                }
            }
            // Operation next 3 bits of Packet 1
            let operation = ((buf[0] >> 10) & 0b111) as u16;
            match ValidOps::try_from(operation) {
                Ok(op) => {
                    self.set_operation(op);
                }
                _ => {
                    return Err("Invalid Operation");
                }
            }
    
            // Default 1
            let size = ((buf[0] >> 8) & 0b11) as u8 + 1;
            let checksum = (buf[0] & 0xFF) as u8;
            let payload = combine_u16_to_u32(&buf[1..]);

            self.set_size(size);
            self.set_host_config(ValidHostInterfaces::SPI);
            self.set_payload(payload);
            self.set_checksum(checksum);

            return self.init_clean()
        }

        pub fn build_from_8bit_spi(mut self, buf: &[u8]) -> Result<HostRequest<Clean>, &'static str> {
            // Interface first 3 bits of Packet 1
            let interface = ((buf[0] >> 5) & 0b111) as u16;
            match ValidInterfaces::try_from(interface) {
                Ok(interface) => {
                    self.set_interface(interface);
                }
                _ => {
                    return Err("Invalid Interface");
                }
            }
            // Operation next 3 bits of Packet 1
            let operation = ((buf[0] >> 2) & 0b111) as u16;
            match ValidOps::try_from(operation) {
                Ok(op) => {
                    self.set_operation(op);
                }
                _ => {
                    return Err("Invalid Operation");
                }
            }
    
            // Default 1
            let size = ((buf[0]) & 0b11) + 1;
            let checksum = buf[1];
            let payload = combine_u8_to_u32(&buf[2..]);

            self.set_size(size);
            self.set_host_config(ValidHostInterfaces::SPI);
            self.set_payload(payload);
            self.set_checksum(checksum);

            return self.init_clean()
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
                        // Opcode    PhyAddr               RegAddr
                        self.payload[0] = encode_smi_read(self.payload[0] as u8, self.payload[1] as u8);
                        self.size = 1;
                    }
                    // If it is SMI Write, we need PHY address and REG address + Data
                    else if self.operation == ValidOps::Write && self.size != 3 {
                        if self.size != 3 {return Err("Invalid Arguments for SMI: Write\n\r")}
                        self.payload[0] = 1 | self.payload[0] << 2 | self.payload[1] << 7 | self.payload[2] << 15;
                        // self.payload[1] = self.payload[2];
                        self.size = 1;
                    }
                }
                ValidInterfaces::Config => {
                }

                ValidInterfaces::GPIO => {
                    // So far, only support output High and Low
                    if self.size != 1 { return Err("Invalid Arguments") }
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

pub mod slave {
    use core::{marker::PhantomData};
    use super::{Respond, ValidHostInterfaces};

        // State of the request
    pub trait State {}
    // The response is ready to go back to the host
    pub struct Ready {
        __private: (),
    }
    #[derive(Debug)]
    // The response is not ready to send back to host
    pub struct NotReady {
        __private: (),
    }

    impl State for NotReady {}
    impl State for Ready {}

    #[derive(Debug)]
    pub struct SlaveResponse<S: State> {
        state: PhantomData<S>,
        pub proc_id: u8,
        pub host_config: ValidHostInterfaces,
        pub size: u8,             // A value between 0 and 4
        pub payload: u32,     // Max payload size over SPI is 4 bytes 
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
                payload: 0,
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

        pub fn set_payload(&mut self, payload: u32) {
            self.payload = payload;
        }
    
        pub fn init_ready(self) -> Result<SlaveResponse<Ready>, &'static str> {
            // if let valid_packet = checksum(self.checksum) ...
            // This will validate all the interface rules for our HostRequest, 
            // Any other kind of packet sanitizing
            self.transition(Ready {__private: () })
        }
    }

    pub enum HostErr {
        Timeout,
        None,
    }
}

// Helper function that converts a list of u16 words into a payload of 4 32 bit words
fn combine_u16_to_u32(values: &[u16]) -> [u32; 4] {
    let mut result = [0u32; 4];

    for (i, chunk) in values.chunks(2).enumerate() {
        let combined = ((chunk[1] as u32) << 16) | chunk[0] as u32;
        result[i] = combined;
    }

    result
}

fn combine_u8_to_u32(values: &[u8]) -> [u32; 4] {
    let mut result = [0u32; 4];

    for (i, chunk) in values.chunks(4).enumerate() {
        let combined = ((chunk[3] as u32) << 24) | ((chunk[2] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[0] as u32);
        result[i] = combined;
    }

    result
}

fn reverse_first_16_bit(num: u32) -> u32 {
    let mut result =  num;
    result = ((result & 0xFFFF0000) >> 16) | ((result & 0x0000FFFF) << 16);
    result = ((result & 0xFF00FF00) >> 8) | ((result & 0x00FF00FF) << 8);
    result = ((result & 0xF0F0F0F0) >> 4) | ((result & 0x0F0F0F0F) << 4);
    result = ((result & 0xCCCCCCCC) >> 2) | ((result & 0x33333333) << 2);
    result = ((result & 0xAAAAAAAA) >> 1) | ((result & 0x55555555) << 1);
    result
}


fn encode_smi_read(phy_addr: u8, reg_addr: u8) -> u32 {
    let mut packet: u32 = 0;

    // Set the op code (bits 0-1)
    packet |= (2 as u32) & 0b11;

    // Set the PHY address (bits 2-6)
    packet |= (((reverse_bits(phy_addr)>> 3) as u32) & 0b11111) << 2;

    // Set the register address (bits 7-11)
    packet |= (((reverse_bits(reg_addr) >> 3) as u32) & 0b11111) << 7;

    packet
}

fn reverse_bits(value: u8) -> u8 {
    let mut result = 0;
    for i in 0..8 {
        result |= ((value >> i) & 1) << (7 - i);
    }
    result
}