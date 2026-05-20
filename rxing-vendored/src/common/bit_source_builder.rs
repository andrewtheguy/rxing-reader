/*
 * Copyright 2008 ZXing authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::io::Write;

/**
 * Class that lets one easily build an array of bytes by appending bits at a time.
 *
 * @author Sean Owen
 */
pub struct BitSourceBuilder {
    output: Vec<u8>,
    next_byte: u32,
    bits_left_in_next_byte: u32,
}

impl BitSourceBuilder {
    pub const fn new() -> Self {
        Self {
            output: Vec::new(),
            next_byte: 0,
            bits_left_in_next_byte: 8,
        }
    }

    pub fn write(&mut self, value: u32, num_bits: u32) {
        if num_bits <= self.bits_left_in_next_byte {
            self.next_byte <<= num_bits;
            self.next_byte |= value;
            self.bits_left_in_next_byte -= num_bits;
            if self.bits_left_in_next_byte == 0 {
                self.output.push(self.next_byte as u8);
                self.next_byte = 0;
                self.bits_left_in_next_byte = 8;
            }
        } else {
            let bits_to_write_now = self.bits_left_in_next_byte;
            let num_rest_of_bits = num_bits - bits_to_write_now;
            let mask = 0xFF >> (8 - bits_to_write_now);
            let value_to_write_now = (value >> num_rest_of_bits) & mask;
            self.write(value_to_write_now, bits_to_write_now);
            self.write(value, num_rest_of_bits);
        }
    }

    pub fn as_byte_array(&mut self) -> &Vec<u8> {
        if self.bits_left_in_next_byte < 8 {
            self.write(0, self.bits_left_in_next_byte);
        }
        &self.output
    }

    pub fn to_byte_array(mut self) -> Vec<u8> {
        if self.bits_left_in_next_byte < 8 {
            self.write(0, self.bits_left_in_next_byte);
        }
        self.output
    }
}

impl Default for BitSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for BitSourceBuilder {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut written = 0;
        for byte in buf.iter() {
            self.write(*byte as u32, 8);
            written += 1;
        }

        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
