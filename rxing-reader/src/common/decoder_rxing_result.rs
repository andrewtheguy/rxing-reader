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

use std::any::Any;

/// Encapsulates the result of decoding a matrix of bits. This typically
/// applies to 2D barcode formats. For now it contains the raw bytes obtained,
/// as well as a String interpretation of those bytes, if applicable.
pub struct DecoderRXingResult {
    raw_bytes: Vec<u8>,
    num_bits: usize,
    text: String,
    byte_segments: Vec<Vec<u8>>,
    ec_level: String,
    errors_corrected: usize,
    erasures: usize,
    other: Option<Box<dyn Any + Send + Sync>>,
    structured_append_parity: i32,
    structured_append_sequence_number: i32,
    symbology_modifier: u32,
    content_type: String,
    is_mirrored: bool,
}

impl DecoderRXingResult {
    pub fn new(
        raw_bytes: Vec<u8>,
        text: String,
        byte_segments: Vec<Vec<u8>>,
        ec_level: String,
    ) -> Self {
        Self::with_all(
            raw_bytes,
            text,
            byte_segments,
            ec_level,
            -1,
            -1,
            0,
            String::new(),
            false,
        )
    }

    pub fn with_symbology(
        raw_bytes: Vec<u8>,
        text: String,
        byte_segments: Vec<Vec<u8>>,
        ec_level: String,
        symbology_modifier: u32,
    ) -> Self {
        Self::with_all(
            raw_bytes,
            text,
            byte_segments,
            ec_level,
            -1,
            -1,
            symbology_modifier,
            String::new(),
            false,
        )
    }

    pub fn with_sa(
        raw_bytes: Vec<u8>,
        text: String,
        byte_segments: Vec<Vec<u8>>,
        ec_level: String,
        sa_sequence: i32,
        sa_parity: i32,
    ) -> Self {
        Self::with_all(
            raw_bytes,
            text,
            byte_segments,
            ec_level,
            sa_sequence,
            sa_parity,
            0,
            String::new(),
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_all(
        raw_bytes: Vec<u8>,
        text: String,
        byte_segments: Vec<Vec<u8>>,
        ec_level: String,
        sa_sequence: i32,
        sa_parity: i32,
        symbology_modifier: u32,
        content_type: String,
        is_mirrored: bool,
    ) -> Self {
        let nb = raw_bytes.len();
        Self {
            raw_bytes,
            num_bits: nb * 8,
            text,
            byte_segments,
            ec_level,
            errors_corrected: 0,
            erasures: 0,
            other: None,
            structured_append_parity: sa_parity,
            structured_append_sequence_number: sa_sequence,
            symbology_modifier,
            content_type,
            is_mirrored,
        }
    }

    /// Returns raw bytes representing the result, or `None` if not applicable.
    pub const fn get_raw_bytes(&self) -> &Vec<u8> {
        &self.raw_bytes
    }

    /// Returns how many bits of [`Self::get_raw_bytes`] are valid; typically 8 times its length.
    pub const fn get_num_bits(&self) -> usize {
        self.num_bits
    }

    /// - `num_bits`: overrides the number of bits that are valid in [`Self::get_raw_bytes`]
    pub const fn set_num_bits(&mut self, num_bits: usize) {
        self.num_bits = num_bits;
    }

    /// Returns text representation of the result.
    pub fn get_text(&self) -> &str {
        &self.text
    }

    /// Returns list of byte segments in the result, or `None` if not applicable.
    pub const fn get_byte_segments(&self) -> &Vec<Vec<u8>> {
        &self.byte_segments
    }

    /// Returns name of error correction level used, or `None` if not applicable.
    pub fn get_eclevel(&self) -> &str {
        &self.ec_level
    }

    /// Returns number of errors corrected, or `None` if not applicable.
    pub const fn get_errors_corrected(&self) -> usize {
        self.errors_corrected
    }

    pub const fn set_errors_corrected(&mut self, errors_corrected: usize) {
        self.errors_corrected = errors_corrected;
    }

    /// Returns number of erasures corrected, or `None` if not applicable.
    pub const fn get_erasures(&self) -> usize {
        self.erasures
    }

    pub const fn set_erasures(&mut self, erasures: usize) {
        self.erasures = erasures
    }

    /// Returns arbitrary additional metadata.
    pub fn get_other(&self) -> Option<&(dyn Any + Send + Sync)> {
        self.other.as_deref()
    }

    pub fn set_other(&mut self, other: Option<Box<dyn Any + Send + Sync>>) {
        self.other = other
    }

    pub const fn has_structured_append(&self) -> bool {
        self.structured_append_parity >= 0 && self.structured_append_sequence_number >= 0
    }

    pub const fn get_structured_append_parity(&self) -> i32 {
        self.structured_append_parity
    }

    pub const fn get_structured_append_sequence_number(&self) -> i32 {
        self.structured_append_sequence_number
    }

    pub const fn get_symbology_modifier(&self) -> u32 {
        self.symbology_modifier
    }

    pub fn get_content_type(&self) -> &str {
        &self.content_type
    }

    pub fn set_content_type(&mut self, content_type: String) {
        self.content_type = content_type
    }

    pub const fn get_is_mirrored(&self) -> bool {
        self.is_mirrored
    }

    pub const fn set_is_mirrored(&mut self, is_mirrored: bool) {
        self.is_mirrored = is_mirrored
    }
}
