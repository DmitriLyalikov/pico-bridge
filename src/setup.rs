

pub struct Counter {
    counter: u32,
    enable: bool,
}

impl Counter {
    pub fn new() -> Self {
        Counter {
            counter: 0_u32,
            enable: true,
        }
    }

    pub fn get(&self) -> u32 {
        self.counter
    }

    pub fn reset(&mut self) {
        self.counter = 0_u32;
    }

    pub fn increment(&mut self) {
        self.counter += 1;
    }

    pub fn enable(&mut self, state: bool) {
        self.enable = state;
    }
}

