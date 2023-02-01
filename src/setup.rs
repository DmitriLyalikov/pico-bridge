use rp_pico::hal as hal;
// USB Device support 
use usb_device::{class_prelude::*, prelude::*};
// USB Communications Class Device support
use usbd_serial::SerialPort;

use core::str;

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
) {
    let buf =  str::from_utf8(buf).unwrap();
    write_serial(serial, "\n\r", false);
    //write_serial(serial, buf, false);
    if slice_contains(buf, "smi") {
        write_serial(serial, "success\n\r", false);
    }

    if slice_contains(buf, "menu") {
        // write_serial(serial, "success\n\r", false);
        print_menu(serial);
    }
    else {
        write_serial(serial, "Invalid Command! \n\r", false);
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

pub fn slice_contains(haystack: &str, needle: &str) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }

    for i in 0..=(haystack.len() - needle.len()) {
        if &haystack[i..(i + needle.len())] == needle {
            return true;
        }
    }

    false
}




