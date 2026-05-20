/*
 * Copyright 2007 ZXing authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 */

use std::fmt;

use bitvec::prelude::*;

use crate::Error;
use anyhow::Result;

type BaseType = super::BitFieldBaseType;
const BASE_BITS: usize = super::BIT_FIELD_BASE_BITS;

type Bits = BitVec<BaseType, Lsb0>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BitArray {
    bits: Bits,
    read_offset: usize,
}

impl BitArray {
    pub fn new() -> Self {
        Self {
            bits: Bits::new(),
            read_offset: 0,
        }
    }

    pub fn with_size(size: usize) -> Self {
        let mut bits = Bits::new();
        bits.resize(size, false);
        Self {
            bits,
            read_offset: 0,
        }
    }

    pub fn with_capacity(size: usize) -> Self {
        Self {
            bits: Bits::with_capacity(size),
            read_offset: 0,
        }
    }

    /// For testing only
    #[cfg(test)]
    pub fn with_initial_values(bits: Vec<BaseType>, size: usize) -> Self {
        let mut bv = Bits::from_vec(bits);
        bv.truncate(size);
        Self {
            bits: bv,
            read_offset: 0,
        }
    }

    pub fn get_size(&self) -> usize {
        self.bits.len()
    }

    pub fn get_size_in_bytes(&self) -> usize {
        self.bits.len().div_ceil(8)
    }

    pub fn get(&self, i: usize) -> bool {
        self.bits[i]
    }

    pub fn try_get(&self, i: usize) -> Option<bool> {
        self.bits.get(i).map(|b| *b)
    }

    pub fn set(&mut self, i: usize) {
        self.bits.set(i, true);
    }

    pub fn unset(&mut self, i: usize) {
        self.bits.set(i, false);
    }

    pub fn flip(&mut self, i: usize) {
        let v = self.bits[i];
        self.bits.set(i, !v);
    }

    pub fn get_next_set(&self, from: usize) -> usize {
        let size = self.bits.len();
        if from >= size {
            return size;
        }
        self.bits[from..]
            .first_one()
            .map(|i| from + i)
            .unwrap_or(size)
    }

    pub fn get_next_unset(&self, from: usize) -> usize {
        let size = self.bits.len();
        if from >= size {
            return size;
        }
        self.bits[from..]
            .first_zero()
            .map(|i| from + i)
            .unwrap_or(size)
    }

    /// Sets a block of `BASE_BITS` bits at the word containing bit `i`.
    /// The least-significant bit of `new_bits` corresponds to bit `(i / BASE_BITS) * BASE_BITS`.
    pub fn set_bulk(&mut self, i: usize, new_bits: BaseType) {
        self.bits.as_raw_mut_slice()[i / BASE_BITS] = new_bits;
    }

    pub fn set_range(&mut self, start: usize, end: usize) -> Result<()> {
        if end < start || end > self.bits.len() {
            return Err(Error::InvalidArgument.into());
        }
        if end == start {
            return Ok(());
        }
        self.bits[start..end].fill(true);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.bits.fill(false);
    }

    pub fn is_range(&self, start: usize, end: usize, value: bool) -> Result<bool> {
        if end < start || end > self.bits.len() {
            return Err(Error::InvalidArgument.into());
        }
        if end == start {
            return Ok(true);
        }
        let slice = &self.bits[start..end];
        Ok(if value { slice.all() } else { slice.not_any() })
    }

    pub fn append_bit(&mut self, bit: bool) {
        self.bits.push(bit);
    }

    /// Appends the least-significant `num_bits` of `value`, from most-significant to
    /// least-significant. For example, appending 6 bits from 0x1E appends 0,1,1,1,1,0.
    pub fn append_bits(&mut self, value: BaseType, num_bits: usize) -> Result<()> {
        if num_bits > BASE_BITS {
            return Err(Error::invalid_argument(format!(
                "num bits must be between 0 and {}",
                BaseType::BITS
            )).into());
        }
        for i in (0..num_bits).rev() {
            self.bits.push((value >> i) & 1 != 0);
        }
        Ok(())
    }

    pub fn append_bit_array(&mut self, other: BitArray) {
        self.append_bit_array_ref(&other)
    }

    pub fn append_bit_array_ref(&mut self, other: &BitArray) {
        self.bits.extend_from_bitslice(&other.bits);
    }

    pub fn xor(&mut self, other: &BitArray) -> Result<()> {
        if self.bits.len() != other.bits.len() {
            return Err(Error::invalid_argument("Sizes don't match").into());
        }
        self.bits ^= &other.bits;
        Ok(())
    }

    /// Writes `num_bytes` bytes starting at `bit_offset` into `array` at `offset`. Bytes are
    /// most-significant-bit first.
    pub fn to_bytes(&self, bit_offset: usize, array: &mut [u8], offset: usize, num_bytes: usize) {
        let mut bit_offset = bit_offset;
        for i in 0..num_bytes {
            let mut the_byte: u8 = 0;
            for j in 0..8 {
                if self.bits[bit_offset] {
                    the_byte |= 1 << (7 - j);
                }
                bit_offset += 1;
            }
            array[offset + i] = the_byte;
        }
    }

    pub fn get_bit_array(&self) -> &[BaseType] {
        self.bits.as_raw_slice()
    }

    pub fn reverse(&mut self) {
        self.bits.reverse();
    }
}

impl fmt::Display for BitArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let size = self.bits.len();
        let mut s = String::with_capacity(size + size / 8 + 1);
        for i in 0..size {
            if (i & 0x07) == 0 {
                s.push(' ');
            }
            s.push(if self.bits[i] { 'X' } else { '.' });
        }
        write!(f, "{s}")
    }
}

impl Default for BitArray {
    fn default() -> Self {
        Self::new()
    }
}

impl From<BitArray> for Vec<u8> {
    fn from(value: BitArray) -> Self {
        (&value).into()
    }
}

impl From<&BitArray> for Vec<u8> {
    fn from(value: &BitArray) -> Self {
        let mut array = vec![0; value.get_size_in_bytes()];
        value.to_bytes(0, &mut array, 0, value.get_size_in_bytes());
        array
    }
}

impl From<BitArray> for Vec<bool> {
    fn from(value: BitArray) -> Self {
        Self::from(&value)
    }
}

impl From<&BitArray> for Vec<bool> {
    fn from(value: &BitArray) -> Self {
        value.bits.iter().map(|b| *b).collect()
    }
}

impl From<Vec<u8>> for BitArray {
    fn from(val: Vec<u8>) -> Self {
        let mut new_array = BitArray::with_size(val.len() * 8);
        for (byte_idx, byte) in val.into_iter().enumerate() {
            for bit in 0..8 {
                if byte & (1 << (7 - bit)) != 0 {
                    new_array.set(byte_idx * 8 + bit);
                }
            }
        }
        new_array
    }
}

impl std::io::Read for BitArray {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.bits.len();
        let desired = buf.len();
        let current_offset = self.read_offset;

        if current_offset >= size {
            return Ok(0);
        }

        let available_bytes = (size - current_offset) / 8;
        if available_bytes == 0 {
            return Ok(0);
        }

        let to_read = desired.min(available_bytes);

        self.to_bytes(current_offset, buf, 0, to_read);

        self.read_offset = current_offset + to_read * 8;

        Ok(to_read)
    }
}

impl std::io::Write for BitArray {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for byte in buf {
            self.append_bits(*byte as BaseType, 8)
                .map_err(|e| std::io::Error::other(e.to_string()))?
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for BitArray {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let size = self.bits.len() as i64;
        let target: i64 = match pos {
            std::io::SeekFrom::Start(s) => i64::try_from(s).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek offset overflow")
            })?,
            std::io::SeekFrom::End(e) => size.checked_add(e).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek offset overflow")
            })?,
            std::io::SeekFrom::Current(c) => (self.read_offset as i64).checked_add(c).ok_or_else(
                || std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek offset overflow"),
            )?,
        };
        if target < 0 || target > size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek position out of bounds",
            ));
        }
        self.read_offset = target as usize;
        Ok(target as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::{BASE_BITS, BitArray};

    #[test]
    fn vec_u8_round_trips_through_bit_array() {
        let bytes = vec![0b1000_0001, 0b0101_1010, 0];
        let bits = BitArray::from(bytes.clone());

        assert_eq!(bits.get_size(), bytes.len() * 8);
        assert_eq!(Vec::<u8>::from(bits), bytes);
    }

    #[test]
    fn set_bulk_stores_new_bits_without_shifting() {
        let mut bits = BitArray::with_size(BASE_BITS * 2);

        bits.set_bulk(5, 0b101);

        assert_eq!(bits.get_bit_array()[0], 0b101);
    }
}
