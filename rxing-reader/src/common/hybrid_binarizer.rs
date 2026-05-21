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

use once_cell::sync::OnceCell;

use anyhow::{Context, Result};

use crate::{Binarizer, Error, LuminanceSource};

use super::{BitMatrix, GlobalHistogramBinarizer};

/// Local-thresholding binarizer.
///
/// This is slower than [`GlobalHistogramBinarizer`] but handles high-frequency
/// QR images with black data on white backgrounds more robustly. It is designed for
/// high-frequency QR images with black data on white backgrounds. For this application,
/// it does a much better job than a global blackpoint with severe shadows and gradients.
/// However it tends to produce artifacts on lower frequency images and is therefore not
/// a good general purpose binarizer for uses outside ZXing.
///
/// It wraps [`GlobalHistogramBinarizer`], using the older histogram approach for 1D readers
/// and the newer local approach for 2D readers. 1D decoding using a per-row histogram is already
/// inherently local, and only fails for horizontal gradients. We can revisit that problem later,
/// but for now it was not a win to use local blocks for 1D.
///
/// This binarizer is the default for the unit tests and the recommended implementation for library users.
pub struct HybridBinarizer<LS: LuminanceSource> {
    ghb: GlobalHistogramBinarizer<LS>,
    black_matrix: OnceCell<BitMatrix>,
}
impl<LS: LuminanceSource> Binarizer for HybridBinarizer<LS> {
    type Source = LS;

    fn luminance_source(&self) -> &LS {
        self.ghb.luminance_source()
    }

    /// Calculates the final BitMatrix once for all requests. This could be called once from the
    /// constructor instead, but there are some advantages to doing it lazily, such as making
    /// profiling easier, and not doing heavy lifting when callers don't expect it.
    fn black_matrix(&self) -> Result<&BitMatrix> {
        let matrix = self
            .black_matrix
            .get_or_try_init(|| Self::calculate_black_matrix(&self.ghb))?;
        Ok(matrix)
    }

    fn black_matrix_mut(&mut self) -> Result<&mut BitMatrix> {
        self.black_matrix
            .get_or_try_init(|| Self::calculate_black_matrix(&self.ghb))?;
        self.black_matrix
            .get_mut()
            .with_context(|| Error::invalid_state("black matrix cache was not initialized"))
    }
}

// This class uses 5x5 blocks to compute local luminance, where each block is 8x8 pixels.
// So this is the smallest dimension in each axis we can accept.
const BLOCK_SIZE_POWER: usize = 3;
const BLOCK_SIZE: usize = 1 << BLOCK_SIZE_POWER; // ...0100...00
const MINIMUM_DIMENSION: usize = BLOCK_SIZE * 5;
const MIN_DYNAMIC_RANGE: usize = 24;

impl<LS: LuminanceSource> HybridBinarizer<LS> {
    pub fn new(source: LS) -> Self {
        let ghb = GlobalHistogramBinarizer::new(source);
        Self {
            black_matrix: OnceCell::new(),
            ghb,
        }
    }

    fn calculate_black_matrix<LS2: LuminanceSource>(
        ghb: &GlobalHistogramBinarizer<LS2>,
    ) -> Result<BitMatrix> {
        let source = ghb.luminance_source();
        let width = source.width();
        let height = source.height();

        if width >= MINIMUM_DIMENSION && height >= MINIMUM_DIMENSION {
            let luminances = source.matrix();
            let sub_width = width.div_ceil(BLOCK_SIZE);
            let sub_height = height.div_ceil(BLOCK_SIZE);
            let black_points =
                Self::calculate_black_points(&luminances, sub_width, sub_height, width, height);

            let mut new_matrix = BitMatrix::new(width, height)
                .context("building hybrid binarizer bit matrix")?;
            Self::calculate_threshold_for_block(
                &luminances,
                sub_width,
                sub_height,
                width,
                height,
                &black_points,
                &mut new_matrix,
            );
            Ok(new_matrix)
        } else {
            // If the image is too small, fall back to the global histogram approach.
            GlobalHistogramBinarizer::build_black_matrix(source)
        }
    }

    /// For each block in the image, calculate the average black point using a 5x5 grid
    /// of the blocks around it. Also handles the corner cases (fractional blocks are computed based
    /// on the last pixels in the row/column which are also used in the previous block).
    fn calculate_threshold_for_block(
        luminances: &[u8],
        sub_width: usize,
        sub_height: usize,
        width: usize,
        height: usize,
        black_points: &[usize],
        matrix: &mut BitMatrix,
    ) {
        let max_yoffset = height - BLOCK_SIZE;
        let max_xoffset = width - BLOCK_SIZE;
        for y in 0..sub_height {
            let yoffset = (y << BLOCK_SIZE_POWER).min(max_yoffset);

            let top = y.clamp(2, sub_height - 3);
            for x in 0..sub_width {
                let xoffset = (x << BLOCK_SIZE_POWER).min(max_xoffset);

                let left = x.clamp(2, sub_width - 3);
                let mut sum = 0;
                for z in 0..5 {
                    let black_row = &black_points[(top + z - 2) * sub_width..];
                    sum += black_row[left - 2]
                        + black_row[left - 1]
                        + black_row[left]
                        + black_row[left + 1]
                        + black_row[left + 2];
                }
                let average = sum / 25;
                Self::threshold_block(luminances, xoffset, yoffset, average, width, matrix);
            }
        }
    }

    /// Applies a single threshold to a block of pixels.
    fn threshold_block(
        luminances: &[u8],
        xoffset: usize,
        yoffset: usize,
        threshold: usize,
        stride: usize,
        matrix: &mut BitMatrix,
    ) {
        let mut offset = yoffset * stride + xoffset;
        for y in 0..BLOCK_SIZE {
            for x in 0..BLOCK_SIZE {
                // Comparison needs to be <= so that black == 0 pixels are black even if the threshold is 0.
                if usize::from(luminances[offset + x]) <= threshold {
                    matrix.set(xoffset + x, yoffset + y);
                }
            }
            offset += stride;
        }
    }

    /// Calculates a single black point for each block of pixels and saves it away.
    /// See the following thread for a discussion of this algorithm:
    /// http://groups.google.com/group/zxing/browse_thread/thread/d06efa2c35a7ddc0
    fn calculate_black_points(
        luminances: &[u8],
        sub_width: usize,
        sub_height: usize,
        width: usize,
        height: usize,
    ) -> Vec<usize> {
        let max_yoffset = height - BLOCK_SIZE;
        let max_xoffset = width - BLOCK_SIZE;
        let mut black_points = vec![0; sub_height * sub_width];
        for y in 0..sub_height {
            let yoffset = (y << BLOCK_SIZE_POWER).min(max_yoffset);

            for x in 0..sub_width {
                let xoffset = (x << BLOCK_SIZE_POWER).min(max_xoffset);

                let mut sum = 0usize;
                let mut min = u8::MAX;
                let mut max = u8::MIN;

                let mut offset = yoffset * width + xoffset;
                let mut yy = 0;
                while yy < BLOCK_SIZE {
                    for &pixel in &luminances[offset..offset + BLOCK_SIZE] {
                        sum += usize::from(pixel);
                        // still looking for good contrast
                        min = min.min(pixel);
                        max = max.max(pixel);
                    }
                    // short-circuit min/max tests once dynamic range is met
                    if usize::from(max - min) > MIN_DYNAMIC_RANGE {
                        // finish the rest of the rows quickly
                        offset += width;
                        yy += 1;
                        while yy < BLOCK_SIZE {
                            sum += luminances[offset..offset + BLOCK_SIZE]
                                .iter()
                                .map(|&b| usize::from(b))
                                .sum::<usize>();
                            yy += 1;
                            offset += width;
                        }
                        break;
                    }
                    yy += 1;
                    offset += width;
                }

                // The default estimate is the average of the values in the block.
                let mut average = sum >> (BLOCK_SIZE_POWER * 2);
                if usize::from(max - min) <= MIN_DYNAMIC_RANGE {
                    // If variation within the block is low, assume this is a block with only light or only
                    // dark pixels. In that case we do not want to use the average, as it would divide this
                    // low contrast area into black and white pixels, essentially creating data out of noise.
                    //
                    // The default assumption is that the block is light/background. Since no estimate for
                    // the level of dark pixels exists locally, use half the min for the block.
                    average = usize::from(min) / 2;

                    if y > 0 && x > 0 {
                        // Correct the "white background" assumption for blocks that have neighbors by comparing
                        // the pixels in this block to the previously calculated black points. This is based on
                        // the fact that dark QR symbols are always surrounded by some amount of light
                        // background for which reasonable black point estimates were made. The bp estimated at
                        // the boundaries is used for the interior.

                        // The (min < bp) is arbitrary but works better than other heuristics that were tried.
                        let average_neighbor_black_point = (black_points
                            [(y - 1) * sub_width + x]
                            + (2 * black_points[y * sub_width + x - 1])
                            + black_points[(y - 1) * sub_width + x - 1])
                            / 4;
                        if usize::from(min) < average_neighbor_black_point {
                            average = average_neighbor_black_point;
                        }
                    }
                }
                black_points[y * sub_width + x] = average;
            }
        }
        black_points
    }
}
