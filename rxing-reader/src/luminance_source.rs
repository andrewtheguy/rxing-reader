/*
 * Copyright 2009 ZXing authors
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

use std::borrow::Cow;

use crate::Error;
use anyhow::Result;

/// Provides greyscale luminance values from an image.
///
/// The trait gives readers a common way to request rows, columns, and complete
/// matrices without depending on a particular image buffer representation.
pub trait LuminanceSource {
    const SUPPORTS_ROTATION: bool = false;
    const SUPPORTS_CROP: bool = false;

    /// Fetches one row of luminance data from the underlying image.
    ///
    /// Values range from `0` (black) to `255` (white). Implementations should
    /// prefer fetching only the requested row when that is cheaper than
    /// materializing the whole matrix.
    ///
    /// - `y`: The row to fetch, which must be in `[0, height)`.
    ///
    /// Returns the luminance data for the row.
    fn get_row(&self, y: usize) -> Option<Cow<'_, [u8]>>;

    /// Returns the luminance values for column `x`, from top to bottom.
    fn get_column(&self, x: usize) -> Vec<u8>;

    /// Fetches row-major luminance data for the underlying image.
    ///
    /// Returns row-major luminance values. Implementations may return a backing
    /// buffer that is larger than `width * height`; callers should only read the
    /// image region described by the dimensions.
    fn get_matrix(&self) -> Cow<'_, [u8]>;

    /// Returns the width of the source image.
    fn get_width(&self) -> usize;

    /// Returns the height of the source image.
    fn get_height(&self) -> usize;

    /// Returns whether this source supports cropping.
    fn is_crop_supported(&self) -> bool {
        Self::SUPPORTS_CROP
    }

    /// Returns whether this source supports counter-clockwise rotation.
    fn is_rotate_supported(&self) -> bool {
        Self::SUPPORTS_ROTATION
    }

    /// Returns a source over cropped image data.
    ///
    /// Implementations may keep a reference to the original data rather than
    /// copying it. Only call this when [`Self::is_crop_supported`] returns `true`.
    ///
    /// - `left`: The left coordinate, which must be in [0,get_width())
    /// - `top`: The top coordinate, which must be in [0,get_height())
    /// - `width`: The width of the rectangle to crop.
    /// - `height`: The height of the rectangle to crop.
    ///
    /// Returns the cropped source.
    fn crop(&self, _left: usize, _top: usize, _width: usize, _height: usize) -> Result<Self>
    where
        Self: Sized,
    {
        Err(Error::UnsupportedOperation {
            message: "This luminance source does not support cropping.".to_owned(),
        }
        .into())
    }

    /// Inverts the luminance values in place: black becomes white and each
    /// value becomes `255 - value`.
    fn invert(&mut self);

    /// Returns a source over image data rotated 90 degrees counter-clockwise.
    ///
    /// Only call this when [`Self::is_rotate_supported`] returns `true`.
    ///
    /// Returns the rotated source.
    fn rotate_counter_clockwise(&self) -> Result<Self>
    where
        Self: Sized,
    {
        Err(Error::UnsupportedOperation {
            message: "This luminance source does not support rotation by 90 degrees.".to_owned(),
        }
        .into())
    }

    /// Returns a source over image data rotated 45 degrees counter-clockwise.
    ///
    /// Only call this when [`Self::is_rotate_supported`] returns `true`.
    ///
    /// Returns the rotated source.
    fn rotate_counter_clockwise_45(&self) -> Result<Self>
    where
        Self: Sized,
    {
        Err(Error::UnsupportedOperation {
            message: "This luminance source does not support rotation by 45 degrees.".to_owned(),
        }
        .into())
    }

    #[inline(always)]
    fn invert_block_of_bytes(&self, vec_to_invert: Vec<u8>) -> Vec<u8> {
        let mut iv = vec_to_invert;
        for itm in iv.iter_mut() {
            *itm = 255 - *itm;
        }
        iv
    }

    fn get_luma8_point(&self, x: usize, y: usize) -> u8;
}
