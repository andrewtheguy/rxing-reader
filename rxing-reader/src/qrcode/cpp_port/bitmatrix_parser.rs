// /*
// * Copyright 2016 Nu-book Inc.
// * Copyright 2016 ZXing authors
// */
// // SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

use crate::{
    Error,
    common::BitMatrix,
    qrcode::{FormatInformation, Version, VersionRef},
};

use super::data_mask::data_mask_bit;
use crate::common::cpp_essentials::append_bit;

pub fn bit(bit_matrix: &BitMatrix, x: u32, y: u32, mirrored: Option<bool>) -> bool {
    let mirrored = mirrored.unwrap_or(false);
    if mirrored {
        bit_matrix.get(y, x)
    } else {
        bit_matrix.get(x, y)
    }
}

pub fn read_version(bit_matrix: &BitMatrix) -> Result<VersionRef> {
    if !Version::has_valid_size(bit_matrix) {
        return Err(Error::InvalidFormat {
            message: format!(
                "QR data is malformed: matrix size {}x{} is not a valid QR size",
                bit_matrix.width(),
                bit_matrix.height(),
            ).into(),
        }
        .into());
    }

    let number = Version::number_from_matrix(bit_matrix);

    Version::for_number(number)
}

pub fn read_format_information(bit_matrix: &BitMatrix) -> Result<FormatInformation> {
    // Read top-left format info bits
    let mut format_info_bits1 = 0;
    for x in 0..6 {
        append_bit(&mut format_info_bits1, bit(bit_matrix, x, 8, None));
    }
    // .. and skip a bit in the timing pattern ...
    append_bit(&mut format_info_bits1, bit(bit_matrix, 7, 8, None));
    append_bit(&mut format_info_bits1, bit(bit_matrix, 8, 8, None));
    append_bit(&mut format_info_bits1, bit(bit_matrix, 8, 7, None));
    // .. and skip a bit in the timing pattern ...
    for y in (0..=5).rev() {
        append_bit(&mut format_info_bits1, bit(bit_matrix, 8, y, None));
    }

    // Read the top-right/bottom-left pattern including the 'Dark Module' from the bottom-left
    // part that has to be considered separately when looking for mirrored symbols.
    // See also FormatInformation::decode_qr
    let dimension = bit_matrix.height();
    let mut format_info_bits2 = 0;
    for y in ((dimension - 8)..=(dimension - 1)).rev() {
        append_bit(&mut format_info_bits2, bit(bit_matrix, 8, y, None));
    }
    for x in (dimension - 8)..dimension {
        append_bit(&mut format_info_bits2, bit(bit_matrix, x, 8, None));
    }

    Ok(FormatInformation::decode_qr(
        format_info_bits1 as u32,
        format_info_bits2 as u32,
    ))
}

pub fn read_qrcodewords(
    bit_matrix: &BitMatrix,
    version: VersionRef,
    format_info: &FormatInformation,
) -> Result<Vec<u8>> {
    let function_pattern: BitMatrix = version.build_function_pattern()?;

    let mut result = Vec::with_capacity(version.total_codewords() as usize);
    let mut current_byte = 0;
    let mut reading_up = true;
    let mut bits_read = 0;
    let dimension = bit_matrix.height();
    // Read columns in pairs, from right to left
    let mut x = (dimension as i32) - 1;
    while x > 0 {
        // Skip whole column with vertical timing pattern.
        if x == 6 {
            x -= 1;
        }
        // Read alternatingly from bottom to top then top to bottom
        for row in 0..dimension {
            let y = if reading_up { dimension - 1 - row } else { row };
            for col in 0..2 {
                let xx = (x - col) as u32;
                // Ignore bits covered by the function pattern
                if !function_pattern.get(xx, y) {
                    // Read a bit
                    append_bit(
                        &mut current_byte,
                        data_mask_bit(format_info.data_mask as u32, xx, y)?
                            != bit(bit_matrix, xx, y, Some(format_info.is_mirrored)),
                    );
                    // If we've made a whole byte, save it off
                    bits_read += 1;
                    if bits_read % 8 == 0 {
                        result.push(std::mem::take(&mut current_byte));
                    }
                }
            }
        }
        reading_up = !reading_up; // switch directions

        x -= 2;
    }
    let expected_codewords = version.total_codewords() as usize;
    let actual_codewords = result.len();
    if actual_codewords != expected_codewords {
        return Err(Error::InvalidFormat {
            message: format!(
                "QR data is malformed: expected {expected_codewords} codewords, found {actual_codewords}"
            ).into(),
        }
        .into());
    }

    Ok(result.iter().copied().map(|x| x as u8).collect())
}

pub fn read_codewords(
    bit_matrix: &BitMatrix,
    version: VersionRef,
    format_info: &FormatInformation,
) -> Result<Vec<u8>> {
    read_qrcodewords(bit_matrix, version, format_info)
}
