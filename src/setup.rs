use crate::protocol::Host::{self, HostRequest, ValidInterfaces, ValidOps};

use rp_pico::hal as hal;
// USB Device support 
use usb_device::{class_prelude::*, prelude::*};
// USB Communications Class Device support
use usbd_serial::SerialPort;

use core::{str, result, u32};
use core::str::{SplitWhitespace};

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

    if slice_contains(buf, "menu") {
        // write_serial(serial, "success\n\r", false);
        print_menu(serial);
    }
    else {
        write_serial(serial, buf, false);
        write_serial(serial, "\n\r", false);
        message_parse_build(buf, serial);
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

// Helper function that takes list of bytes and deconstructs
// into HostRequest fields. 
// NOTE: Preliminary behavior is to drop message and log to serial an invalid message
// if fields are missing or invalid
pub fn message_parse_build<'input>(input: &'input str,
    serial: &mut SerialPort<'static, hal::usb::UsbBus>
    ) -> Result<HostRequest<Host::Unclean>, &'static str>{
    let mut payload = [0u32; 4];

    // Split up the given string
    let mut HR = HostRequest::new();
    let words = |input: &'input str| -> SplitWhitespace<'input>  {input.split_whitespace()};
    let mut command = words(input);
    let command_count = command.clone().count();
    if command_count > 6 {
        write_serial(serial, "Too many arguments!\n\r", false);
        return Err("Too many arguments")
    }
    // Match on the first word
    match command.next() {
        Some("smi" | "SMI") => {
            HR.set_interface(ValidInterfaces::SMI);
            write_serial(serial, "got smi\n\r", false);
        }
        _ => {
            write_serial(serial, "Invalid Interface\n\r", false);
            return Err("Invalid Interface")
        }
    }
    // Match on the second word. This should be an operation. If not log incorrect
    match command.next() {
        Some("r" | "R") => {
            HR.set_operation(ValidOps::Read);
            write_serial(serial, "got read\n\r", false);
        }
        Some("w" | "W") => {
            HR.set_operation(ValidOps::Write);
        }
        _ => {
            write_serial(serial, "Invalid Operation\n\r", false);  
            return Err("Invalid Operation");
        }
    }
    let mut size: u8 = 0;
    while size < (command_count - 3) as u8 {
        let val = command.nth(0).unwrap();
        // Match on the third word
            write_serial(serial, val, false);
            match bytes_to_number(val) {
                Ok(value) => {
                    payload[size as usize] = value;
                }
                Err(err) => {
                    return Err(err)
                }
        }
        size+=1;
    }
    HR.set_size(size);
    HR.set_payload(payload);
    Ok(HR)
}

// Helper function to take &str in decimal or hex form
// and return u32.
// ie: s = "0xFF"  will return decimal value 255
pub fn bytes_to_number(s: &str) -> Result<u32, &'static str> {
    let mut result: u32 = 0;
    // Check if the input is hex or decimal
    let mut chars = s.chars();
    if let Some(c) = chars.next() {
        if c != '0' || chars.next() != Some('x') {
            if '0' <= c && c <= '9' {
                result += c as u32 - '0' as u32;
                for c in chars {
                    let digit= match c {
                        '0'..='9' => c as u32 - '0' as u32,
                        _ => return Err("Invalid decimal character"),
                    };
                    if result >= 429_496_720 {
                        return Err("Integer number too large!")
                    }
                    result = result * 10 + digit;
                    
                }
                return Ok(result)
            }
            return Err("Not a hex or decimal string")
        }
    }
    if chars.clone().count() > 8 {
        return Err("Integer number too large!")
    }
    for c in chars {
        let digit =  match c {
            '0'..='9' => c as u32 - '0' as u32,
            'a'..='f' => c as u32 - 'a' as u32 + 10,
            'A'..='F' => c as u32 - 'A' as u32 + 10,
            _ => return Err("Invalid hex character"),
        };
        result = result * 16 + digit;
    }
    Ok(result)
    // It is decimal form
}

