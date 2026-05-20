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

use anyhow::Result;

use crate::Error;

use super::ErrorCorrectionLevel;

pub const FORMAT_INFO_MASK_QR: u16 = 0x5412;
const FORMAT_INFO_BITS: u32 = 15;
const FORMAT_INFO_DATA_SHIFT: u16 = 10;
const FORMAT_INFO_ERROR_CORRECTION_SHIFT: u8 = 3;
const FORMAT_INFO_DATA_MASK: u8 = 0x07;
const MAX_FORMAT_INFO_ERRORS: u32 = 3;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub enum DataMask {
    Pattern000,
    Pattern001,
    Pattern010,
    Pattern011,
    Pattern100,
    Pattern101,
    Pattern110,
    Pattern111,
}

impl DataMask {
    pub fn is_masked(self, x: u32, y: u32) -> bool {
        match self {
            DataMask::Pattern000 => (y + x).is_multiple_of(2),
            DataMask::Pattern001 => y.is_multiple_of(2),
            DataMask::Pattern010 => x.is_multiple_of(3),
            DataMask::Pattern011 => (y + x).is_multiple_of(3),
            DataMask::Pattern100 => ((y / 2) + (x / 3)).is_multiple_of(2),
            DataMask::Pattern101 => (y * x).is_multiple_of(6),
            DataMask::Pattern110 => ((y * x) % 6) < 3,
            DataMask::Pattern111 => (y + x + ((y * x) % 3)).is_multiple_of(2),
        }
    }
}

impl TryFrom<u8> for DataMask {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Pattern000),
            1 => Ok(Self::Pattern001),
            2 => Ok(Self::Pattern010),
            3 => Ok(Self::Pattern011),
            4 => Ok(Self::Pattern100),
            5 => Ok(Self::Pattern101),
            6 => Ok(Self::Pattern110),
            7 => Ok(Self::Pattern111),
            _ => Err(Error::InvalidArgument {
                message: format!("QR data-mask bits {value} out of range (expected 0..=7)").into(),
            }
            .into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatInfoSource {
    Primary,
    Secondary,
    MirroredPrimary,
    MirroredSecondary,
}

impl FormatInfoSource {
    fn from_bits_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Primary),
            1 => Some(Self::Secondary),
            2 => Some(Self::MirroredPrimary),
            3 => Some(Self::MirroredSecondary),
            _ => None,
        }
    }

    const fn is_mirrored(self) -> bool {
        matches!(
            self,
            Self::MirroredPrimary | Self::MirroredSecondary
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct FormatInfoCandidate {
    hamming_distance: u32,
    data: u8,
    source: FormatInfoSource,
}

/// Encapsulates a QR Code's format information, including the data mask used and
/// error correction level.
///
/// See DataMask.
/// See ErrorCorrectionLevel.
#[derive(Hash, Eq, PartialEq, Debug)]
pub struct FormatInformation {
    pub hamming_distance: u32,
    pub error_correction_level: ErrorCorrectionLevel,
    pub data_mask: DataMask,
    pub is_mirrored: bool,
}

impl FormatInformation {
    /// - `format_info_bits1`: format info indicator, with mask still applied
    /// - `format_info_bits2`: second copy of same info; both are checked at the same time to establish best match
    pub fn decode_qr(format_info_bits1: u16, format_info_bits2: u16) -> Result<Self> {
        // Mask out the Dark Module for mirrored and non-mirrored cases.
        let mirrored_format_info_bits2 = Self::mirror_bits(
            ((format_info_bits2 >> 1) & 0b111111110000000) | (format_info_bits2 & 0b1111111),
        );
        let format_info_bits2 =
            ((format_info_bits2 >> 1) & 0b111111100000000) | (format_info_bits2 & 0b11111111);
        // Some QR codes do not apply the XOR mask. Try with standard masking and without it.
        let candidate = Self::find_best_format_info(
            &[FORMAT_INFO_MASK_QR, 0],
            &[
                format_info_bits1,
                format_info_bits2,
                Self::mirror_bits(format_info_bits1),
                mirrored_format_info_bits2,
            ],
        )
        .filter(|candidate| candidate.hamming_distance <= MAX_FORMAT_INFO_ERRORS)
        .ok_or_else(|| Error::InvalidFormat {
            message: format!(
                "QR format information is too damaged to decode (top-left=0x{format_info_bits1:04X}, top-right/bottom-left=0x{format_info_bits2:04X})"
            )
            .into(),
        })?;

        // Use bits 3/4 for error correction, and 0-2 for mask.
        let error_correction_level =
            ErrorCorrectionLevel::for_bits(candidate.data >> FORMAT_INFO_ERROR_CORRECTION_SHIFT)?;
        let data_mask = DataMask::try_from(candidate.data & FORMAT_INFO_DATA_MASK)?;

        Ok(Self {
            hamming_distance: candidate.hamming_distance,
            error_correction_level,
            data_mask,
            is_mirrored: candidate.source.is_mirrored(),
        })
    }

    #[inline(always)]
    pub fn mirror_bits(bits: u16) -> u16 {
        bits.reverse_bits() >> (u16::BITS - FORMAT_INFO_BITS)
    }

    fn find_best_format_info(masks: &[u16], bits: &[u16]) -> Option<FormatInfoCandidate> {
        // See ISO 18004:2015, Annex C, Table C.1
        const QR_MASKED_PATTERNS: [u16; 32] = [
            0x5412, 0x5125, 0x5E7C, 0x5B4B, 0x45F9, 0x40CE, 0x4F97, 0x4AA0, 0x77C4, 0x72F3, 0x7DAA,
            0x789D, 0x662F, 0x6318, 0x6C41, 0x6976, 0x1689, 0x13BE, 0x1CE7, 0x19D0, 0x0762, 0x0255,
            0x0D0C, 0x083B, 0x355F, 0x3068, 0x3F31, 0x3A06, 0x24B4, 0x2183, 0x2EDA, 0x2BED,
        ];

        let mut best = None;
        for mask in masks {
            for (bits_index, bits_item) in bits.iter().enumerate() {
                let Some(source) = FormatInfoSource::from_bits_index(bits_index) else {
                    continue;
                };
                for ref_pattern in QR_MASKED_PATTERNS {
                    let pattern = ref_pattern ^ FORMAT_INFO_MASK_QR;
                    let hamming_dist = ((bits_item ^ mask) ^ pattern).count_ones();
                    let data = u8::try_from(pattern >> FORMAT_INFO_DATA_SHIFT).ok()?;
                    let candidate = FormatInfoCandidate {
                        hamming_distance: hamming_dist,
                        data,
                        source,
                    };
                    if best
                        .map(|best: FormatInfoCandidate| hamming_dist < best.hamming_distance)
                        .unwrap_or(true)
                    {
                        best = Some(candidate);
                    }
                }
            }
        }

        best
    }
}
