mod common;
mod decode;
mod error;

use binarizer::Binarizer;
use binary_bitmap::BinaryBitmap;
use decode_hints::DecodeHints;
use error::Error;
use luma_luma_source::{Luma8LuminanceSource, downscale_luma_buffer};
use luminance_source::LuminanceSource;
use rxing_result_point::{Point, PointI, point, point_i};

pub use decode::{decode_qr_codes_luma, rgba_to_luma};

mod qrcode;

// Reading
mod decode_hints;

mod rxing_result_point;

// Reading sources
mod binarizer;

mod binary_bitmap;

mod luminance_source;

mod luma_luma_source;
