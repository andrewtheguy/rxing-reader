/*
 * Copyright 2007 ZXing authors
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

use std::cmp;
use std::io::{ErrorKind, Read};

use anyhow::{Result, bail};

use crate::Error;

/// This provides an easy abstraction to read bits at a time from a sequence of bytes, where the
/// number of bits read is not often a multiple of 8.
///
/// `BitSource` borrows immutable input bytes and stores read position, so use
/// one instance per independent bit stream.
pub struct BitSource<'a> {
    bytes: &'a [u8],
    byte_offset: usize,
    bit_offset: usize,
}

impl<'a> BitSource<'a> {
    /// - `bytes`: bytes from which this will read bits. Bits will be read from the first byte first.
    ///   Bits are read within a byte from most-significant to least-significant bit.
    pub const fn new(bytes: &'a [u8]) -> Self {
        BitSource {
            bytes,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// - `num_bits`: number of bits to read
    ///
    /// Returns the requested bits as the least-significant bits of a `u32`.
    ///
    /// Returns an invalid-argument error if `num_bits` is outside `1..=32` or
    /// more bits are requested than remain available.
    pub fn read_bits(&mut self, num_bits: usize) -> Result<u32> {
        let available_bits = self.available();
        if !(1..=32).contains(&num_bits) || num_bits > available_bits {
            let mut message = format!("invalid num_bits: {num_bits}; expected 1..=32");
            if num_bits > available_bits {
                message.push_str(&format!("; available bits: {available_bits}"));
            }
            bail!(Error::invalid_argument(message));
        }

        let mut result: u32 = 0;

        let mut num_bits = num_bits;

        // First, read remainder from current byte
        if self.bit_offset > 0 {
            let bits_left = 8 - self.bit_offset;
            let to_read = cmp::min(num_bits, bits_left);
            let bits_to_not_read = bits_left - to_read;
            let mask = (0xFF >> (8 - to_read)) << bits_to_not_read;

            result = (self.bytes[self.byte_offset] & mask) as u32 >> bits_to_not_read;
            num_bits -= to_read;
            self.bit_offset += to_read;
            if self.bit_offset == 8 {
                self.bit_offset = 0;
                self.byte_offset += 1;
            }
        }

        // Next read whole bytes
        if num_bits > 0 {
            while num_bits >= 8 {
                result = (result << 8) | self.bytes[self.byte_offset] as u32;
                self.byte_offset += 1;
                num_bits -= 8;
            }

            // Finally read a partial byte
            if num_bits > 0 {
                let bits_to_not_read = 8 - num_bits;
                let mask = (0xFF >> bits_to_not_read) << bits_to_not_read;
                result = (result << num_bits)
                    | ((self.bytes[self.byte_offset] & mask) as u32 >> bits_to_not_read);
                self.bit_offset += num_bits;
            }
        }

        Ok(result)
    }

    pub fn peek_bits(&self, num_bits: usize) -> Result<u32> {
        let available_bits = self.available();
        if !(1..=32).contains(&num_bits) || num_bits > available_bits {
            let mut message = format!("invalid num_bits: {num_bits}; expected 1..=32");
            if num_bits > available_bits {
                message.push_str(&format!("; available bits: {available_bits}"));
            }
            bail!(Error::invalid_argument(message));
        }

        let mut bit_offset = self.bit_offset;
        let mut byte_offset = self.byte_offset;

        let mut result: u32 = 0;

        let mut num_bits = num_bits;

        // First, read remainder from current byte
        if self.bit_offset > 0 {
            let bits_left = 8 - self.bit_offset;
            let to_read = cmp::min(num_bits, bits_left);
            let bits_to_not_read = bits_left - to_read;
            let mask = (0xFF >> (8 - to_read)) << bits_to_not_read;

            result = (self.bytes[self.byte_offset] & mask) as u32 >> bits_to_not_read;
            num_bits -= to_read;
            bit_offset += to_read;
            if bit_offset == 8 {
                byte_offset += 1;
            }
        }

        // Next read whole bytes
        if num_bits > 0 {
            while num_bits >= 8 {
                result = (result << 8) | self.bytes[byte_offset] as u32;
                byte_offset += 1;
                num_bits -= 8;
            }

            // Finally read a partial byte
            if num_bits > 0 {
                let bits_to_not_read = 8 - num_bits;
                let mask = (0xFF >> bits_to_not_read) << bits_to_not_read;
                result = (result << num_bits)
                    | ((self.bytes[byte_offset] & mask) as u32 >> bits_to_not_read);
            }
        }

        Ok(result)
    }

    /// Returns number of bits that can be read successfully.
    pub fn available(&self) -> usize {
        8 * (self.bytes.len() - self.byte_offset) - self.bit_offset
    }
}

impl Read for BitSource<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let requested_bytes = buf.len();
        let available_bytes = self.available() / 8;

        let to_read = if requested_bytes <= available_bytes {
            requested_bytes
        } else {
            available_bytes
        };

        for byte in buf.iter_mut().take(to_read) {
            let Ok(bits) = self.read_bits(8) else {
                return Err(std::io::Error::new(
                    ErrorKind::Unsupported,
                    "unable to read bits",
                ));
            };
            *byte = bits as u8;
        }

        Ok(to_read)
    }
}
