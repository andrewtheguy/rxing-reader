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

use super::ErrorCorrectionLevel;

pub const FORMAT_INFO_MASK_QR: u32 = 0x5412;
pub const FORMAT_INFO_MASK_MODEL2: u32 = FORMAT_INFO_MASK_QR;

/// See ISO 18004:2006, Annex C, Table C.1
pub const FORMAT_INFO_DECODE_LOOKUP: [[u32; 2]; 32] = [
    [0x5412, 0x00],
    [0x5125, 0x01],
    [0x5E7C, 0x02],
    [0x5B4B, 0x03],
    [0x45F9, 0x04],
    [0x40CE, 0x05],
    [0x4F97, 0x06],
    [0x4AA0, 0x07],
    [0x77C4, 0x08],
    [0x72F3, 0x09],
    [0x7DAA, 0x0A],
    [0x789D, 0x0B],
    [0x662F, 0x0C],
    [0x6318, 0x0D],
    [0x6C41, 0x0E],
    [0x6976, 0x0F],
    [0x1689, 0x10],
    [0x13BE, 0x11],
    [0x1CE7, 0x12],
    [0x19D0, 0x13],
    [0x0762, 0x14],
    [0x0255, 0x15],
    [0x0D0C, 0x16],
    [0x083B, 0x17],
    [0x355F, 0x18],
    [0x3068, 0x19],
    [0x3F31, 0x1A],
    [0x3A06, 0x1B],
    [0x24B4, 0x1C],
    [0x2183, 0x1D],
    [0x2EDA, 0x1E],
    [0x2BED, 0x1F],
];

/// Encapsulates a QR Code's format information, including the data mask used and
/// error correction level.
///
/// See DataMask.
/// See ErrorCorrectionLevel.
#[derive(Hash, Eq, PartialEq, Debug)]
pub struct FormatInformation {
    pub hamming_distance: u32,
    pub error_correction_level: ErrorCorrectionLevel,
    pub data_mask: u8,
    pub micro_version: u32,
    pub is_mirrored: bool,

    pub mask: u32,      // = 0
    pub data: u32,      // = 255
    pub bits_index: u8, // = 255;
}

impl Default for FormatInformation {
    fn default() -> Self {
        Self {
            hamming_distance: 255,
            error_correction_level: ErrorCorrectionLevel::Invalid,
            data_mask: Default::default(),
            micro_version: 0,
            is_mirrored: false,
            mask: 0,
            data: 255,
            bits_index: 255,
        }
    }
}

impl FormatInformation {
    fn new(format_info: u8) -> Result<Self> {
        // Bits 3,4
        let error_correction_level = ErrorCorrectionLevel::for_bits((format_info >> 3) & 0x03)?;
        // Bottom 3 bits
        let data_mask = format_info & 0x07;
        Ok(Self {
            hamming_distance: 255,
            micro_version: 0,
            error_correction_level,
            data_mask,
            is_mirrored: false,
            mask: 0,
            bits_index: 255,
            data: 255,
        })
    }

    pub fn num_bits_differing(a: u32, b: u32) -> u32 {
        (a ^ b).count_ones()
    }

    /// - `maskedFormatInfo1`: format info indicator, with mask still applied
    /// - `maskedFormatInfo2`: second copy of same info; both are checked at the same time
    ///   to establish best match
    ///
    /// Returns information about the format it specifies, or `None`.
    /// if doesn't seem to match any known pattern
    pub fn decode_format_information(
        masked_format_info1: u32,
        masked_format_info2: u32,
    ) -> Option<FormatInformation> {
        let format_info =
            Self::do_decode_format_information(masked_format_info1, masked_format_info2);
        if format_info.is_some() {
            return format_info;
        }
        // Should return null, but, some QR codes apparently
        // do not mask this info. Try again by actually masking the pattern
        // first
        Self::do_decode_format_information(
            masked_format_info1 ^ FORMAT_INFO_MASK_QR,
            masked_format_info2 ^ FORMAT_INFO_MASK_QR,
        )
    }

    fn do_decode_format_information(
        masked_format_info1: u32,
        masked_format_info2: u32,
    ) -> Option<FormatInformation> {
        // Find the int in FORMAT_INFO_DECODE_LOOKUP with fewest bits differing
        let mut best_difference = u32::MAX;
        let mut best_format_info = 0;
        for decode_info in FORMAT_INFO_DECODE_LOOKUP {
            let target_info = decode_info[0];
            if target_info == masked_format_info1 || target_info == masked_format_info2 {
                // Found an exact match
                return FormatInformation::new(decode_info[1] as u8).ok();
            }
            let mut bits_difference = Self::num_bits_differing(masked_format_info1, target_info);
            if bits_difference < best_difference {
                best_format_info = decode_info[1] as u8;
                best_difference = bits_difference;
            }
            if masked_format_info1 != masked_format_info2 {
                // also try the other option
                bits_difference = Self::num_bits_differing(masked_format_info2, target_info);
                if bits_difference < best_difference {
                    best_format_info = decode_info[1] as u8;
                    best_difference = bits_difference;
                }
            }
        }
        // Hamming distance of the 32 masked codes is 7, by construction, so <= 3 bits
        // differing means we found a match
        if best_difference <= 3 {
            return FormatInformation::new(best_format_info).ok();
        }
        None
    }

    pub fn get_error_correction_level(&self) -> ErrorCorrectionLevel {
        self.error_correction_level
    }

    pub fn get_data_mask(&self) -> u8 {
        self.data_mask
    }
}
