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

use std::fmt;

use anyhow::Result;

use crate::{Binarizer, common::BitMatrix};

/// One-bit view of an image produced by a [`Binarizer`].
///
/// Readers consume `BinaryBitmap` values when searching for barcodes.
pub struct BinaryBitmap<B: Binarizer> {
    binarizer: B,
}

impl<B: Binarizer> BinaryBitmap<B> {
    pub fn new(binarizer: B) -> Self {
        Self { binarizer }
    }

    /// Returns the mutable image matrix, where `true` means black.
    /// Returns an error if the image cannot be binarized into a matrix.
    pub fn black_matrix_mut(&mut self) -> Result<&mut BitMatrix> {
        self.binarizer.black_matrix_mut()
    }

    /// Converts the image to a black/white matrix for QR detection.
    /// Returns the image matrix, where `true` means black.
    /// Returns an error if the image cannot be binarized into a matrix.
    pub fn black_matrix(&self) -> Result<&BitMatrix> {
        self.binarizer.black_matrix()
    }

    /// Apply a 3×3 morphological close to the cached BitMatrix: dilate (set
    /// any pixel whose 3×3 neighborhood contains at least one black pixel),
    /// then erode (keep only pixels whose 3×3 neighborhood is entirely
    /// black). Useful as a post-binarization denoise step for marginal
    /// photos (cf. zxing-cpp's `tryDenoise`). The 1-pixel border is left
    /// unchanged. No-op if the matrix is smaller than 3×3.
    pub fn close(&mut self) -> Result<()> {
        let matrix = self.black_matrix_mut()?;
        let (w, h) = (matrix.width(), matrix.height());
        if w < 3 || h < 3 {
            return Ok(());
        }
        let mut tmp = BitMatrix::new(w, h)?;
        sum_filter_3x3(matrix, &mut tmp, |sum| sum > 0); // dilate
        sum_filter_3x3(&tmp, matrix, |sum| sum == 9); // erode
        Ok(())
    }
}

/// 3×3 neighborhood scan: for each interior pixel of `output`, sum the 9
/// `input` pixels in its 3×3 neighborhood and run `predicate` on the count.
/// Out-of-bounds reads default to false (only touches the 1-pixel border).
fn sum_filter_3x3<F: Fn(u8) -> bool>(input: &BitMatrix, output: &mut BitMatrix, predicate: F) {
    let (w, h) = (output.width(), output.height());
    for row in 1..h - 1 {
        for col in 1..w - 1 {
            let mut sum: u8 = 0;
            for dx in 0..3u32 {
                sum += input.try_get(col + dx - 1, row - 1).unwrap_or(false) as u8
                    + input.try_get(col + dx - 1, row).unwrap_or(false) as u8
                    + input.try_get(col + dx - 1, row + 1).unwrap_or(false) as u8;
            }
            output.set_bool(col, row, predicate(sum));
        }
    }
}

impl<B: Binarizer> fmt::Display for BinaryBitmap<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.binarizer.black_matrix() {
            Ok(matrix) => write!(f, "{matrix:?}"),
            Err(_) => write!(f, "<unavailable>"),
        }
    }
}
