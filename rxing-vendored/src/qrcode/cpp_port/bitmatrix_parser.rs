// /*
// * Copyright 2016 Nu-book Inc.
// * Copyright 2016 ZXing authors
// */
// // SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

use crate::{
    Error,
    common::BitMatrix,
    qrcode::common::{ErrorCorrectionLevel, FormatInformation, Version, VersionRef},
};

use super::{Type, data_mask::get_data_mask_bit};
use crate::common::cpp_essentials::append_bit;

pub fn get_bit(bit_matrix: &BitMatrix, x: u32, y: u32, mirrored: Option<bool>) -> bool {
    let mirrored = mirrored.unwrap_or(false);
    if mirrored {
        bit_matrix.get(y, x)
    } else {
        bit_matrix.get(x, y)
    }
}

pub fn read_version(bit_matrix: &BitMatrix, qr_type: Type) -> Result<VersionRef> {
    if !Version::has_valid_size(bit_matrix) {
        return Err(Error::InvalidFormat {
            message: format!(
                "QR data is malformed: matrix size {}x{} is not a valid QR size",
                bit_matrix.width(),
                bit_matrix.height(),
            ),
        }
        .into());
    }

    let number = Version::number(bit_matrix);

    match qr_type {
        Type::Model1 => Version::model1(number),
        Type::Micro => Version::micro(number),
        Type::Model2 => Version::model2(number),
        Type::RectMicro => Version::r_mqr(number),
    }
}

pub fn read_format_information(bit_matrix: &BitMatrix) -> Result<FormatInformation> {
    if Version::has_valid_size_type(bit_matrix, Type::Micro) {
        // Read top-left format info bits
        let mut format_info_bits = 0;
        for x in 1..9 {
            append_bit(&mut format_info_bits, get_bit(bit_matrix, x, 8, None));
        }
        for y in (1..=7).rev() {
            append_bit(&mut format_info_bits, get_bit(bit_matrix, 8, y, None));
        }

        return Ok(FormatInformation::decode_mqr(format_info_bits as u32));
    }

    if Version::has_valid_size_type(bit_matrix, Type::RectMicro) {
        // Read top-left format info bits
        let mut format_info_bits1 = 0;
        for y in (1..=3).rev() {
            append_bit(&mut format_info_bits1, get_bit(bit_matrix, 11, y, None));
        }
        for x in (8..=10).rev() {
            for y in (1..=5).rev() {
                append_bit(&mut format_info_bits1, get_bit(bit_matrix, x, y, None));
            }
        }
        // Read bottom-right format info bits
        let mut format_info_bits2 = 0;
        let width = bit_matrix.width();
        let height = bit_matrix.height();
        for x in 3..=5 {
            append_bit(
                &mut format_info_bits2,
                get_bit(bit_matrix, width - x, height - 6, None),
            );
        }
        for x in 6..=8 {
            for y in 2..=6 {
                append_bit(
                    &mut format_info_bits2,
                    get_bit(bit_matrix, width - x, height - y, None),
                );
            }
        }
        return Ok(FormatInformation::decode_rmqr(
            format_info_bits1 as u32,
            format_info_bits2 as u32,
        ));
    }

    // Read top-left format info bits
    let mut format_info_bits1 = 0;
    for x in 0..6 {
        append_bit(&mut format_info_bits1, get_bit(bit_matrix, x, 8, None));
    }
    // .. and skip a bit in the timing pattern ...
    append_bit(&mut format_info_bits1, get_bit(bit_matrix, 7, 8, None));
    append_bit(&mut format_info_bits1, get_bit(bit_matrix, 8, 8, None));
    append_bit(&mut format_info_bits1, get_bit(bit_matrix, 8, 7, None));
    // .. and skip a bit in the timing pattern ...
    for y in (0..=5).rev() {
        append_bit(&mut format_info_bits1, get_bit(bit_matrix, 8, y, None));
    }

    // Read the top-right/bottom-left pattern including the 'Dark Module' from the bottom-left
    // part that has to be considered separately when looking for mirrored symbols.
    // See also FormatInformation::decode_qr
    let dimension = bit_matrix.height();
    let mut format_info_bits2 = 0;
    for y in ((dimension - 8)..=(dimension - 1)).rev() {
        append_bit(&mut format_info_bits2, get_bit(bit_matrix, 8, y, None));
    }
    for x in (dimension - 8)..dimension {
        append_bit(&mut format_info_bits2, get_bit(bit_matrix, x, 8, None));
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

    let mut result = Vec::with_capacity(version.get_total_codewords() as usize);
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
                        get_data_mask_bit(format_info.data_mask as u32, xx, y, None)?
                            != get_bit(bit_matrix, xx, y, Some(format_info.is_mirrored)),
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
    let expected_codewords = version.get_total_codewords() as usize;
    let actual_codewords = result.len();
    if actual_codewords != expected_codewords {
        return Err(Error::InvalidFormat {
            message: format!(
                "QR data is malformed: expected {expected_codewords} codewords, found {actual_codewords}"
            ),
        }
        .into());
    }

    Ok(result.iter().copied().map(|x| x as u8).collect())
}

pub fn read_mqrcodewords(
    bit_matrix: &BitMatrix,
    version: VersionRef,
    format_info: &FormatInformation,
) -> Result<Vec<u8>> {
    let function_pattern = version.build_function_pattern()?;

    // D3 in a Version M1 symbol, D11 in a Version M3-L symbol and D9
    // in a Version M3-M symbol is a 2x2 square 4-module block.
    // See ISO 18004:2006 6.7.3.
    let has_d4m_block = version.get_version_number() % 2 == 1;
    let d4m_block_index = if version.get_version_number() == 1 {
        3
    } else if format_info.error_correction_level == ErrorCorrectionLevel::L {
        11
    } else {
        9
    };

    let mut result = Vec::with_capacity(version.get_total_codewords() as usize);
    let mut current_byte = 0;
    let mut reading_up = true;
    let mut bits_read = 0;
    let dimension = bit_matrix.height();
    // Read columns in pairs, from right to left
    let mut x = dimension - 1;
    while x > 0 {
        // Read alternatingly from bottom to top then top to bottom
        for row in 0..dimension {
            let y = if reading_up { dimension - 1 - row } else { row };
            for col in 0..2 {
                let xx = x - col;
                // Ignore bits covered by the function pattern
                if !function_pattern.get(xx, y) {
                    // Read a bit
                    append_bit(
                        &mut current_byte,
                        get_data_mask_bit(format_info.data_mask as u32, xx, y, Some(true))?
                            != get_bit(bit_matrix, xx, y, Some(format_info.is_mirrored)),
                    );
                    bits_read += 1;
                    // If we've made a whole byte, save it off; save early if 2x2 data block.
                    if bits_read == 8
                        || (bits_read == 4
                            && has_d4m_block
                            && (result.len()) == d4m_block_index - 1)
                    {
                        result.push(std::mem::take(&mut current_byte));
                        bits_read = 0;
                    }
                }
            }
        }
        reading_up = !reading_up; // switch directions

        x -= 2;
    }
    let expected_codewords = version.get_total_codewords() as usize;
    let actual_codewords = result.len();
    if actual_codewords != expected_codewords {
        return Err(Error::InvalidFormat {
            message: format!(
                "QR data is malformed: expected {expected_codewords} codewords, found {actual_codewords}"
            ),
        }
        .into());
    }

    Ok(result.iter().copied().map(|x| x as u8).collect())
}

pub fn read_qrcodewords_model1(
    bit_matrix: &BitMatrix,
    version: VersionRef,
    format_info: &FormatInformation,
) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(version.get_total_codewords() as usize);
    let dimension = bit_matrix.height();
    let columns = dimension / 4 + 1 + 2;
    for j in 0..columns {
        if j <= 1 {
            // vertical symbols on the right side
            let rows = (dimension - 8) / 4;
            for i in 0..rows {
                if j == 0 && i % 2 == 0 && i > 0 && i < rows - 1
                // extension
                {
                    continue;
                }
                let x = (dimension - 1) - (j * 2);
                let y = (dimension - 1) - (i * 4);
                let mut current_byte = 0;
                for b in 0..8 {
                    append_bit(
                        &mut current_byte,
                        get_data_mask_bit(
                            format_info.data_mask as u32,
                            x - b % 2,
                            y - (b / 2),
                            None,
                        )? != get_bit(
                            bit_matrix,
                            x - b % 2,
                            y - (b / 2),
                            Some(format_info.is_mirrored),
                        ),
                    );
                }
                result.push(current_byte);
            }
        } else if columns - j <= 4 {
            // vertical symbols on the left side
            let rows = (dimension - 16) / 4;
            for i in 0..rows {
                let x = (columns - j - 1) * 2 + 1 + (if columns - j == 4 { 1 } else { 0 }); // timing
                let y = (dimension - 1) - 8 - (i * 4);
                let mut current_byte = 0;
                for b in 0..8 {
                    append_bit(
                        &mut current_byte,
                        get_data_mask_bit(
                            format_info.data_mask as u32,
                            x - b % 2,
                            y - (b / 2),
                            None,
                        )? != get_bit(
                            bit_matrix,
                            x - b % 2,
                            y - (b / 2),
                            Some(format_info.is_mirrored),
                        ),
                    );
                }
                result.push(current_byte);
            }
        } else {
            // horizontal symbols
            let rows = dimension / 2;
            for i in 0..rows {
                if j == 2 && i >= rows - 4
                // alignment & finder
                {
                    continue;
                }
                if i == 0 && j % 2 == 1 && j + 1 != columns - 4
                // extension
                {
                    continue;
                }
                let x = (dimension - 1) - (2 * 2) - (j - 2) * 4;
                let y = (dimension - 1) - (i * 2) - (if i >= rows - 3 { 1 } else { 0 }); // timing
                let mut current_byte = 0;
                for b in 0..8 {
                    append_bit(
                        &mut current_byte,
                        get_data_mask_bit(
                            format_info.data_mask as u32,
                            x - b % 4,
                            y - (b / 4),
                            None,
                        )? != get_bit(
                            bit_matrix,
                            x - b % 4,
                            y - (b / 4),
                            Some(format_info.is_mirrored),
                        ),
                    );
                }
                result.push(current_byte);
            }
        }
    }

    result[0] &= 0xf; // ignore corner
    let expected_codewords = version.get_total_codewords() as usize;
    let actual_codewords = result.len();
    if actual_codewords != expected_codewords {
        return Err(Error::InvalidFormat {
            message: format!(
                "QR data is malformed: expected {expected_codewords} codewords, found {actual_codewords}"
            ),
        }
        .into());
    }

    Ok(result.iter().copied().map(|x| x as u8).collect())
}

pub fn read_rmqrcodewords(
    bit_matrix: &BitMatrix,
    version: VersionRef,
    format_info: &FormatInformation,
) -> Result<Vec<u8>> {
    let function_pattern = version.build_function_pattern()?;

    let mut result = Vec::with_capacity(version.get_total_codewords() as usize);
    let mut current_byte = 0;
    let mut reading_up = true;
    let mut bits_read = 0;
    let width = bit_matrix.width();
    let height = bit_matrix.height();
    // Read columns in pairs, from right to left. Skip right edge alignment.
    let mut x = width as i32 - 1 - 1;
    while x > 0 {
        // Read alternatingly from bottom to top then top to bottom
        for row in 0..height {
            let y = if reading_up { height - 1 - row } else { row };
            for col in 0..2 {
                let xx = x - col;
                // Ignore bits covered by the function pattern
                if !function_pattern.get(xx as u32, y) {
                    // Read a bit
                    append_bit(
                        &mut current_byte,
                        get_data_mask_bit(format_info.data_mask as u32, xx as u32, y, None)?
                            != get_bit(bit_matrix, xx as u32, y, Some(format_info.is_mirrored)),
                    );
                    // If we've made a whole byte, save it off
                    bits_read += 1;
                    if bits_read % 8 == 0 {
                        result.push(current_byte);
                        current_byte = 0;
                    }
                }
            }
        }
        reading_up = !reading_up; // switch directions

        x -= 2
    }
    let expected_codewords = version.get_total_codewords() as usize;
    let actual_codewords = result.len();
    if actual_codewords != expected_codewords {
        return Err(Error::InvalidFormat {
            message: format!(
                "QR data is malformed: expected {expected_codewords} codewords, found {actual_codewords}"
            ),
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
    match version.qr_type {
        Type::Model1 => read_qrcodewords_model1(bit_matrix, version, format_info),
        Type::Model2 => read_qrcodewords(bit_matrix, version, format_info),
        Type::Micro => read_mqrcodewords(bit_matrix, version, format_info),
        Type::RectMicro => read_rmqrcodewords(bit_matrix, version, format_info),
    }
}
