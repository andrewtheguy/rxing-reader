pub mod common;
pub mod decode;
mod error;

use std::collections::HashMap;

pub use error::Error;

pub type MetadataDictionary = HashMap<RXingResultMetadataType, RXingResultMetadataValue>;

mod barcode_format;
pub use barcode_format::*;

/// Callback which is invoked when a possible result point (significant
/// point in the barcode image such as a corner) is found.
pub type PointCallback = Box<dyn Fn(Point) + Send + Sync>;

mod dimension;
pub use dimension::*;

pub mod qrcode;

// Reading
mod decode_hints;
pub use decode_hints::*;

mod rxing_result_metadata;
pub use rxing_result_metadata::*;

mod rxing_result;
pub use rxing_result::*;

mod result_point;
pub use result_point::*;

pub mod result_point_utils;

mod rxing_result_point;
pub use rxing_result_point::*;

pub type DecodingHintDictionary = HashMap<DecodeHintType, DecodeHintValue>;

// Reading sources
mod binarizer;
pub use binarizer::*;

mod binary_bitmap;
pub use binary_bitmap::*;

mod luminance_source;
pub use luminance_source::*;

mod luma_luma_source;
pub use luma_luma_source::*;
