
pub mod transactions {
    pub struct Request {
        counter: u32,
        enable: bool,
    }
    
    impl Request {
        fn new() -> Self {
            Request {
                counter: 0_u32,
                enable: true,
            }
        }
    
        fn get(&self) -> u32 {
            self.counter
        }
    
        fn reset(&mut self) {
            self.counter = 0_u32;
        }
    
        fn increment(&mut self) {
            self.counter += 1;
        }
    
        fn enable(&mut self, state: bool) {
            self.enable = state;
        }
    }

pub mod PioDriver {

    // Programmable IO State Machine Trait
    pub trait PioDriver {
        // Initialize and configure the state machine and its pins/clocks
        fn init(&self);
        fn write(&self, data: u32);
        fn read(&self) -> u32;
    }


}
