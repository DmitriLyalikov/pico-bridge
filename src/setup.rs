use rp_pico::hal as hal;
// USB Device support 
use usb_device::{class_prelude::*, prelude::*};
// USB Communications Class Device support
use usbd_serial::SerialPort;

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
// Helper function to ensure all data is written across the serial interface
pub fn write_serial(serial: &mut SerialPort<'static, hal::usb::UsbBus>, buf: &str, block: bool) {
    let write_ptr = buf.as_bytes();

    // Because the buffer is of constant size and initialized to zero (0) we 
    //  add a test to determine the size that's really occupied by the str that we
    // want to send. From index zero to first byte that is as the zero byte value
    let mut index = 0;
    while index < write_ptr.len() && write_ptr[index] != 0 {
        index += 1;
    }
    let mut write_ptr = &write_ptr[0..index];

    while !write_ptr.is_empty() {
        match serial.write(write_ptr) {
            Ok(len) => write_ptr = &write_ptr[len..],
            // Meaning the USB write buffer is full
            Err(UsbError::WouldBlock) => {
                if !block {
                    break;
                }
            }
            // On error, just drop unwritten data
            Err(_) => break,
        }
    }
    let _ = serial.flush();
}

// Match the Serial Input commands to a hardware/software request
pub fn match_usb_serial_buf(
    buf: &[u8; 64],
    // Add any accessed 'static peripherals (PIO, SPI, etc) that will be controlled by host
    serial: &mut SerialPort<'static, hal::usb::UsbBus>,
    counter: &mut Counter,
) {
    let _buf_len = buf.len();
    //write_serial((serial), buf[0:], false);
    match buf[0..1] {
        // Print Menu
        [b'S', b'm'] | [b'S', b'm'] => {

            write_serial(serial, "M - Print Menu\n\r", false);
            print_menu(serial);
        }
        /* 
        // 0 - Reset counter
        b'0' => {
        write_serial(serial, "M - Print Menu\n\r", false);
            counter.reset();
        }
        // 1 - Increment counter
        b'1' => {
            write_serial(serial, "1 - Increment counter\n\r", false);
            counter.increment();
        }
        // 2 - Start continues counter
        b'2' => {
            write_serial(serial, "2 - Start continues counter\n\r", false);
            counter.enable(true);
        }
        // 3 - Stop continues counter
        b'3' => {
            write_serial(serial, "3 - Stop continues counter\n\r", false);
            counter.enable(false);
        } */
        _ => {
            write_serial(
                serial,
                unsafe { core::str::from_utf8_unchecked(buf) },
                false,
            );
            write_serial(serial, "Invalid option!\n\r", false);
        }
    }
}

pub fn print_menu(serial: &mut SerialPort<'static, hal::usb::UsbBus>){
    let mut _buf = [0u8; 273];
    // Create the Menu.
    let menu_str = "***************** \n\r
*  RP2040 RPC \n\r
*  Menu:\n\r
* \n\r
*  M / m - Print menu \n\r
*    0   - smi r phyAddr RegAddr \n\r
*    1   - smi w phyAddr RegAddr Data \n\r
*    2   - smi reset \n\r
*    3   - smi setclk frequency \n\r
***************** \n\r
Enter option: ";

    write_serial(serial, menu_str, true);
}


