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

use anyhow::Result;

use crate::{LuminanceSource, common::BitMatrix};

/// Converts greyscale luminance data into one-bit black/white barcode data.
///
/// Implementations can choose different thresholding strategies while exposing
/// the same row, line, and matrix accessors to readers.
pub trait Binarizer {
    type Source: LuminanceSource;

    fn get_luminance_source(&self) -> &Self::Source;

    /// Converts the image to a black/white matrix for QR detection.
    /// Returns the image bits, where `true` means black.
    /// Returns an error if the image cannot be binarized into a matrix.
    fn get_black_matrix(&self) -> Result<&BitMatrix>;

    fn get_black_matrix_mut(&mut self) -> Result<&mut BitMatrix>;
}
