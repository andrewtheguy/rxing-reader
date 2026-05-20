/*
* Copyright 2016 Nu-book Inc.
* Copyright 2016 ZXing authors
*/
// SPDX-License-Identifier: Apache-2.0

use crate::Error;
use anyhow::Result;

/// Encapsulates data masks for the data bits in a QR code, per ISO 18004:2006 6.8.
///
/// Note that the diagram in section 6.8.1 is misleading since it indicates that i is column position
/// and j is row position. In fact, as the text says, i is row position and j is column position.
pub fn get_data_mask_bit(mask_index: u32, x: u32, y: u32) -> Result<bool> {
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

