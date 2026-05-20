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

use super::ErrorCorrectionLevel;

pub const FORMAT_INFO_MASK_QR: u32 = 0x5412;

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
            is_mirrored: false,
            mask: 0,
            data: 255,
            bits_index: 255,
        }
    }
}

impl FormatInformation {
    /// - `format_info_bits1`: format info indicator, with mask still applied
    /// - `format_info_bits2`: second copy of same info; both are checked at the same time to establish best match
    pub fn decode_qr(format_info_bits1: u32, format_info_bits2: u32) -> Self {
        // Mask out the Dark Module for mirrored and non-mirrored cases.
        let mirrored_format_info_bits2 = Self::mirror_bits(
            ((format_info_bits2 >> 1) & 0b111111110000000) | (format_info_bits2 & 0b1111111),
        );
        let format_info_bits2 =
            ((format_info_bits2 >> 1) & 0b111111100000000) | (format_info_bits2 & 0b11111111);
        // Some QR codes do not apply the XOR mask. Try with standard masking and without it.
        let mut format_info = Self::find_best_format_info(
            &[FORMAT_INFO_MASK_QR, 0],
            &[
                format_info_bits1,
                format_info_bits2,
                Self::mirror_bits(format_info_bits1),
                mirrored_format_info_bits2,
            ],
        );

        // Use bits 3/4 for error correction, and 0-2 for mask.
        format_info.error_correction_level =
            ErrorCorrectionLevel::from_format_bits((format_info.data >> 3) as u8);
        format_info.data_mask = format_info.data as u8 & 0x07;
        format_info.is_mirrored = format_info.bits_index > 1;

        format_info
    }

    #[inline(always)]
    pub fn mirror_bits(bits: u32) -> u32 {
        (bits.reverse_bits()) >> 17
    }

    pub fn find_best_format_info(masks: &[u32], bits: &[u32]) -> Self {
        let mut fi = FormatInformation::default();

        // See ISO 18004:2015, Annex C, Table C.1
        const QR_MASKED_PATTERNS: [u32; 32] = [
            0x5412, 0x5125, 0x5E7C, 0x5B4B, 0x45F9, 0x40CE, 0x4F97, 0x4AA0, 0x77C4, 0x72F3, 0x7DAA,
            0x789D, 0x662F, 0x6318, 0x6C41, 0x6976, 0x1689, 0x13BE, 0x1CE7, 0x19D0, 0x0762, 0x0255,
            0x0D0C, 0x083B, 0x355F, 0x3068, 0x3F31, 0x3A06, 0x24B4, 0x2183, 0x2EDA, 0x2BED,
        ];

        for mask in masks {
            for (bits_index, bits_item) in bits.iter().enumerate() {
                for ref_pattern in QR_MASKED_PATTERNS {
                    let pattern = ref_pattern ^ FORMAT_INFO_MASK_QR;
                    let hamming_dist = ((bits_item ^ mask) ^ pattern).count_ones();
                    if hamming_dist < fi.hamming_distance {
                        fi.mask = *mask;
                        fi.data = pattern >> 10;
                        fi.hamming_distance = hamming_dist;
                        fi.bits_index = bits_index as u8;
                    }
                }
            }
        }

        fi
    }
}
