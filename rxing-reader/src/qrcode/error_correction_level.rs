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

use std::fmt::{self, Display};
use std::str::FromStr;

use crate::Error;
use anyhow::Result;

/// See ISO 18004:2006, 6.5.1. This enum encapsulates the four error correction levels
/// defined by the QR code standard.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ErrorCorrectionLevel {
    /// L = ~7% correction
    L,
    /// M = ~15% correction
    M,
    /// Q = ~25% correction
    Q,
    /// H = ~30% correction
    H,
}

impl ErrorCorrectionLevel {
    /// - `bits`: int containing the two bits encoding a QR Code's error correction level
    ///
    /// Returns ErrorCorrectionLevel representing the encoded error correction level.
    pub fn for_bits(bits: u8) -> Result<Self> {
        match bits {
            0 => Ok(Self::M),
            1 => Ok(Self::L),
            2 => Ok(Self::H),
            3 => Ok(Self::Q),
            _ => Err(Error::InvalidArgument {
                message: format!("{bits} is not a valid bit selection").into(),
            }
            .into()),
        }
    }

    pub const fn format_bits(self) -> u8 {
        match self {
            ErrorCorrectionLevel::L => 0x01,
            ErrorCorrectionLevel::M => 0x00,
            ErrorCorrectionLevel::Q => 0x03,
            ErrorCorrectionLevel::H => 0x02,
        }
    }

    pub const fn ec_blocks_index(self) -> usize {
        match self {
            ErrorCorrectionLevel::L => 0,
            ErrorCorrectionLevel::M => 1,
            ErrorCorrectionLevel::Q => 2,
            ErrorCorrectionLevel::H => 3,
        }
    }
}

impl TryFrom<u8> for ErrorCorrectionLevel {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        ErrorCorrectionLevel::for_bits(value)
    }
}

impl From<ErrorCorrectionLevel> for u8 {
    fn from(value: ErrorCorrectionLevel) -> Self {
        value.format_bits()
    }
}

impl FromStr for ErrorCorrectionLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // First try to see if the string is just the name of the value
        let as_str = match s.to_uppercase().as_str() {
            "L" => Some(ErrorCorrectionLevel::L),
            "M" => Some(ErrorCorrectionLevel::M),
            "Q" => Some(ErrorCorrectionLevel::Q),
            "H" => Some(ErrorCorrectionLevel::H),
            _ => None,
        };

        // If we find something, cool, return it, otherwise keep trying as numbers
        if let Some(as_str) = as_str {
            return Ok(as_str);
        }

        let number_possible = s.parse::<u8>();
        if let Ok(number_possible) = number_possible {
            return number_possible.try_into();
        }

        Err(Error::InvalidArgument {
            message: format!("could not parse {s} into an ec level").into(),
        }
        .into())
    }
}

impl Display for ErrorCorrectionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ErrorCorrectionLevel::L => "L",
            ErrorCorrectionLevel::M => "M",
            ErrorCorrectionLevel::Q => "Q",
            ErrorCorrectionLevel::H => "H",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorCorrectionLevel;

    #[test]
    fn maps_format_bits_to_levels() {
        assert_eq!(ErrorCorrectionLevel::for_bits(0).unwrap(), ErrorCorrectionLevel::M);
        assert_eq!(ErrorCorrectionLevel::for_bits(1).unwrap(), ErrorCorrectionLevel::L);
        assert_eq!(ErrorCorrectionLevel::for_bits(2).unwrap(), ErrorCorrectionLevel::H);
        assert_eq!(ErrorCorrectionLevel::for_bits(3).unwrap(), ErrorCorrectionLevel::Q);
        assert!(ErrorCorrectionLevel::for_bits(4).is_err());
    }
}
