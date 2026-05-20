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
pub enum Mode {
    Terminator,
    Numeric,
    Alphanumeric,
    StructuredAppend,
    Byte,
    Eci,
    Kanji,
    Fnc1FirstPosition,
    Fnc1SecondPosition,
    /// See GBT 18284-2000; "Hanzi" is a transliteration of this mode name.
    Hanzi,
}

impl Mode {
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
                message: format!("{bits} is not valid"),
            }
            .into()),
        }
    }

    /// Returns the character-count field width for this mode and QR version.
    ///
    /// The returned value is the number of bits used to encode the count of
    /// characters that follow this mode indicator.
    pub fn get_character_count_bits(&self, version: &Version) -> u8 {
        let number = version.get_version_number();

        let offset = if number <= 9 {
            0
        } else if number <= 26 {
            1
        } else {
            2
        };
        self.get_character_counts()[offset]
    }

    fn get_character_counts(&self) -> &[u8] {
        match self {
            Mode::Terminator => &[0, 0, 0],
            Mode::Numeric => &[10, 12, 14],
            Mode::Alphanumeric => &[9, 11, 13],
            Mode::StructuredAppend => &[0, 0, 0],
            Mode::Byte => &[8, 16, 16],
            Mode::Eci => &[0, 0, 0],
            Mode::Kanji => &[8, 10, 12],
            Mode::Fnc1FirstPosition => &[0, 0, 0],
            Mode::Fnc1SecondPosition => &[0, 0, 0],
            Mode::Hanzi => &[8, 10, 12],
        }
    }

    pub fn get_bits(&self) -> u8 {
        match self {
            Mode::Terminator => 0x00,
            Mode::Numeric => 0x01,
            Mode::Alphanumeric => 0x02,
            Mode::StructuredAppend => 0x03,
            Mode::Byte => 0x04,
            Mode::Eci => 0x07,
            Mode::Kanji => 0x08,
            Mode::Fnc1FirstPosition => 0x05,
            Mode::Fnc1SecondPosition => 0x09,
            Mode::Hanzi => 0x0D,
        }
    }

    pub fn get_terminator_bit_length(version: &Version) -> u8 {
        let _ = version;
        4
    }

    pub fn get_codec_mode_bits_length(version: &Version) -> u8 {
        let _ = version;
        4
    }
    /// Converts a QR mode indicator into a [`Mode`].
    ///
    /// - `bits`: variable-width mode indicator.
    ///
    /// Returns an invalid-format error if `bits` does not correspond to a known mode.
    pub fn codec_mode_for_bits(bits: u32) -> Result<Self> {
        if (0x00..=0x05).contains(&bits) || (0x07..=0x09).contains(&bits) || bits == 0x0d {
            return Mode::try_from(bits);
        }

        Err(Error::InvalidFormat {
            message: format!("Invalid QR codec mode bits 0x{bits:X}"),
        }
        .into())
    }

    /// Returns the character-count field width for this mode and symbol version.
    ///
    /// The returned value is the number of bits used to encode the count of
    /// characters that follow this mode indicator.
    pub fn character_count_bits(&self, version: &Version) -> u32 {
        self.get_character_count_bits(version) as u32
    }
}

impl From<Mode> for u8 {
    fn from(value: Mode) -> Self {
        value.get_bits()
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
        if value > u32::from(u8::MAX) {
            return Err(Error::InvalidArgument {
                message: format!("{value} is not valid"),
            }
            .into());
        }
        Self::for_bits(value as u8)
    }
}
