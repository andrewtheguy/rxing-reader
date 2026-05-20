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

use anyhow::Result;

use crate::{Error, common::BitMatrix};

use super::DataMask;
use crate::qrcode::common::{FormatInformation, Version, VersionRef};

/**
 * @author Sean Owen
 */
pub struct BitMatrixParser {
    bit_matrix: BitMatrix,
    parsed_version: Option<VersionRef>,
    parsed_format_info: Option<FormatInformation>,
    mirror: bool,
}

impl BitMatrixParser {
    /**
     * @param bit_matrix {@link BitMatrix} to parse
     * Returns an invalid-format error if dimension is not >= 21 and 1 mod 4
     */
    pub fn new(bit_matrix: BitMatrix) -> Result<Self> {
        let dimension = bit_matrix.get_height();
        if dimension < 21 || (dimension & 0x03) != 1 {
            Err(
                Error::InvalidFormat { message: format!("{dimension} < 21 || ({dimension} & 0x03) != 1") }
                    .into(),
            )
        } else {
            Ok(Self {
                bit_matrix,
                parsed_version: None,
                parsed_format_info: None,
                mirror: false,
            })
        }
    }

    /**
     * <p>Reads format information from one of its two locations within the QR Code.</p>
     *
     * @return {@link FormatInformation} encapsulating the QR Code's format info
     * Returns an invalid-format error if both format information locations cannot be parsed as
     * the valid encoding of format information
     */
    pub fn read_format_information(&mut self) -> Result<&FormatInformation> {
        if self.parsed_format_info.is_some() {
            return self
                .parsed_format_info
                .as_ref()
                .ok_or_else(|| Error::Parse { message: "required parsed value was missing".to_owned() }.into());
        }

        // Read top-left format info bits
        let mut format_info_bits1 = 0;
        for i in 0..6 {
            format_info_bits1 = self.copy_bit(i, 8, format_info_bits1);
        }
        // .. and skip a bit in the timing pattern ...
        format_info_bits1 = self.copy_bit(7, 8, format_info_bits1);
        format_info_bits1 = self.copy_bit(8, 8, format_info_bits1);
        format_info_bits1 = self.copy_bit(8, 7, format_info_bits1);
        // .. and skip a bit in the timing pattern ...
        for j in (0..=5).rev() {
            format_info_bits1 = self.copy_bit(8, j, format_info_bits1);
        }

        // Read the top-right/bottom-left pattern too
        let dimension = self.bit_matrix.get_height();
        let mut format_info_bits2 = 0;
        let j_min = dimension - 7;
        for j in (j_min..=dimension - 1).rev() {
            format_info_bits2 = self.copy_bit(8, j, format_info_bits2);
        }
        for i in (dimension - 8)..dimension {
            format_info_bits2 = self.copy_bit(i, 8, format_info_bits2);
        }

        self.parsed_format_info =
            FormatInformation::decode_format_information(format_info_bits1, format_info_bits2);

        self.parsed_format_info
            .as_ref()
            .ok_or_else(|| Error::InvalidFormat { message: "QR data is malformed".to_owned() }.into())
    }

    /**
     * <p>Reads version information from one of its two locations within the QR Code.</p>
     *
     * @return {@link Version} encapsulating the QR Code's version
     * Returns an invalid-format error if both version information locations cannot be parsed as
     * the valid encoding of version information
     */
    pub fn read_version(&mut self) -> Result<VersionRef> {
        if let Some(pv) = self.parsed_version {
            return Ok(pv);
        }

        let dimension = self.bit_matrix.get_height();

        let provisional_version = (dimension - 17) / 4;
        if provisional_version <= 6 {
            return Version::get_version_for_number(provisional_version);
        }

        // Read top-right version info: 3 wide by 6 tall
        let mut version_bits = 0;
        let ij_min = dimension - 11;
        for j in (0..=5).rev() {
            for i in (ij_min..(dimension - 8)).rev() {
                version_bits = self.copy_bit(i, j, version_bits);
            }
        }

        if let Ok(the_parsed_version) = Version::decode_version_information(version_bits)
            && the_parsed_version.get_dimension_for_version() == dimension
        {
            self.parsed_version = Some(the_parsed_version);
            return Ok(the_parsed_version);
        }

        // Hmm, failed. Try bottom left: 6 wide by 3 tall
        version_bits = 0;
        for i in (0..=5).rev() {
            for j in (ij_min..(dimension - 8)).rev() {
                version_bits = self.copy_bit(i, j, version_bits);
            }
        }

        if let Ok(the_parsed_version) = Version::decode_version_information(version_bits)
            && the_parsed_version.get_dimension_for_version() == dimension
        {
            self.parsed_version = Some(the_parsed_version);
            return Ok(the_parsed_version);
        }
        Err(Error::InvalidFormat { message: "QR data is malformed".to_owned() }.into())
    }

    fn copy_bit(&self, i: u32, j: u32, version_bits: u32) -> u32 {
        let bit = if self.mirror {
            self.bit_matrix.get(j, i)
        } else {
            self.bit_matrix.get(i, j)
        };

        if bit {
            (version_bits << 1) | 0x1
        } else {
            version_bits << 1
        }
    }

    /**
     * <p>Reads the bits in the {@link BitMatrix} representing the finder pattern in the
     * correct order in order to reconstruct the codewords bytes contained within the
     * QR Code.</p>
     *
     * @return bytes encoded within the QR Code
     * Returns an invalid-format error if the exact number of bytes expected is not read
     */
    pub fn read_codewords(&mut self) -> Result<Vec<u8>> {
        let version = self.read_version()?;

        // Get the data mask for the format used in this QR Code. This will exclude
        // some bits from reading as we wind through the bit matrix.
        let data_mask: DataMask = self.read_format_information()?.get_data_mask().try_into()?;
        let dimension = self.bit_matrix.get_height();
        data_mask.unmask_bit_matrix(&mut self.bit_matrix, dimension);

        let function_pattern = version.build_function_pattern()?;

        let mut reading_up = true;
        let mut result = vec![0u8; version.get_total_codewords() as usize];
        let mut result_offset = 0;
        let mut current_byte = 0;
        let mut bits_read = 0;
        // Read columns in pairs, from right to left
        let mut j = dimension as i32 - 1;
        while j > 0 {
            if j == 6 {
                // Skip whole column with vertical alignment pattern;
                // saves time and makes the other code proceed more cleanly
                j -= 1;
            }
            // Read alternatingly from bottom to top then top to bottom
            for count in 0..dimension {
                let i = if reading_up {
                    dimension - 1 - count
                } else {
                    count
                };
                for col in 0..2 {
                    // Ignore bits covered by the function pattern
                    if !function_pattern.get(j as u32 - col, i) {
                        // Read a bit
                        bits_read += 1;
                        current_byte <<= 1;
                        if self.bit_matrix.get(j as u32 - col, i) {
                            current_byte |= 1;
                        }
                        // If we've made a whole byte, save it off
                        if bits_read == 8 {
                            result[result_offset] = current_byte;
                            result_offset += 1;
                            bits_read = 0;
                            current_byte = 0;
                        }
                    }
                }
            }
            reading_up ^= true; // reading_up = !reading_up; // switch directions

            j -= 2;
        }

        if result_offset != version.get_total_codewords() as usize {
            return Err(Error::InvalidFormat { message: "QR data is malformed".to_owned() }.into());
        }
        Ok(result)
    }

    /**
     * Revert the mask removal done while reading the code words. The bit matrix should revert to its original state.
     */
    pub fn remask(&mut self) -> Result<()> {
        if let Some(pfi) = &self.parsed_format_info {
            let data_mask: DataMask = pfi.get_data_mask().try_into()?;
            let dimension = self.bit_matrix.get_height();
            data_mask.unmask_bit_matrix(&mut self.bit_matrix, dimension);
        } else {
            // We have no format information, and have no data mask
        }
        Ok(())
    }

    /**
     * Prepare the parser for a mirrored operation.
     * This flag has effect only on the {@link #read_format_information()} and the
     * {@link #read_version()}. Before proceeding with {@link #read_codewords()} the
     * {@link #mirror()} method should be called.
     *
     * @param mirror Whether to read version and format information mirrored.
     */
    pub fn set_mirror(&mut self, mirror: bool) {
        self.parsed_version = None;
        self.parsed_format_info = None;
        self.mirror = mirror;
    }

    /** Mirror the bit matrix in order to attempt a second reading. */
    pub fn mirror(&mut self) {
        for x in 0..self.bit_matrix.get_width() {
            for y in (x + 1)..self.bit_matrix.get_height() {
                if self.bit_matrix.get(x, y) != self.bit_matrix.get(y, x) {
                    self.bit_matrix.flip_coords(y, x);
                    self.bit_matrix.flip_coords(x, y);
                }
            }
        }
    }
}
