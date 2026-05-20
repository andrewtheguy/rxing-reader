use crate::qrcode::{
    ErrorCorrectionLevel, FORMAT_INFO_MASK_MODEL2, FORMAT_INFO_MASK_QR, FormatInformation,
};

impl FormatInformation {
    /// - `format_info_bits1`: format info indicator, with mask still applied
    /// - `format_info_bits2`: second copy of same info; both are checked at the same time to establish best match
    pub fn decode_qr(format_info_bits1: u32, format_info_bits2: u32) -> Self {
        // maks out the 'Dark Module' for mirrored and non-mirrored case (see Figure 25 in ISO/IEC 18004:2015)
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
            ErrorCorrectionLevel::eclevel_from_bits((format_info.data >> 3) as u8 & 0x03);
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
        const MODEL2_MASKED_PATTERNS: [u32; 32] = [
            0x5412, 0x5125, 0x5E7C, 0x5B4B, 0x45F9, 0x40CE, 0x4F97, 0x4AA0, 0x77C4, 0x72F3, 0x7DAA,
            0x789D, 0x662F, 0x6318, 0x6C41, 0x6976, 0x1689, 0x13BE, 0x1CE7, 0x19D0, 0x0762, 0x0255,
            0x0D0C, 0x083B, 0x355F, 0x3068, 0x3F31, 0x3A06, 0x24B4, 0x2183, 0x2EDA, 0x2BED,
        ];

        for mask in masks {
            for (bits_index, bits_item) in bits.iter().enumerate() {
                for ref_pattern in MODEL2_MASKED_PATTERNS {
                    // 'unmask' the pattern first to get the original 5-data bits + 10-ec bits back
                    let pattern = ref_pattern ^ FORMAT_INFO_MASK_MODEL2;
                    // Find the pattern with fewest bits differing
                    let hamming_dist = ((bits_item ^ mask) ^ pattern).count_ones();
                    if hamming_dist < fi.hamming_distance {
                        fi.mask = *mask; // store the used mask to discriminate between types/models
                        fi.data = pattern >> 10; // drop the 10 BCH error correction bits
                        fi.hamming_distance = hamming_dist;
                        fi.bits_index = bits_index as u8;
                    }
                }
            }
        }

        fi
    }
}
