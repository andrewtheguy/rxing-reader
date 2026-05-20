/*
* Copyright 2016 Nu-book Inc.
* Copyright 2016 ZXing authors
*/
// SPDX-License-Identifier: Apache-2.0

use crate::Error;
use crate::common::BitMatrix;
use anyhow::Result;

/// Encapsulates data masks for the data bits in a QR  and micro QR code, per ISO 18004:2006 6.8.
///
/// Note that the diagram in section 6.8.1 is misleading since it indicates that i is column position
/// and j is row position. In fact, as the text says, i is row position and j is column position.
pub fn get_data_mask_bit(mask_index: u32, x: u32, y: u32, is_micro: Option<bool>) -> Result<bool> {
    let is_micro = is_micro.unwrap_or(false);
    let mut mask_index = mask_index;
    if is_micro {
        if !(0..4).contains(&mask_index) {
            return Err(Error::InvalidArgument {
                message: format!("MicroQR maskIndex {mask_index} out of range (expected 0..=3)"),
            }
            .into());
        }
        mask_index = [1, 4, 6, 7][mask_index as usize]; // map from MQR to QR indices
    }

    match mask_index {
        0 => return Ok((y + x).is_multiple_of(2)),
        1 => return Ok(y.is_multiple_of(2)),
        2 => return Ok(x.is_multiple_of(3)),
        3 => return Ok((y + x).is_multiple_of(3)),
        4 => return Ok(((y / 2) + (x / 3)).is_multiple_of(2)),
        5 => return Ok((y * x).is_multiple_of(6)),
        6 => return Ok(((y * x) % 6) < 3),
        7 => return Ok((y + x + ((y * x) % 3)).is_multiple_of(2)),
        _ => {}
    }

    Err(Error::InvalidArgument {
        message: format!("QRCode maskIndex {mask_index} out of range (expected 0..=7)"),
    }
    .into())
}

#[allow(dead_code)]
pub fn get_masked_bit(
    bit_matrix: &BitMatrix,
    x: u32,
    y: u32,
    mask_index: u32,
    is_micro: Option<bool>,
) -> Result<bool> {
    Ok(get_data_mask_bit(mask_index, x, y, is_micro)? != bit_matrix.get(x, y))
}
