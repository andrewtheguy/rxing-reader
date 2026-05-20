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

use anyhow::Result;

use crate::{
    LuminanceSource,
    common::{BitArray, BitMatrix, LineOrientation},
};

/// Converts greyscale luminance data into one-bit black/white barcode data.
///
/// Implementations can choose different thresholding strategies while exposing
/// the same row, line, and matrix accessors to readers.
pub trait Binarizer {
    type Source: LuminanceSource;

    fn get_luminance_source(&self) -> &Self::Source;

    /// Converts one row of luminance data to 1 bit data. May actually do the conversion, or return
    /// cached data. Callers should assume this method is expensive and call it as seldom as possible.
    /// This method is intended for decoding 1D barcodes and may choose to apply sharpening.
    /// Callers should avoid repeated calls for the same row unless the
    /// implementation documents cheap caching.
    ///
    /// - `y`: The row to fetch, which must be in `[0, height)`.
    ///
    /// Returns the row bits, where `true` means black.
    /// Returns an error if the row cannot be binarized.
    fn get_black_row(&self, y: usize) -> Result<Cow<'_, BitArray>>;

    // An alternate version of get_black_row that fetches the line from the matrix if
    // it has already been generated, falling back to get_black_row if it hasn't.
    fn get_black_row_from_matrix(&self, y: usize) -> Result<Cow<'_, BitArray>>;

    /// Converts a 2D array of luminance data to 1 bit data. As above, assume this method is expensive
    /// and do not call it repeatedly. This method is intended for decoding 2D barcodes and may or
    /// may not apply sharpening. Therefore, a row from this matrix may not be identical to one
    /// fetched using `get_black_row()`, so don't mix and match between them.
    ///
    /// Returns the image bits, where `true` means black.
    /// Returns an error if the image cannot be binarized into a matrix.
    fn get_black_matrix(&self) -> Result<&BitMatrix>;

    fn get_black_matrix_mut(&mut self) -> Result<&mut BitMatrix>;

    /// Returns a black-pixel line in the requested orientation.
    ///
    /// `l` is interpreted as a row index for horizontal lines and a column
    /// index for vertical lines.
    fn get_black_line(&self, l: usize, lt: LineOrientation) -> Result<Cow<'_, BitArray>>;

    /// Creates a fresh binarizer of the same concrete type for `source`.
    ///
    /// This is used when a luminance source is cropped or rotated and any
    /// cached one-bit data from the previous source must be discarded.
    ///
    /// Returns a new binarizer implementation object.
    fn create_binarizer(&self, source: Self::Source) -> Self
    where
        Self: Sized;

    fn get_width(&self) -> usize;

    fn get_height(&self) -> usize;
}
