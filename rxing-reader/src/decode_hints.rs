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

use std::collections::{HashMap, HashSet};

use crate::{BarcodeFormat, PointCallback};

/// Identifies a hint that can be passed to a barcode reader.
///
/// Hints let callers trade speed for accuracy, restrict the expected formats,
/// provide character-set information, or request callbacks while decoding a
/// [`crate::BinaryBitmap`]. Each reader decides which hints it understands.
#[derive(Eq, PartialEq, Hash, Debug, Clone, Copy)]
pub enum DecodeHintType {
    /// Unspecified, application-specific hint. Maps to an unspecified value.
    OTHER,

    /// Image is known to be of one of a few possible formats.
    /// Maps to a [`Vec`] of [`BarcodeFormat`]s.
    PossibleFormats,

    /// Spend more time looking for a barcode; optimize for accuracy rather than speed.
    /// The associated value is a boolean flag.
    TryHarder,

    /// Character encoding to use when decoding, where applicable.
    CharacterSet,

    /// Allowed encoded-data lengths; reject anything else.
    AllowedLengths,

    /// Assume Code 39 symbols include a check digit.
    AssumeCode39CheckDigit,

    /// Treat the barcode as GS1 data and adjust format-specific behavior as needed.
    /// For example, this affects FNC1 handling for Code 128 (GS1-128).
    AssumeGs1,

    /// Return the alphabetic Codabar start/end guards instead of stripping them.
    ReturnCodabarStartEnd,

    /// The caller needs to be notified via callback when a possible [`crate::Point`]
    /// is found. Maps to a [`PointCallback`].
    NeedResultPointCallback,

    /// Allowed extension lengths for EAN or UPC barcodes. Other formats will ignore this.
    /// The value contains allowed extension lengths such as `[2]`, `[5]`, or `[2, 5]`.
    /// If it is optional to have an extension, do not set this hint. If this is set,
    /// and a UPC or EAN barcode is found but an extension is not, then no result will be returned
    /// at all.
    AllowedEanExtensions,

    /// Translate ASCII values parsed by the Telepen reader into Telepen Numeric form.
    TelepenAsNumeric,
}

pub enum DecodeHintValue {
    /// Unspecified, application-specific hint. Maps to an unspecified value.
    Other(String),

    /// Image is known to be of one of a few possible formats.
    /// Maps to a [`Vec`] of [`BarcodeFormat`]s.
    PossibleFormats(HashSet<BarcodeFormat>),

    /// Spend more time looking for a barcode; optimize for accuracy rather than speed.
    TryHarder(bool),

    /// Character encoding to use when decoding, where applicable.
    CharacterSet(String),

    /// Allowed encoded-data lengths; reject anything else.
    AllowedLengths(Vec<u32>),

    /// Assume Code 39 symbols include a check digit.
    AssumeCode39CheckDigit(bool),

    /// Treat the barcode as GS1 data and adjust format-specific behavior as needed.
    /// For example, this affects FNC1 handling for Code 128 (GS1-128).
    AssumeGs1(bool),

    /// Return the alphabetic Codabar start/end guards instead of stripping them.
    ReturnCodabarStartEnd(bool),

    /// The caller needs to be notified via callback when a possible [`crate::Point`]
    /// is found. Maps to a [`PointCallback`].
    NeedResultPointCallback(PointCallback),

    /// Allowed extension lengths for EAN or UPC barcodes. Other formats will ignore this.
    /// The value contains allowed extension lengths such as `[2]`, `[5]`, or `[2, 5]`.
    /// If it is optional to have an extension, do not set this hint. If this is set,
    /// and a UPC or EAN barcode is found but an extension is not, then no result will be returned
    /// at all.
    AllowedEanExtensions(Vec<u32>),

    /// Translate ASCII values parsed by the Telepen reader into Telepen Numeric form.
    TelepenAsNumeric(bool),
}

#[derive(Default)]
pub struct DecodeHints {
    /// Unspecified, application-specific hint. Maps to an unspecified value.
    pub other: Option<String>,

    /// Image is known to be of one of a few possible formats.
    /// Maps to a [`Vec`] of [`BarcodeFormat`]s.
    pub possible_formats: Option<HashSet<BarcodeFormat>>,

    /// Spend more time looking for a barcode; optimize for accuracy rather than speed.
    pub try_harder: Option<bool>,

    /// Character encoding to use when decoding, where applicable.
    pub character_set: Option<String>,

    /// Allowed encoded-data lengths; reject anything else.
    pub allowed_lengths: Option<Vec<u32>>,

    /// Assume Code 39 symbols include a check digit.
    pub assume_code_39_check_digit: Option<bool>,

    /// Treat the barcode as GS1 data and adjust format-specific behavior as needed.
    /// For example, this affects FNC1 handling for Code 128 (GS1-128).
    pub assume_gs1: Option<bool>,

    /// Return the alphabetic Codabar start/end guards instead of stripping them.
    pub return_codabar_start_end: Option<bool>,

    /// The caller needs to be notified via callback when a possible [`crate::Point`]
    /// is found. Maps to a [`PointCallback`].
    pub need_result_point_callback: Option<PointCallback>,

    /// Allowed extension lengths for EAN or UPC barcodes. Other formats will ignore this.
    /// The value contains allowed extension lengths such as `[2]`, `[5]`, or `[2, 5]`.
    /// If it is optional to have an extension, do not set this hint. If this is set,
    /// and a UPC or EAN barcode is found but an extension is not, then no result will be returned
    /// at all.
    pub allowed_ean_extensions: Option<Vec<u32>>,

    /// Translate ASCII values parsed by the Telepen reader into Telepen Numeric form.
    pub telepen_as_numeric: Option<bool>,
}

impl From<super::DecodingHintDictionary> for DecodeHints {
    fn from(value: super::DecodingHintDictionary) -> Self {
        let mut new_self: Self = Self::default();
        for (_, v) in value.into_iter() {
            match v {
                DecodeHintValue::Other(v) => new_self.other = Some(v),
                DecodeHintValue::PossibleFormats(v) => new_self.possible_formats = Some(v),
                DecodeHintValue::TryHarder(v) => new_self.try_harder = Some(v),
                DecodeHintValue::CharacterSet(v) => new_self.character_set = Some(v),
                DecodeHintValue::AllowedLengths(v) => new_self.allowed_lengths = Some(v),
                DecodeHintValue::AssumeCode39CheckDigit(v) => {
                    new_self.assume_code_39_check_digit = Some(v)
                }
                DecodeHintValue::AssumeGs1(v) => new_self.assume_gs1 = Some(v),
                DecodeHintValue::ReturnCodabarStartEnd(v) => {
                    new_self.return_codabar_start_end = Some(v)
                }
                DecodeHintValue::NeedResultPointCallback(v) => {
                    new_self.need_result_point_callback = Some(v)
                }
                DecodeHintValue::AllowedEanExtensions(v) => {
                    new_self.allowed_ean_extensions = Some(v)
                }
                DecodeHintValue::TelepenAsNumeric(v) => new_self.telepen_as_numeric = Some(v),
            }
        }
        new_self
    }
}

impl From<DecodeHints> for super::DecodingHintDictionary {
    fn from(value: DecodeHints) -> Self {
        let mut new_self = HashMap::default();

        if let Some(v) = value.other {
            new_self.insert(DecodeHintType::OTHER, DecodeHintValue::Other(v));
        }

        if let Some(v) = value.possible_formats {
            new_self.insert(
                DecodeHintType::PossibleFormats,
                DecodeHintValue::PossibleFormats(v),
            );
        }

        if let Some(v) = value.try_harder {
            new_self.insert(DecodeHintType::TryHarder, DecodeHintValue::TryHarder(v));
        }

        if let Some(v) = value.character_set {
            new_self.insert(
                DecodeHintType::CharacterSet,
                DecodeHintValue::CharacterSet(v),
            );
        }

        if let Some(v) = value.allowed_lengths {
            new_self.insert(
                DecodeHintType::AllowedLengths,
                DecodeHintValue::AllowedLengths(v),
            );
        }

        if let Some(v) = value.assume_code_39_check_digit {
            new_self.insert(
                DecodeHintType::AssumeCode39CheckDigit,
                DecodeHintValue::AssumeCode39CheckDigit(v),
            );
        }

        if let Some(v) = value.assume_gs1 {
            new_self.insert(DecodeHintType::AssumeGs1, DecodeHintValue::AssumeGs1(v));
        }

        if let Some(v) = value.return_codabar_start_end {
            new_self.insert(
                DecodeHintType::ReturnCodabarStartEnd,
                DecodeHintValue::ReturnCodabarStartEnd(v),
            );
        }

        if let Some(v) = value.need_result_point_callback {
            new_self.insert(
                DecodeHintType::NeedResultPointCallback,
                DecodeHintValue::NeedResultPointCallback(v),
            );
        }

        if let Some(v) = value.allowed_ean_extensions {
            new_self.insert(
                DecodeHintType::AllowedEanExtensions,
                DecodeHintValue::AllowedEanExtensions(v),
            );
        }

        if let Some(v) = value.telepen_as_numeric {
            new_self.insert(
                DecodeHintType::TelepenAsNumeric,
                DecodeHintValue::TelepenAsNumeric(v),
            );
        }

        new_self
    }
}

impl DecodeHints {
    pub fn with(mut self, value: DecodeHintValue) -> Self {
        match value {
            DecodeHintValue::Other(v) => self.other = Some(v),
            DecodeHintValue::PossibleFormats(v) => self.possible_formats = Some(v),
            DecodeHintValue::TryHarder(v) => self.try_harder = Some(v),
            DecodeHintValue::CharacterSet(v) => self.character_set = Some(v),
            DecodeHintValue::AllowedLengths(v) => self.allowed_lengths = Some(v),
            DecodeHintValue::AssumeCode39CheckDigit(v) => self.assume_code_39_check_digit = Some(v),
            DecodeHintValue::AssumeGs1(v) => self.assume_gs1 = Some(v),
            DecodeHintValue::ReturnCodabarStartEnd(v) => self.return_codabar_start_end = Some(v),
            DecodeHintValue::NeedResultPointCallback(v) => {
                self.need_result_point_callback = Some(v)
            }
            DecodeHintValue::AllowedEanExtensions(v) => self.allowed_ean_extensions = Some(v),
            DecodeHintValue::TelepenAsNumeric(v) => self.telepen_as_numeric = Some(v),
        }
        self
    }
}
