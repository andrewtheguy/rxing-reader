/*
 * Copyright 2008 ZXing authors
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

/// Represents some type of metadata about the result of the decoding that the decoder
/// wishes to communicate back to the caller.
#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub enum RXingResultMetadataType {
    /// Unspecified, application-specific metadata. Maps to an unspecified value.
    OTHER,

    /// Denotes the likely approximate orientation of the barcode in the image. This value
    /// is given as degrees rotated clockwise from the normal, upright orientation.
    /// For example a 1D barcode which was found by reading top-to-bottom would be
    /// said to have orientation "90". This key maps to an integer whose
    /// value is in the range [0,360).
    ORIENTATION,

    /// 2D barcode formats typically encode text, but allow for a sort of 'byte mode'
    /// which is sometimes used to encode binary data. While [`crate::RXingResult`] makes available
    /// the complete raw bytes in the barcode for these formats, it does not offer the bytes
    /// from the byte segments alone.
    ///
    /// This maps to a [`Vec`] of byte arrays corresponding to the
    /// raw bytes in the byte segments in the barcode, in order.
    ByteSegments,

    /// Error correction level used, if applicable. The value type depends on the
    /// format, but is typically a String.
    ErrorCorrectionLevel,

    /// For some periodicals, indicates the issue number as an integer.
    IssueNumber,

    /// For some products, indicates the suggested retail price in the barcode as a
    /// formatted [`String`].
    SuggestedPrice,

    /// For some products, the possible country of manufacture as a [`String`] denoting the
    /// ISO country code. Some map to multiple possible countries, like "US/CA".
    PossibleCountry,

    /// For some products, the extension text
    UpcEanExtension,

    /// If the code format supports structured append and the current scanned code is part of one then the
    /// sequence number is given with it.
    StructuredAppendSequence,

    /// If the code format supports structured append and the current scanned code is part of one then the
    /// parity is given with it.
    StructuredAppendParity,

    /// Barcode Symbology Identifier.
    /// Note: According to the GS1 specification the identifier may have to replace a leading FNC1/GS character
    /// when prepending to the barcode content.
    SymbologyIdentifier,

    IsMirrored,

    ContentType,

    IsInverted,

    // In a filtered context, was the image "closed"
    FilteredClosed,

    // In a filtered context, what was the final read resolution
    FilteredResolution,
}

impl From<String> for RXingResultMetadataType {
    fn from(in_str: String) -> Self {
        match in_str.to_uppercase().as_str() {
            "OTHER" => RXingResultMetadataType::OTHER,
            "ORIENTATION" => RXingResultMetadataType::ORIENTATION,
            "BYTESEGMENTS" => RXingResultMetadataType::ByteSegments,
            "ERRORCORRECTIONLEVEL" | "ECLEVEL" => RXingResultMetadataType::ErrorCorrectionLevel,
            "ISSUENUMBER" => RXingResultMetadataType::IssueNumber,
            "SUGGESTEDPRICE" => RXingResultMetadataType::SuggestedPrice,
            "POSSIBLECOUNTRY" => RXingResultMetadataType::PossibleCountry,
            "UPCEANEXTENSION" => RXingResultMetadataType::UpcEanExtension,
            "STRUCTUREDAPPENDSEQUENCE" => RXingResultMetadataType::StructuredAppendSequence,
            "STRUCTUREDAPPENDPARITY" => RXingResultMetadataType::StructuredAppendParity,
            "SYMBOLOGYIDENTIFIER" => RXingResultMetadataType::SymbologyIdentifier,
            "ISMIRRORED" => RXingResultMetadataType::IsMirrored,
            "CONTENTTYPE" => RXingResultMetadataType::ContentType,
            "ISINVERTED" => RXingResultMetadataType::IsInverted,
            "FILTEREDCLOSED" => RXingResultMetadataType::FilteredClosed,
            "FILTEREDRESOLUTION" => RXingResultMetadataType::FilteredResolution,
            _ => RXingResultMetadataType::OTHER,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RXingResultMetadataValue {
    /// Unspecified, application-specific metadata. Maps to an unspecified value.
    OTHER(String),

    /// Denotes the likely approximate orientation of the barcode in the image. This value
    /// is given as degrees rotated clockwise from the normal, upright orientation.
    /// For example a 1D barcode which was found by reading top-to-bottom would be
    /// said to have orientation "90". This key maps to an integer whose
    /// value is in the range [0,360).
    Orientation(i32),

    /// 2D barcode formats typically encode text, but allow for a sort of 'byte mode'
    /// which is sometimes used to encode binary data. While [`crate::RXingResult`] makes available
    /// the complete raw bytes in the barcode for these formats, it does not offer the bytes
    /// from the byte segments alone.
    ///
    /// This maps to a [`Vec`] of byte arrays corresponding to the
    /// raw bytes in the byte segments in the barcode, in order.
    ByteSegments(Vec<Vec<u8>>),

    /// Error correction level used, if applicable. The value type depends on the
    /// format, but is typically a String.
    ErrorCorrectionLevel(String),

    /// For some periodicals, indicates the issue number as an integer.
    IssueNumber(i32),

    /// For some products, indicates the suggested retail price in the barcode as a
    /// formatted [`String`].
    SuggestedPrice(String),

    /// For some products, the possible country of manufacture as a [`String`] denoting the
    /// ISO country code. Some map to multiple possible countries, like "US/CA".
    PossibleCountry(String),

    /// For some products, the extension text
    UpcEanExtension(String),

    /// If the code format supports structured append and the current scanned code is part of one then the
    /// sequence number is given with it.
    StructuredAppendSequence(i32),

    /// If the code format supports structured append and the current scanned code is part of one then the
    /// parity is given with it.
    StructuredAppendParity(i32),

    /// Barcode Symbology Identifier.
    /// Note: According to the GS1 specification the identifier may have to replace a leading FNC1/GS character
    /// when prepending to the barcode content.
    SymbologyIdentifier(String),

    IsMirrored(bool),

    ContentType(String),

    IsInverted(bool),

    FilteredClosed(bool),

    FilteredResolution((usize, usize)),
}
