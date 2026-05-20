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

/**
 * Encapsulates a type of hint that a caller may pass to a barcode reader to help it
 * more quickly or accurately decode it. It is up to implementations to decide what,
 * if anything, to do with the information that is supplied.
 *
 * @author Sean Owen
 * @author dswitkin@google.com (Daniel Switkin)
 * @see Reader#decode(BinaryBitmap,java.util.Map)
 */
#[derive(Eq, PartialEq, Hash, Debug, Clone, Copy)]
pub enum DecodeHintType {
    /**
     * Unspecified, application-specific hint. Maps to an unspecified {@link Object}.
     */
    OTHER,

    /**
     * Image is known to be of one of a few possible formats.
     * Maps to a {@link List} of {@link BarcodeFormat}s.
     */
    PossibleFormats,

    /**
     * Spend more time to try to find a barcode; optimize for accuracy, not speed.
     * Doesn't matter what it maps to; use {@link Boolean#TRUE}.
     */
    TryHarder,

    /**
     * Specifies what character encoding to use when decoding, where applicable (type String)
     */
    CharacterSet,

    /**
     * Allowed lengths of encoded data -- reject anything else. Maps to an {@code int[]}.
     */
    AllowedLengths,

    /**
     * Assume Code 39 codes employ a check digit. Doesn't matter what it maps to;
     * use {@link Boolean#TRUE}.
     */
    AssumeCode39CheckDigit,

    /**
     * Assume the barcode is being processed as a GS1 barcode, and modify behavior as needed.
     * For example this affects FNC1 handling for Code 128 (aka GS1-128). Doesn't matter what it maps to;
     * use {@link Boolean#TRUE}.
     */
    AssumeGs1,

    /**
     * If true, return the start and end digits in a Codabar barcode instead of stripping them. They
     * are alpha, whereas the rest are numeric. By default, they are stripped, but this causes them
     * to not be. Doesn't matter what it maps to; use {@link Boolean#TRUE}.
     */
    ReturnCodabarStartEnd,

    /**
     * The caller needs to be notified via callback when a possible {@link Point}
     * is found. Maps to a {@link PointCallback}.
     */
    NeedResultPointCallback,

    /**
     * Allowed extension lengths for EAN or UPC barcodes. Other formats will ignore this.
     * Maps to an {@code int[]} of the allowed extension lengths, for example [2], [5], or [2, 5].
     * If it is optional to have an extension, do not set this hint. If this is set,
     * and a UPC or EAN barcode is found but an extension is not, then no result will be returned
     * at all.
     */
    AllowedEanExtensions,

    /*
     * Will translate the ASCII values parsed by the Telepen reader into the Telepen Numeric form.
     */
    TelepenAsNumeric,
    /*
     * Data type the hint is expecting.
     * Among the possible values the {@link Void} stands out as being used for
     * hints that do not expect a value to be supplied (flag hints). Such hints
     * will possibly have their value ignored, or replaced by a
     * {@link Boolean#TRUE}. Hint suppliers should probably use
     * {@link Boolean#TRUE} as directed by the actual hint documentation.
     */
    /*
    private final Class<?> valueType;

    DecodeHintType(Class<?> valueType) {
      this.valueType = valueType;
    }

    public Class<?> getValueType() {
      return valueType;
    }*/
}

#[derive(Clone)]
pub enum DecodeHintValue {
    /**
     * Unspecified, application-specific hint. Maps to an unspecified {@link Object}.
     */
    Other(String),

    /**
     * Image is known to be of one of a few possible formats.
     * Maps to a {@link List} of {@link BarcodeFormat}s.
     */
    PossibleFormats(HashSet<BarcodeFormat>),

    /**
     * Spend more time to try to find a barcode; optimize for accuracy, not speed.
     * Doesn't matter what it maps to; use {@link Boolean#TRUE}.
     */
    TryHarder(bool),

    /**
     * Specifies what character encoding to use when decoding, where applicable (type String)
     */
    CharacterSet(String),

    /**
     * Allowed lengths of encoded data -- reject anything else. Maps to an {@code int[]}.
     */
    AllowedLengths(Vec<u32>),

    /**
     * Assume Code 39 codes employ a check digit. Doesn't matter what it maps to;
     * use {@link Boolean#TRUE}.
     */
    AssumeCode39CheckDigit(bool),

    /**
     * Assume the barcode is being processed as a GS1 barcode, and modify behavior as needed.
     * For example this affects FNC1 handling for Code 128 (aka GS1-128). Doesn't matter what it maps to;
     * use {@link Boolean#TRUE}.
     */
    AssumeGs1(bool),

    /**
     * If true, return the start and end digits in a Codabar barcode instead of stripping them. They
     * are alpha, whereas the rest are numeric. By default, they are stripped, but this causes them
     * to not be. Doesn't matter what it maps to; use {@link Boolean#TRUE}.
     */
    ReturnCodabarStartEnd(bool),

    /**
     * The caller needs to be notified via callback when a possible {@link Point}
     * is found. Maps to a {@link PointCallback}.
     */
    NeedResultPointCallback(PointCallback),

    /**
     * Allowed extension lengths for EAN or UPC barcodes. Other formats will ignore this.
     * Maps to an {@code int[]} of the allowed extension lengths, for example [2], [5], or [2, 5].
     * If it is optional to have an extension, do not set this hint. If this is set,
     * and a UPC or EAN barcode is found but an extension is not, then no result will be returned
     * at all.
     */
    AllowedEanExtensions(Vec<u32>),

    /**
     * Translate the ASCII values parsed by the Telepen reader into the Telepen Numeric form; use {@link Boolean#TRUE}.
     */
    TelepenAsNumeric(bool),
}

#[derive(Default, Clone)]
pub struct DecodeHints {
    /**
     * Unspecified, application-specific hint. Maps to an unspecified {@link Object}.
     */
    pub other: Option<String>,

    /**
     * Image is known to be of one of a few possible formats.
     * Maps to a {@link List} of {@link BarcodeFormat}s.
     */
    pub possible_formats: Option<HashSet<BarcodeFormat>>,

    /**
     * Spend more time to try to find a barcode; optimize for accuracy, not speed.
     * Doesn't matter what it maps to; use {@link Boolean#TRUE}.
     */
    pub try_harder: Option<bool>,

    /**
     * Specifies what character encoding to use when decoding, where applicable (type String)
     */
    pub character_set: Option<String>,

    /**
     * Allowed lengths of encoded data -- reject anything else. Maps to an {@code int[]}.
     */
    pub allowed_lengths: Option<Vec<u32>>,

    /**
     * Assume Code 39 codes employ a check digit. Doesn't matter what it maps to;
     * use {@link Boolean#TRUE}.
     */
    pub assume_code_39_check_digit: Option<bool>,

    /**
     * Assume the barcode is being processed as a GS1 barcode, and modify behavior as needed.
     * For example this affects FNC1 handling for Code 128 (aka GS1-128). Doesn't matter what it maps to;
     * use {@link Boolean#TRUE}.
     */
    pub assume_gs1: Option<bool>,

    /**
     * If true, return the start and end digits in a Codabar barcode instead of stripping them. They
     * are alpha, whereas the rest are numeric. By default, they are stripped, but this causes them
     * to not be. Doesn't matter what it maps to; use {@link Boolean#TRUE}.
     */
    pub return_codabar_start_end: Option<bool>,

    /**
     * The caller needs to be notified via callback when a possible {@link Point}
     * is found. Maps to a {@link PointCallback}.
     */
    pub need_result_point_callback: Option<PointCallback>,

    /**
     * Allowed extension lengths for EAN or UPC barcodes. Other formats will ignore this.
     * Maps to an {@code int[]} of the allowed extension lengths, for example [2], [5], or [2, 5].
     * If it is optional to have an extension, do not set this hint. If this is set,
     * and a UPC or EAN barcode is found but an extension is not, then no result will be returned
     * at all.
     */
    pub allowed_ean_extensions: Option<Vec<u32>>,

    /**
     * Translate the ASCII values parsed by the Telepen reader into the Telepen Numeric form; use {@link Boolean#TRUE}.
     */
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
                DecodeHintValue::AllowedEanExtensions(v) => new_self.allowed_ean_extensions = Some(v),
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
            DecodeHintValue::NeedResultPointCallback(v) => self.need_result_point_callback = Some(v),
            DecodeHintValue::AllowedEanExtensions(v) => self.allowed_ean_extensions = Some(v),
            DecodeHintValue::TelepenAsNumeric(v) => self.telepen_as_numeric = Some(v),
        }
        self
    }
}
