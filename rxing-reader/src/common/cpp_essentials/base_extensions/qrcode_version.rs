/*
* Copyright 2016 Nu-book Inc.
* Copyright 2016 ZXing authors
* Copyright 2023 Axel Waggershauser
*/
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

use crate::common::BitMatrix;
use crate::qrcode::{VERSION_DECODE_INFO, VERSIONS, Version, VersionRef};
use crate::{Error, PointI, point};

impl Version {
    pub fn model2(version_number: u32) -> Result<VersionRef> {
        if !(1..=40).contains(&version_number) {
            Err(Error::InvalidArgument {
                message: format!(
                    "Version::model2: version_number {version_number} out of range (expected 1..=40)"
                ),
            }
            .into())
        } else {
            Ok(&VERSIONS[version_number as usize - 1])
        }
    }

    pub const fn dimension_of_version(version: u32) -> u32 {
        17 + 4 * version
    }

    pub fn decode_version_information_pair(
        version_bits_a: i32,
        version_bits_b: i32,
    ) -> Result<VersionRef> {
        let mut best_difference = u32::MAX;
        let mut best_version = 0;
        for (i, target_version) in VERSION_DECODE_INFO.into_iter().enumerate() {
            for bits in [version_bits_a, version_bits_b] {
                let bits_difference = ((bits as u32) ^ target_version).count_ones();
                if bits_difference < best_difference {
                    best_version = i + 7;
                    best_difference = bits_difference;
                }
            }
            if best_difference == 0 {
                break;
            }
        }
        // We can tolerate up to 3 bits of error since no two version info codewords will
        // differ in less than 8 bits.
        if best_difference <= 3 {
            return Self::get_version_for_number(best_version as u32);
        }
        // If we didn't find a close enough match, fail
        Err(Error::InvalidState {
            message: "required internal state is missing".to_owned(),
        }
        .into())
    }

    pub fn symbol_size(version: u32) -> PointI {
        let version = version as i32;

        let square = |s: i32| point(s, s);
        let valid = |v: i32, max: i32| v >= 1 && v <= max;

        if valid(version, 40) {
            square(17 + 4 * version)
        } else {
            PointI::default()
        }
    }

    pub fn is_valid_size(size: PointI) -> bool {
        size.x == size.y && size.x >= 21 && size.x <= 177 && (size.x % 4 == 1)
    }

    pub fn has_valid_size(matrix: &BitMatrix) -> bool {
        Self::is_valid_size(point(matrix.width() as i32, matrix.height() as i32))
    }

    pub fn number_point(size: PointI) -> u32 {
        if Self::is_valid_size(size) {
            ((size.x - 17) / 4) as u32
        } else {
            0
        }
    }

    pub fn number(bit_matrix: &BitMatrix) -> u32 {
        Self::number_point(point(bit_matrix.width() as i32, bit_matrix.height() as i32))
    }
}
