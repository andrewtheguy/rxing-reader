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

use crate::{Error, common::BitMatrix};
use anyhow::Result;

use super::ErrorCorrectionLevel;

use once_cell::sync::Lazy;

pub type VersionRef = &'static Version;

pub static VERSIONS: Lazy<Box<[Version]>> = Lazy::new(Version::build_versions);

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
}
impl Version {
    pub(super) fn new(
        version_number: u32,
        alignment_pattern_centers: Box<[u32]>,
        ec_blocks: [ECBlocks; 4],
    ) -> Self {
        let mut total = 0;
        let ec_codewords = ec_blocks[1].ec_codewords_per_block();
        let ecb_array = ec_blocks[1].blocks();
        for ecb in ecb_array {
            total += ecb.count() * (ecb.data_codewords() + ec_codewords);
        }

        Self {
            version_number,
            alignment_pattern_centers,
            ec_blocks: Box::new(ec_blocks),
            total_codewords: total,
        }
    }

    pub const fn number(&self) -> u32 {
        self.version_number
    }

    pub const fn alignment_pattern_centers(&self) -> &[u32] {
        &self.alignment_pattern_centers
    }

    pub const fn total_codewords(&self) -> u32 {
        self.total_codewords
    }

    pub fn dimension(&self) -> u32 {
        Self::dimension_of_version(self.version_number)
    }

    pub fn ec_blocks_for_level(&self, ec_level: ErrorCorrectionLevel) -> Result<&ECBlocks> {
        self.ec_blocks
            .get(ec_level.ordinal() as usize)
            .ok_or_else(|| {
                Error::InvalidArgument {
                    message: format!(
                        "ErrorCorrectionLevel ordinal {} out of range for {} EC blocks",
                        ec_level.ordinal(),
                        self.ec_blocks.len()
                    ).into(),
                }
                .into()
            })
    }

    /// Deduces version information purely from QR Code dimensions.
    ///
    /// - `dimension`: dimension in modules
    ///
    /// Returns Version for a QR Code of that dimension.
    /// Version 1 has dimension 21. Returns an invalid-format error if
    /// dimension is less than 21 or (dimension - 1) % 4 != 0.
    pub fn provisional_for_dimension(dimension: u32) -> Result<VersionRef> {
        if dimension % 4 != 1 || dimension < 21 {
            return Err(Error::InvalidFormat {
                message: format!(
                    "QR dimension {dimension} is invalid (expected >= 21 and (dimension - 1) % 4 == 0)"
                )
                .into(),
            }
            .into());
        }
        Self::for_number((dimension - 17) / 4)
    }

    pub fn for_number(version_number: u32) -> Result<VersionRef> {
        if !(1..=40).contains(&version_number) {
            return Err(Error::InvalidArgument {
                message: format!("QR version {version_number} is out of spec (expected 1..=40)").into(),
            }
            .into());
        }
        Ok(&VERSIONS[version_number as usize - 1])
    }

    /// See ISO 18004:2006 Annex E
    pub fn build_function_pattern(&self) -> Result<BitMatrix> {
        let dimension = self.dimension();
        let mut bit_matrix = BitMatrix::with_single_dimension(dimension)?;

        // Top left finder pattern + separator + format
        bit_matrix.set_region(0, 0, 9, 9)?;

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
                // else no alignment patterns near the three finder patterns
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
    ec_blocks: Box<[Ecb]>,
}

impl ECBlocks {
    pub const fn new(ec_codewords_per_block: u32, ec_blocks: Box<[Ecb]>) -> Self {
        Self {
            ec_codewords_per_block,
            ec_blocks,
        }
    }

    pub const fn ec_codewords_per_block(&self) -> u32 {
        self.ec_codewords_per_block
    }

    pub fn num_blocks(&self) -> u32 {
        let mut total = 0;
        for ec_block in self.ec_blocks.iter() {
            total += ec_block.count();
        }
        total
    }

    pub fn total_ec_codewords(&self) -> u32 {
        self.ec_codewords_per_block * self.num_blocks()
    }

    pub fn blocks(&self) -> &[Ecb] {
        &self.ec_blocks
    }
}

/// Encapsulates the parameters for one error-correction block in one symbol version.
/// This includes the number of data codewords, and the number of times a block with these
/// parameters is used consecutively in the QR code version's format.
#[derive(Debug, Clone, Copy)]
pub struct Ecb {
    count: u32,
    data_codewords: u32,
}

impl Ecb {
    pub const fn new(count: u32, data_codewords: u32) -> Self {
        Self {
            count,
            data_codewords,
        }
    }

    pub const fn count(&self) -> u32 {
        self.count
    }

    pub const fn data_codewords(&self) -> u32 {
        self.data_codewords
    }
}
