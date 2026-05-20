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

use crate::Error;
use anyhow::Result;

use crate::qrcode::common::{ErrorCorrectionLevel, VersionRef};

/// Block of data and error-correction codewords within a QR Code.
///
/// QR Codes may split their payload across multiple blocks. Each block carries
/// a unit of data codewords plus the error-correction codewords needed for it.
pub struct DataBlock {
    num_data_codewords: u32,
    codewords: Vec<u8>,
}

impl DataBlock {
    fn new(num_data_codewords: u32, codewords: Vec<u8>) -> Self {
        Self {
            num_data_codewords,
            codewords,
        }
    }

    /// When QR Codes use multiple data blocks, they are actually interleaved.
    /// That is, the first byte of data block 1 to n is written, then the second bytes, and so on. This
    /// method will separate the data into original blocks.
    ///
    /// - `raw_codewords`: bytes as read directly from the QR Code
    /// - `version`: version of the QR Code
    /// - `ec_level`: error-correction level of the QR Code
    ///
    /// Returns DataBlocks containing original bytes, "de-interleaved" from representation in the.
    /// QR Code
    pub fn get_data_blocks(
        raw_codewords: &[u8],
        version: VersionRef,
        ec_level: ErrorCorrectionLevel,
    ) -> Result<Vec<Self>> {
        if raw_codewords.len() as u32 != version.get_total_codewords() {
            return Err(Error::InvalidArgument {
                message: format!(
                    "raw codewords length {} does not match expected total codewords {}",
                    raw_codewords.len(),
                    version.get_total_codewords()
                ),
            }
            .into());
        }

        // Figure out the number and size of data blocks used by this version and
        // error correction level
        let ec_blocks = version.get_ecblocks_for_level(ec_level)?;

        // First count the total number of data blocks
        let mut _total_blocks = 0;
        let ec_block_array = ec_blocks.get_ecblocks();
        for ec_block in ec_block_array {
            _total_blocks += ec_block.get_count();
        }

        // Now establish DataBlocks of the appropriate size and number of data codewords
        let mut result = Vec::new();
        let mut num_rxing_result_blocks = 0;
        for ec_block in ec_block_array {
            for _i in 0..ec_block.get_count() {
                let num_data_codewords = ec_block.get_data_codewords();
                let num_block_codewords =
                    ec_blocks.get_eccodewords_per_block() + num_data_codewords;
                result.push(DataBlock::new(
                    num_data_codewords,
                    vec![0u8; num_block_codewords as usize],
                ));
                num_rxing_result_blocks += 1;
            }
        }

        // All blocks have the same amount of data, except that the last n
        // (where n may be 0) have 1 more byte. Figure out where these start.
        if result.is_empty() {
            return Err(Error::InvalidArgument {
                message:
                    "result block list is empty — possible data corruption or misconfiguration"
                        .to_owned(),
            }
            .into());
        }
        let shorter_blocks_total_codewords = result[0].codewords.len();
        let mut longer_blocks_start_at = result.len() - 1;
        loop {
            let num_codewords = result[longer_blocks_start_at].codewords.len();

            if num_codewords == shorter_blocks_total_codewords {
                break;
            }
            longer_blocks_start_at -= 1;
        }
        longer_blocks_start_at += 1;

        let shorter_blocks_num_data_codewords =
            shorter_blocks_total_codewords - ec_blocks.get_eccodewords_per_block() as usize;
        // The last elements of result may be 1 element longer;
        // first fill out as many elements as all of them have
        let mut raw_codewords_offset = 0;
        for i in 0..shorter_blocks_num_data_codewords {
            for result_j in result.iter_mut().take(num_rxing_result_blocks) {
                result_j.codewords[i] = raw_codewords[raw_codewords_offset];
                raw_codewords_offset += 1;
            }
        }
        // Fill out the last data block in the longer ones
        for res in result
            .iter_mut()
            .take(num_rxing_result_blocks)
            .skip(longer_blocks_start_at)
        {
            res.codewords[shorter_blocks_num_data_codewords] = raw_codewords[raw_codewords_offset];
            raw_codewords_offset += 1;
        }
        // Now add in error correction blocks
        let max = result[0].codewords.len();
        for i in shorter_blocks_num_data_codewords..max {
            for (j, res) in result.iter_mut().enumerate().take(num_rxing_result_blocks) {
                let i_offset = if j < longer_blocks_start_at { i } else { i + 1 };
                res.codewords[i_offset] = raw_codewords[raw_codewords_offset];
                raw_codewords_offset += 1;
            }
        }
        Ok(result)
    }

    pub fn get_num_data_codewords(&self) -> u32 {
        self.num_data_codewords
    }

    pub fn get_codewords(&self) -> &[u8] {
        &self.codewords
    }
}
