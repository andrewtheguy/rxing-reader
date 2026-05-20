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

use crate::{Error, common::BitMatrix};

/// Encapsulates data masks for the data bits in a QR code, per ISO 18004:2006 6.8.
///
/// Each mask can unmask a raw [`BitMatrix`]. For simplicity, masks unmask the entire matrix,
/// including areas used for finder patterns, timing patterns, etc. These areas should be unused
/// after the point they are unmasked anyway.
///
/// Note that the diagram in section 6.8.1 is misleading since it indicates that i is column position
/// and j is row position. In fact, as the text says, i is row position and j is column position.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DataMask {
    // See ISO 18004:2006 6.8.1
    /// 000: mask bits for which (x + y) mod 2 == 0
    DataMask000,

    /// 001: mask bits for which x mod 2 == 0
    DataMask001,

    /// 010: mask bits for which y mod 3 == 0
    DataMask010,

    /// 011: mask bits for which (x + y) mod 3 == 0
    DataMask011,

    /// 100: mask bits for which (x/2 + y/3) mod 2 == 0
    DataMask100,

    /// 101: mask bits for which xy mod 2 + xy mod 3 == 0
    /// equivalently, such that xy mod 6 == 0
    DataMask101,

    /// 110: mask bits for which (xy mod 2 + xy mod 3) mod 2 == 0
    /// equivalently, such that xy mod 6 < 3
    DataMask110,

    /// 111: mask bits for which ((x+y)mod 2 + xy mod 3) mod 2 == 0
    /// equivalently, such that (x + y + xy mod 3) mod 2 == 0
    DataMask111,
    // End of enum constants.
}

impl DataMask {
    /// Implementations of this method reverse the data masking process applied to a QR Code and
    /// make its bits ready to read.
    ///
    /// - `bits`: representation of QR Code bits
    /// - `dimension`: dimension of QR Code, represented by bits, being unmasked
    pub fn unmask_bit_matrix(&self, bits: &mut BitMatrix, dimension: u32) {
        for i in 0..dimension {
            for j in 0..dimension {
                if self.is_masked(i, j) {
                    bits.flip_coords(j, i);
                }
            }
        }
    }

    pub fn is_masked(&self, i: u32, j: u32) -> bool {
        match self {
            DataMask::DataMask000 => ((i + j) & 0x01) == 0,
            DataMask::DataMask001 => (i & 0x01) == 0,
            DataMask::DataMask010 => j.is_multiple_of(3),
            DataMask::DataMask011 => (i + j).is_multiple_of(3),
            DataMask::DataMask100 => (((i / 2) + (j / 3)) & 0x01) == 0,
            DataMask::DataMask101 => (i * j).is_multiple_of(6),
            DataMask::DataMask110 => ((i * j) % 6) < 3,
            DataMask::DataMask111 => ((i + j + ((i * j) % 3)) & 0x01) == 0,
        }
    }
}

impl TryFrom<u8> for DataMask {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DataMask::DataMask000),
            1 => Ok(DataMask::DataMask001),
            2 => Ok(DataMask::DataMask010),
            3 => Ok(DataMask::DataMask011),
            4 => Ok(DataMask::DataMask100),
            5 => Ok(DataMask::DataMask101),
            6 => Ok(DataMask::DataMask110),
            7 => Ok(DataMask::DataMask111),
            _ => Err(Error::InvalidArgument {
                message: format!("{value} is not between 0 and 7"),
            }
            .into()),
        }
    }
}
