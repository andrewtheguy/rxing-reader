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

use crate::Error;
use anyhow::Result;

use super::Version;

/// See ISO 18004:2006, 6.4.1, Tables 2 and 3. This enum encapsulates the various modes in which
/// data can be encoded to bits in the QR code standard.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum Mode {
    Terminator = 0x0,
    Numeric = 0x1,
    Alphanumeric = 0x2,
    StructuredAppend = 0x3,
    Byte = 0x4,
    Fnc1FirstPosition = 0x5,
    Eci = 0x7,
    Kanji = 0x8,
    Fnc1SecondPosition = 0x9,
    /// See GBT 18284-2000; "Hanzi" is a transliteration of this mode name.
    Hanzi = 0xD,
}

impl Mode {
    const MODE_INDICATOR_BITS: usize = 4;

    /// Converts the four-bit QR Code mode indicator into a [`Mode`].
    ///
    /// - `bits`: four bits encoding a QR Code data mode.
    ///
    /// Returns an invalid-argument error if `bits` does not correspond to a known mode.
    pub fn for_bits(bits: u8) -> Result<Self> {
        match bits {
            0x0 => Ok(Self::Terminator),
            0x1 => Ok(Self::Numeric),
            0x2 => Ok(Self::Alphanumeric),
            0x3 => Ok(Self::StructuredAppend),
            0x4 => Ok(Self::Byte),
            0x5 => Ok(Self::Fnc1FirstPosition),
            0x7 => Ok(Self::Eci),
            0x8 => Ok(Self::Kanji),
            0x9 => Ok(Self::Fnc1SecondPosition),
            0xD =>
            // 0xD is defined in GBT 18284-2000, may not be supported in foreign country
            {
                Ok(Self::Hanzi)
            }
            _ => Err(Error::InvalidArgument {
                message: format!("{bits} is not valid").into(),
            }
            .into()),
        }
    }

    /// Returns the character-count field width for this mode and QR version.
    ///
    /// The returned value is the number of bits used to encode the count of
    /// characters that follow this mode indicator.
    pub fn character_count_bits(&self, version: &Version) -> usize {
        let number = version.number();

        let offset = if number <= 9 {
            0
        } else if number <= 26 {
            1
        } else {
            2
        };
        self.character_counts()[offset]
    }

    fn character_counts(&self) -> [usize; 3] {
        match self {
            Mode::Terminator => [0, 0, 0],
            Mode::Numeric => [10, 12, 14],
            Mode::Alphanumeric => [9, 11, 13],
            Mode::StructuredAppend => [0, 0, 0],
            Mode::Byte => [8, 16, 16],
            Mode::Eci => [0, 0, 0],
            Mode::Kanji => [8, 10, 12],
            Mode::Fnc1FirstPosition => [0, 0, 0],
            Mode::Fnc1SecondPosition => [0, 0, 0],
            Mode::Hanzi => [8, 10, 12],
        }
    }

    pub const fn bits(self) -> u8 {
        self as u8
    }

    pub fn terminator_bit_length(version: &Version) -> usize {
        let _ = version;
        Self::MODE_INDICATOR_BITS
    }

    pub fn codec_mode_bits_length(version: &Version) -> usize {
        let _ = version;
        Self::MODE_INDICATOR_BITS
    }
    /// Converts a QR mode indicator into a [`Mode`].
    ///
    /// - `bits`: variable-width mode indicator.
    ///
    /// Returns an invalid-format error if `bits` does not correspond to a known mode.
    pub fn codec_mode_for_bits(bits: u32) -> Result<Self> {
        Mode::try_from(bits).map_err(|_| Error::InvalidFormat {
            message: format!("Invalid QR codec mode bits 0x{bits:X}").into(),
        }
        .into())
    }
}

impl From<Mode> for u8 {
    fn from(value: Mode) -> Self {
        value.bits()
    }
}

impl TryFrom<u8> for Mode {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::for_bits(value)
    }
}

impl TryFrom<u32> for Mode {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let value = u8::try_from(value).map_err(|_| Error::InvalidArgument {
            message: format!("{value} is not valid").into(),
        })?;
        Self::for_bits(value)
    }
}
