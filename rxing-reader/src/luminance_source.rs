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

/// Provides greyscale luminance values from an image.
///
/// The trait gives readers a common way to request rows, columns, and complete
/// matrices without depending on a particular image buffer representation.
pub trait LuminanceSource {
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

}
