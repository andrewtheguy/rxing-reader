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

use std::fmt;

use crate::{Error, common::BitMatrix, qrcode::cpp_port::Type};
use anyhow::Result;

use super::{ErrorCorrectionLevel, FormatInformation};

use once_cell::sync::Lazy;

pub type VersionRef = &'static Version;

pub static VERSIONS: Lazy<Box<[Version]>> = Lazy::new(Version::build_versions);
pub static MICRO_VERSIONS: Lazy<Box<[Version]>> = Lazy::new(Version::build_micro_versions);
pub static MODEL1_VERSIONS: Lazy<Box<[Version]>> = Lazy::new(Version::build_model1_versions);
pub static RMQR_VERSIONS: Lazy<Box<[Version]>> = Lazy::new(Version::build_rmqr_versions);

/// See ISO 18004:2006 Annex D.
/// Element i represents the raw version bits that specify version i + 7
pub const VERSION_DECODE_INFO: [u32; 34] = [
    0x07C94, 0x085BC, 0x09A99, 0x0A4D3, 0x0BBF6, 0x0C762, 0x0D847, 0x0E60D, 0x0F928, 0x10B78,
    0x1145D, 0x12A17, 0x13532, 0x149A6, 0x15683, 0x168C9, 0x177EC, 0x18EC4, 0x191E1, 0x1AFAB,
    0x1B08E, 0x1CC1A, 0x1D33F, 0x1ED75, 0x1F250, 0x209D5, 0x216F0, 0x228BA, 0x2379F, 0x24B0B,
    0x2542E, 0x26A64, 0x27541, 0x28C69,
];

/// See ISO 18004:2006 Annex D
#[derive(Debug)]
pub struct Version {
    version_number: u32,
    alignment_pattern_centers: Box<[u32]>,
    ec_blocks: Box<[ECBlocks]>,
    total_codewords: u32,
    pub(crate) qr_type: Type,
}
impl Version {
    pub(super) fn new(
        version_number: u32,
        alignment_pattern_centers: Box<[u32]>,
        ec_blocks: [ECBlocks; 4],
    ) -> Self {
        let mut total = 0;
        let ec_codewords = ec_blocks[1].get_eccodewords_per_block();
        let ecb_array = ec_blocks[1].get_ecblocks();
        for ecb in ecb_array {
            total += ecb.get_count() * (ecb.get_data_codewords() + ec_codewords);
        }

        Self {
            version_number,
            alignment_pattern_centers,
            qr_type: if ec_blocks[0].get_eccodewords_per_block() != 0 {
                Type::Model2
            } else {
                Type::RectMicro
            },
            ec_blocks: Box::new(ec_blocks),
            total_codewords: total,
        }
    }

    pub(super) fn without_alignment_patterns(
        version_number: u32,
        ec_blocks: Box<[ECBlocks]>,
    ) -> Self {
        let mut total = 0;
        let ec_codewords = ec_blocks[0].get_eccodewords_per_block();
        let ecb_array = ec_blocks[0].get_ecblocks();
        for ecb_array_element in ecb_array {
            total += ecb_array_element.get_count()
                * (ecb_array_element.get_data_codewords() + ec_codewords);
        }

        let symbol_type = if ec_blocks[0].get_eccodewords_per_block() < 7
            || ec_blocks[0].get_eccodewords_per_block() == 8
        {
            Type::Micro
        } else {
            Type::Model1
        };

        Self {
            version_number,
            alignment_pattern_centers: Box::default(),
            ec_blocks,
            total_codewords: total,
            qr_type: symbol_type,
        }
    }

    pub const fn get_version_number(&self) -> u32 {
        self.version_number
    }

    pub const fn get_alignment_pattern_centers(&self) -> &[u32] {
        &self.alignment_pattern_centers
    }

    pub const fn get_total_codewords(&self) -> u32 {
        self.total_codewords
    }

    pub fn get_dimension_for_version(&self) -> u32 {
        Self::dimension_of_version(self.version_number, self.qr_type == Type::Micro)
    }

    pub fn get_ecblocks_for_level(&self, ec_level: ErrorCorrectionLevel) -> Result<&ECBlocks> {
        self.ec_blocks
            .get(ec_level.get_ordinal() as usize)
            .ok_or_else(|| {
                Error::InvalidArgument {
                    message: format!(
                        "ErrorCorrectionLevel ordinal {} out of range for {} EC blocks",
                        ec_level.get_ordinal(),
                        self.ec_blocks.len()
                    ),
                }
                .into()
            })
    }

    /// Deduces version information purely from QR Code dimensions.
    ///
    /// - `dimension`: dimension in modules
    ///
    /// Returns Version for a QR Code of that dimension.
    /// Returns an invalid-format error if dimension is not 1 mod 4 or dimension less than 17
    pub fn get_provisional_version_for_dimension(dimension: u32) -> Result<VersionRef> {
        if dimension % 4 != 1 || dimension < 21 {
            return Err(Error::InvalidFormat {
                message: format!(
                    "QR dimension {dimension} is invalid (expected >= 21 and (dimension - 1) % 4 == 0)"
                ),
            }
            .into());
        }
        Self::get_version_for_number((dimension - 17) / 4)
    }

    pub fn get_version_for_number(version_number: u32) -> Result<VersionRef> {
        if !(1..=40).contains(&version_number) {
            return Err(Error::InvalidArgument {
                message: format!(
                    "QR version {version_number} is out of spec (expected 1..=40)"
                ),
            }
            .into());
        }
        Ok(&VERSIONS[version_number as usize - 1])
    }

    pub fn decode_version_information(version_bits: u32) -> Result<VersionRef> {
        let mut best_difference = u32::MAX;
        let mut best_version = 0;
        for i in 0..VERSION_DECODE_INFO.len() as u32 {
            let target_version = VERSION_DECODE_INFO[i as usize];
            // Do the version info bits match exactly? done.
            if target_version == version_bits {
                return Self::get_version_for_number(i + 7);
            }
            // Otherwise see if this is the closest to a real version info bit string
            // we have seen so far
            let bits_difference =
                FormatInformation::num_bits_differing(version_bits, target_version);
            if bits_difference < best_difference {
                best_version = i + 7;
                best_difference = bits_difference;
            }
        }
        // We can tolerate up to 3 bits of error since no two version info codewords will
        // differ in less than 8 bits.
        if best_difference <= 3 {
            return Self::get_version_for_number(best_version);
        }
        // If we didn't find a close enough match, fail
        Err(Error::NotFound {
            message: "barcode pattern was not detected".to_owned(),
        }
        .into())
    }

    /// See ISO 18004:2006 Annex E
    pub fn build_function_pattern(&self) -> Result<BitMatrix> {
        if self.is_rmqr() {
            let size = Version::symbol_size(self.version_number, Type::RectMicro);
            let mut bit_matrix = BitMatrix::new(size.x as u32, size.y as u32)?;

            // Set edge timing patterns
            bit_matrix.set_region(0, 0, size.x as u32, 1)?; // Top
            bit_matrix.set_region(0, (size.y - 1) as u32, size.x as u32, 1)?; // Bottom
            bit_matrix.set_region(0, 1, 1, (size.y - 2) as u32)?; // Left
            bit_matrix.set_region((size.x - 1) as u32, 1, 1, (size.y - 2) as u32)?; // Right

            // Set vertical timing and alignment patterns
            let max = self.alignment_pattern_centers.len(); // Same as vertical timing column
            for x in 0..max {
                let cx = self.alignment_pattern_centers[x];
                bit_matrix.set_region(cx - 1, 1, 3, 2)?; // Top alignment pattern
                bit_matrix.set_region(cx - 1, (size.y - 3) as u32, 3, 2)?; // Bottom alignment pattern
                bit_matrix.set_region(cx, 3, 1, (size.y - 6) as u32)?; // Vertical timing pattern
            }

            // Top left finder pattern + separator
            bit_matrix.set_region(1, 1, 8 - 1, 8 - 1 - u32::from(size.y == 7))?; // R7 finder bottom flush with edge
            // Top left format
            bit_matrix.set_region(8, 1, 3, 5)?;
            bit_matrix.set_region(11, 1, 1, 3)?;

            // Bottom right finder subpattern
            bit_matrix.set_region((size.x - 5) as u32, (size.y - 5) as u32, 5 - 1, 5 - 1)?;
            // Bottom right format
            bit_matrix.set_region((size.x - 8) as u32, (size.y - 6) as u32, 3, 5)?;
            bit_matrix.set_region((size.x - 5) as u32, (size.y - 6) as u32, 3, 1)?;

            // Top right corner finder
            bit_matrix.set((size.x - 2) as u32, 1);
            if size.y > 9 {
                // Bottom left corner finder
                bit_matrix.set(1, (size.y - 2) as u32);
            }

            return Ok(bit_matrix);
        }

        let dimension = self.get_dimension_for_version();
        let mut bit_matrix = BitMatrix::with_single_dimension(dimension)?;

        // Top left finder pattern + separator + format
        bit_matrix.set_region(0, 0, 9, 9)?;

        if self.qr_type != Type::Micro {
            // Top right finder pattern + separator + format
            bit_matrix.set_region(dimension - 8, 0, 8, 9)?;
            // Bottom left finder pattern + separator + format
            bit_matrix.set_region(0, dimension - 8, 9, 8)?;

            // Alignment patterns
            let max = self.alignment_pattern_centers.len();
            for x in 0..max {
                let i = self.alignment_pattern_centers[x] - 2;
                for y in 0..max {
                    if (x != 0 || (y != 0 && y != max - 1)) && (x != max - 1 || y != 0) {
                        bit_matrix.set_region(self.alignment_pattern_centers[y] - 2, i, 5, 5)?;
                    }
                    // else no o alignment patterns near the three finder patterns
                }
            }

            // Vertical timing pattern
            bit_matrix.set_region(6, 9, 1, dimension - 17)?;
            // Horizontal timing pattern
            bit_matrix.set_region(9, 6, dimension - 17, 1)?;

            if self.version_number > 6 {
                // Version info, top right
                bit_matrix.set_region(dimension - 11, 0, 3, 6)?;
                // Version info, bottom left
                bit_matrix.set_region(0, dimension - 11, 6, 3)?;
            }
        } else {
            // Vertical timing pattern
            bit_matrix.set_region(9, 0, dimension - 9, 1)?;

            // Horizontal timing pattern
            bit_matrix.set_region(0, 9, 1, dimension - 9)?;
        }

        Ok(bit_matrix)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version_number)
    }
}

/// Encapsulates a set of error-correction blocks in one symbol version. Most versions will
/// use blocks of differing sizes within one version, so, this encapsulates the parameters for
/// each set of blocks. It also holds the number of error-correction codewords per block since it
/// will be the same across all blocks within one version.
#[derive(Debug, Clone)]
pub struct ECBlocks {
    ec_codewords_per_block: u32,
    ec_blocks: Box<[ECB]>,
}

impl ECBlocks {
    pub const fn new(ec_codewords_per_block: u32, ec_blocks: Box<[ECB]>) -> Self {
        Self {
            ec_codewords_per_block,
            ec_blocks,
        }
    }

    pub const fn get_eccodewords_per_block(&self) -> u32 {
        self.ec_codewords_per_block
    }

    pub fn get_num_blocks(&self) -> u32 {
        let mut total = 0;
        for ec_block in self.ec_blocks.iter() {
            total += ec_block.get_count();
        }
        total
    }

    pub fn get_total_eccodewords(&self) -> u32 {
        self.ec_codewords_per_block * self.get_num_blocks()
    }

    pub fn get_ecblocks(&self) -> &[ECB] {
        &self.ec_blocks
    }
}

/// Encapsulates the parameters for one error-correction block in one symbol version.
/// This includes the number of data codewords, and the number of times a block with these
/// parameters is used consecutively in the QR code version's format.
#[derive(Debug, Clone, Copy)]
pub struct ECB {
    count: u32,
    data_codewords: u32,
}

impl ECB {
    pub const fn new(count: u32, data_codewords: u32) -> Self {
        Self {
            count,
            data_codewords,
        }
    }

    pub const fn get_count(&self) -> u32 {
        self.count
    }

    pub const fn get_data_codewords(&self) -> u32 {
        self.data_codewords
    }
}
