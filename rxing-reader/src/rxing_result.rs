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

use std::{collections::HashMap, fmt};

use crate::{
    BarcodeFormat, MetadataDictionary, Point, RXingResultMetadataType, RXingResultMetadataValue,
    common::cpp_essentials::DecoderResult,
};

pub type RXingResultMetaDataDictionary = HashMap<RXingResultMetadataType, RXingResultMetadataValue>;

/**
 * <p>Encapsulates the result of decoding a barcode within an image.</p>
 *
 * @author Sean Owen
 */
#[derive(Clone, Debug, PartialEq)]
pub struct RXingResult {
    text: String,
    raw_bytes: Vec<u8>,
    num_bits: usize,
    result_points: Vec<Point>,
    format: BarcodeFormat,
    result_metadata: RXingResultMetaDataDictionary,
    timestamp: u128,
    line_count: usize,
}
impl RXingResult {
    pub fn new(
        text: &str,
        raw_bytes: Vec<u8>,
        result_points: Vec<Point>,
        format: BarcodeFormat,
    ) -> Self {
        Self::new_timestamp(
            text,
            raw_bytes,
            result_points,
            format,
            chrono::Utc::now().timestamp_millis() as u128,
        )
    }

    pub fn new_timestamp(
        text: &str,
        raw_bytes: Vec<u8>,
        result_points: Vec<Point>,
        format: BarcodeFormat,
        timestamp: u128,
    ) -> Self {
        let l = raw_bytes.len();
        Self::new_complex(text, raw_bytes, 8 * l, result_points, format, timestamp)
    }

    pub fn new_complex(
        text: &str,
        raw_bytes: Vec<u8>,
        num_bits: usize,
        result_points: Vec<Point>,
        format: BarcodeFormat,
        timestamp: u128,
    ) -> Self {
        Self {
            text: text.to_owned(),
            raw_bytes,
            num_bits,
            result_points,
            format,
            result_metadata: HashMap::new(),
            timestamp,
            line_count: 0,
        }
    }

    pub fn with_point(self, points: Vec<Point>) -> Self {
        Self {
            text: self.text,
            raw_bytes: self.raw_bytes,
            num_bits: self.num_bits,
            result_points: points,
            format: self.format,
            result_metadata: self.result_metadata,
            timestamp: self.timestamp,
            line_count: self.line_count,
        }
    }

    pub fn with_decoder_result<T>(
        res: DecoderResult<T>,
        result_points: &[Point],
        format: BarcodeFormat,
    ) -> Self
    where
        T: Copy + Clone + Default + Eq + PartialEq,
    {
        let mut new_res = Self::new(
            &res.text(),
            res.content().bytes().to_vec(),
            result_points.to_vec(),
            format,
        );

        let mut meta_data = MetadataDictionary::new();
        meta_data.insert(
            RXingResultMetadataType::ErrorCorrectionLevel,
            RXingResultMetadataValue::ErrorCorrectionLevel(res.ec_level().to_owned()),
        );
        meta_data.insert(
            RXingResultMetadataType::StructuredAppendParity,
            RXingResultMetadataValue::StructuredAppendParity(res.structured_append().count),
        );
        meta_data.insert(
            RXingResultMetadataType::StructuredAppendSequence,
            RXingResultMetadataValue::StructuredAppendSequence(res.structured_append().index),
        );
        meta_data.insert(
            RXingResultMetadataType::SymbologyIdentifier,
            RXingResultMetadataValue::SymbologyIdentifier(res.symbology_identifier()),
        );

        new_res.put_all_metadata(meta_data);

        new_res
    }

    /// Like [`with_decoder_result`] but skips the `res.text()` UTF-8 build.
    /// Use when the caller only consumes `get_raw_bytes()` — saves an
    /// `ECIStringBuilder::to_string()` per frame.
    pub fn with_decoder_result_bytes_only<T>(
        res: DecoderResult<T>,
        result_points: &[Point],
        format: BarcodeFormat,
    ) -> Self
    where
        T: Copy + Clone + Default + Eq + PartialEq,
    {
        let mut new_res = Self::new(
            "",
            res.content().bytes().to_vec(),
            result_points.to_vec(),
            format,
        );

        let mut meta_data = MetadataDictionary::new();
        meta_data.insert(
            RXingResultMetadataType::ErrorCorrectionLevel,
            RXingResultMetadataValue::ErrorCorrectionLevel(res.ec_level().to_owned()),
        );
        meta_data.insert(
            RXingResultMetadataType::StructuredAppendParity,
            RXingResultMetadataValue::StructuredAppendParity(res.structured_append().count),
        );
        meta_data.insert(
            RXingResultMetadataType::StructuredAppendSequence,
            RXingResultMetadataValue::StructuredAppendSequence(res.structured_append().index),
        );
        meta_data.insert(
            RXingResultMetadataType::SymbologyIdentifier,
            RXingResultMetadataValue::SymbologyIdentifier(res.symbology_identifier()),
        );

        new_res.put_all_metadata(meta_data);

        new_res
    }

    /**
     * @return raw text encoded by the barcode
     */
    pub fn get_text(&self) -> &str {
        &self.text
    }

    /**
     * @return raw bytes encoded by the barcode, if applicable, otherwise {@code null}
     */
    pub fn get_raw_bytes(&self) -> &[u8] {
        &self.raw_bytes
    }

    /**
     * @return how many bits of {@link #get_raw_bytes()} are valid; typically 8 times its length
     * @since 3.3.0
     */
    pub fn get_num_bits(&self) -> usize {
        self.num_bits
    }

    /**
     * @return points related to the barcode in the image. These are typically points
     *         identifying finder patterns or the corners of the barcode. The exact meaning is
     *         specific to the type of barcode that was decoded.
     */
    pub fn get_points(&self) -> &[Point] {
        &self.result_points
    }

    pub fn get_points_mut(&mut self) -> &mut [Point] {
        &mut self.result_points
    }

    /** Currently necessary because the external OneDReader proc macro uses it. */
    pub fn get_rxing_result_points(&self) -> &[Point] {
        &self.result_points
    }

    /** Currently necessary because the external OneDReader proc macro uses it. */
    pub fn get_rxing_result_points_mut(&mut self) -> &mut [Point] {
        &mut self.result_points
    }

    /**
     * @return {@link BarcodeFormat} representing the format of the barcode that was decoded
     */
    pub fn get_barcode_format(&self) -> &BarcodeFormat {
        &self.format
    }

    /**
     * @return {@link Map} mapping {@link RXingResultMetadataType} keys to values. May be
     *   {@code null}. This contains optional metadata about what was detected about the barcode,
     *   like orientation.
     */
    pub fn get_rxing_result_metadata(&self) -> &RXingResultMetaDataDictionary {
        &self.result_metadata
    }

    pub fn put_metadata(
        &mut self,
        md_type: RXingResultMetadataType,
        value: RXingResultMetadataValue,
    ) {
        self.result_metadata.insert(md_type, value);
    }

    pub fn put_all_metadata(&mut self, metadata: RXingResultMetaDataDictionary) {
        if self.result_metadata.is_empty() {
            let _ = std::mem::replace(&mut self.result_metadata, metadata);
        } else {
            for (key, value) in metadata.into_iter() {
                self.result_metadata.insert(key, value);
            }
        }
    }

    pub fn add_points(&mut self, new_points: &mut Vec<Point>) {
        if !new_points.is_empty() {
            self.result_points.append(new_points);
        }
    }

    pub fn get_timestamp(&self) -> u128 {
        self.timestamp
    }

    pub fn line_count(&self) -> usize {
        self.line_count
    }

    pub fn set_line_count(&mut self, lc: usize) {
        self.line_count = lc
    }

    pub fn replace_points(&mut self, points: Vec<Point>) {
        self.result_points = points
    }
}

impl fmt::Display for RXingResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}
