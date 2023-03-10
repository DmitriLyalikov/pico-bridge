// NOTE this file came from: 
//      https://github.com/Nashenas88/dactyl-manuform-kb2040-rs/blob/main/src/fmt.rs

//! Formatting Module for helping with logging over a serial connection.
//! This is usefule since there are no debugging pins exposed on the board

use core::fmt;

pub(crate) struct Wrapper<'a> {
    buf: &'a mut [u8],
    offset: usize,
}


impl<'a> fmt::Write for Wrapper<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();

        // Skip over already copied data.
        let remainder = &mut self.buf[self.offset..];
        // Check if there is space remaining (return error instead of panicing)
        if remainder.len() < bytes.len() {
            return Err(core::fmt::Error);
        }

        // Make the two slices the same length
        let remainder = &mut remainder[..bytes.len()];
        // Copy
        remainder.copy_from_slice(bytes);

        // Update offset to avoid overwriting
        self.offset += bytes.len();

        Ok(())
    }
}
