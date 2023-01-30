pub mod protocol_spi {

    pub trait PIODriver {
        
    }
    pub enum Interface {
        None,
        SPI,
        SMI,
        JTAG,
        I2C,
        Config       // Not an interface, specifies a system config request
    }
    pub enum ValidOps  {
        None,
        Read,
        Write, 
        SetClk,
        GetClk,
    }
    pub struct Request {
        proc_id: u8,
        transaction: Interface,
        operation: ValidOps,
        size: u8,             // A value between 0 and 4
        payload: [u8; 4],     // Max payload size over SPI is 4 bytes 
    }

    impl Request {
        pub fn new() -> Self {
            Request {
                proc_id: 0_u8,
                transaction: Interface::None,
                operation: ValidOps::None,
                size: 0_u8,           
                payload: [0_u8; 4],
            }
        }
        pub fn set_proc_id(&mut self, proc_id: u8) {
            self.proc_id =  proc_id;
        }

        pub fn set_transaction(&mut self, transaction: Interface) {
            self.transaction =  transaction;
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
    }
}
