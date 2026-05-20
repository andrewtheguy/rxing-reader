/*
* Copyright 2016 Nu-book Inc.
* Copyright 2016 ZXing authors
* Copyright 2023 Axel Waggershauser
*/
// SPDX-License-Identifier: Apache-2.0

use num::Integer;

use anyhow::Result;

use crate::common::BitMatrix;
use crate::qrcode::common::{
    MICRO_VERSIONS, MODEL1_VERSIONS, RMQR_VERSIONS, VERSION_DECODE_INFO, VERSIONS, Version,
    VersionRef,
};
use crate::qrcode::cpp_port::Type;
use crate::{Error, PointI, point};

const RMQR_SIZES: [PointI; 32] = [
    point(43, 7),
    point(59, 7),
    point(77, 7),
    point(99, 7),
    point(139, 7),
    point(43, 9),
    point(59, 9),
    point(77, 9),
    point(99, 9),
    point(139, 9),
    point(27, 11),
    point(43, 11),
    point(59, 11),
    point(77, 11),
    point(99, 11),
    point(139, 11),
    point(27, 13),
    point(43, 13),
    point(59, 13),
    point(77, 13),
    point(99, 13),
    point(139, 13),
    point(43, 15),
    point(59, 15),
    point(77, 15),
    point(99, 15),
    point(139, 15),
    point(43, 17),
    point(59, 17),
    point(77, 17),
    point(99, 17),
    point(139, 17),
];

impl Version {
    pub fn model1(version_number: u32) -> Result<VersionRef> {
        if !(1..=14).contains(&version_number) {
            Err(Error::InvalidArgument {
                message: "argument is out of range".to_owned(),
            }
            .into())
        } else {
            Ok(&MODEL1_VERSIONS[version_number as usize - 1])
        }
    }

    pub fn model2(version_number: u32) -> Result<VersionRef> {
        if !(1..=40).contains(&version_number) {
            Err(Error::InvalidArgument {
                message: "argument is out of range".to_owned(),
            }
            .into())
        } else {
            Ok(&VERSIONS[version_number as usize - 1])
        }
    }

    pub fn micro(version_number: u32) -> Result<VersionRef> {
        if !(1..=4).contains(&version_number) {
            Err(Error::InvalidArgument {
                message: "argument is out of range".to_owned(),
            }
            .into())
        } else {
            Ok(&MICRO_VERSIONS[version_number as usize - 1])
        }
    }

    pub fn r_mqr(version_number: u32) -> Result<VersionRef> {
        let version_number = version_number as usize;
        if version_number < 1 || version_number > (RMQR_VERSIONS.len()) {
            Err(Error::InvalidArgument {
                message: "argument is out of range".to_owned(),
            }
            .into())
        } else {
            Ok(&RMQR_VERSIONS[version_number - 1])
        }
    }

    pub const fn dimension_of_version(version: u32, is_micro: bool) -> u32 {
        Self::dimension_offset(is_micro) + Self::dimension_step(is_micro) * version
    }

    pub const fn dimension_offset(is_micro: bool) -> u32 {
        match is_micro {
            true => 9,
            false => 17,
        }
    }

    pub const fn dimension_step(is_micro: bool) -> u32 {
        match is_micro {
            true => 2,
            false => 4,
        }
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

    pub const fn is_micro(&self) -> bool {
        Type::const_eq(self.qr_type, Type::Micro)
    }

    pub const fn is_model1(&self) -> bool {
        Type::const_eq(self.qr_type, Type::Model1)
    }

    pub const fn is_model2(&self) -> bool {
        Type::const_eq(self.qr_type, Type::Model2)
    }

    pub const fn is_rmqr(&self) -> bool {
        Type::const_eq(self.qr_type, Type::RectMicro)
    }

    pub fn symbol_size(version: u32, qr_type: Type) -> PointI {
        let version = version as i32;

        let square = |s: i32| point(s, s);
        let valid = |v: i32, max: i32| v >= 1 && v <= max;

        match qr_type {
            Type::Model1 => {
                if valid(version, 14) {
                    square(17 + 4 * version)
                } else {
                    PointI::default()
                }
            }
            Type::Model2 => {
                if valid(version, 40) {
                    square(17 + 4 * version)
                } else {
                    PointI::default()
                }
            }
            Type::Micro => {
                if valid(version, 4) {
                    square(9 + 2 * version)
                } else {
                    PointI::default()
                }
            }
            Type::RectMicro => {
                if valid(version, 32) {
                    RMQR_SIZES[(version - 1) as usize]
                } else {
                    PointI::default()
                }
            }
        }
    }

    pub fn is_valid_size(size: PointI, qr_type: Type) -> bool {
        match qr_type {
            Type::Model1 => size.x == size.y && size.x >= 21 && size.x <= 73 && (size.x % 4 == 1),
            Type::Model2 => size.x == size.y && size.x >= 21 && size.x <= 177 && (size.x % 4 == 1),
            Type::Micro => size.x == size.y && size.x >= 11 && size.x <= 17 && (size.x % 2 == 1),
            Type::RectMicro => {
                size.x != size.y
                    && size.x.is_odd()
                    && size.y.is_odd()
                    && size.x >= 27
                    && size.x <= 139
                    && size.y >= 7
                    && size.y <= 17
                    && Self::index_of(&RMQR_SIZES, size).is_some()
            }
        }
    }
    pub fn has_valid_size_type(bit_matrix: &BitMatrix, qr_type: Type) -> bool {
        Self::is_valid_size(
            point(bit_matrix.width() as i32, bit_matrix.height() as i32),
            qr_type,
        )
    }

    pub fn has_valid_size(matrix: &BitMatrix) -> bool {
        Self::has_valid_size_type(matrix, Type::Model1)
            || Self::has_valid_size_type(matrix, Type::Model2)
            || Self::has_valid_size_type(matrix, Type::Micro)
            || Self::has_valid_size_type(matrix, Type::RectMicro)
    }

    fn index_of(points: &[PointI], search: PointI) -> Option<usize> {
        points.iter().position(|p| *p == search)
    }

    pub fn number_point(size: PointI) -> u32 {
        if size.x != size.y {
            Self::index_of(&RMQR_SIZES, size)
                .map(|idx| (idx + 1) as u32)
                .unwrap_or(0)
        } else if Self::is_valid_size(size, Type::Model2) {
            ((size.x - 17) / 4) as u32
        } else if Self::is_valid_size(size, Type::Micro) {
            ((size.x - 9) / 2) as u32
        } else {
            0
        }
    }

    pub fn number(bit_matrix: &BitMatrix) -> u32 {
        Self::number_point(point(bit_matrix.width() as i32, bit_matrix.height() as i32))
    }
}
