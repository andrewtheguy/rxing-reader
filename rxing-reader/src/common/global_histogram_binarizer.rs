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

// package com.google.zxing.common;

// import com.google.zxing.Binarizer;
// import com.google.zxing.LuminanceSource;

use once_cell::sync::OnceCell;

use crate::{Binarizer, Error, LuminanceSource};
use anyhow::Result;

use super::BitMatrix;

const LUMINANCE_BITS: usize = 5;
const LUMINANCE_SHIFT: usize = 8 - LUMINANCE_BITS;
const LUMINANCE_BUCKETS: usize = 1 << LUMINANCE_BITS;

/// This Binarizer implementation uses the old ZXing global histogram approach. It is suitable
/// for low-end mobile devices which don't have enough CPU or memory to use a local thresholding
/// algorithm. However, because it picks a global black point, it cannot handle difficult shadows
/// and gradients.
///
/// Faster mobile devices and all desktop applications should probably use HybridBinarizer instead.
pub struct GlobalHistogramBinarizer<LS: LuminanceSource> {
    //_luminances: Vec<u8>,
    source: LS,
    black_matrix: OnceCell<BitMatrix>,
}

impl<LS: LuminanceSource> Binarizer for GlobalHistogramBinarizer<LS> {
    type Source = LS;

    fn luminance_source(&self) -> &Self::Source {
        &self.source
    }

    // Does not sharpen the data, as this call is intended to only be used by 2D Readers.
    fn black_matrix(&self) -> Result<&BitMatrix> {
        let matrix = self
            .black_matrix
            .get_or_try_init(|| Self::build_black_matrix(&self.source))?;
        Ok(matrix)
    }

    fn black_matrix_mut(&mut self) -> Result<&mut BitMatrix> {
        self.black_matrix
            .get_or_try_init(|| Self::build_black_matrix(&self.source))?;
        self.black_matrix.get_mut().ok_or_else(|| {
            Error::InvalidState {
                message: "black matrix cache was not initialized".into(),
            }
            .into()
        })
    }
}

impl<LS: LuminanceSource> GlobalHistogramBinarizer<LS> {
    pub fn new(source: LS) -> Self {
        Self {
            black_matrix: OnceCell::new(),
            source,
        }
    }

    pub(super) fn build_black_matrix(source: &LS) -> Result<BitMatrix> {
        let width = source.width();
        let height = source.height();
        let mut matrix = BitMatrix::new(width as u32, height as u32)?;

        // Quickly calculates the histogram by sampling four rows from the image. This proved to be
        // more robust on the blackbox tests than sampling a diagonal as we used to do.
        let mut local_buckets = [0; LUMINANCE_BUCKETS];
        for y in 1..5 {
            let row = height * y / 5;
            let local_luminances = source.row(row).ok_or_else(|| Error::InvalidState {
                message: format!("luminance source returned no data for sampled row {row}").into(),
            })?;
            let right = (width * 4) / 5;
            for pixel in &local_luminances[(width / 5)..right] {
                local_buckets[(pixel >> LUMINANCE_SHIFT) as usize] += 1;
            }
        }
        let black_point = Self::estimate_black_point(&local_buckets)?;

        // We delay reading the entire image luminance until the black point estimation succeeds.
        // Although we end up reading four rows twice, it is consistent with our motto of
        // "fail quickly" which is necessary for continuous scanning.
        let local_luminances = source.matrix();
        for y in 0..height {
            let offset = y * width;
            for x in 0..width {
                let pixel = local_luminances[offset + x];
                if (pixel as u32) < black_point {
                    matrix.set(x as u32, y as u32);
                }
            }
        }

        Ok(matrix)
    }

    fn estimate_black_point<const BUCKET_COUNT: usize>(
        buckets: &[u32; BUCKET_COUNT],
    ) -> Result<u32> {
        // Find the tallest peak in the histogram.
        let mut max_bucket_count = 0;
        let mut first_peak = 0;
        let mut first_peak_size = 0;
        for (x, &bucket) in buckets.iter().enumerate() {
            if bucket > first_peak_size {
                first_peak = x;
                first_peak_size = bucket;
            }
            if bucket > max_bucket_count {
                max_bucket_count = bucket;
            }
        }

        // Find the second-tallest peak which is somewhat far from the tallest peak.
        let mut second_peak = 0;
        let mut second_peak_score = 0;
        for (x, bucket) in buckets.iter().enumerate() {
            let distance_to_biggest = (x as i32 - first_peak as i32).unsigned_abs();
            // Encourage more distant second peaks by multiplying by square of distance.
            let score = *bucket * distance_to_biggest * distance_to_biggest;
            if score > second_peak_score {
                second_peak = x;
                second_peak_score = score;
            }
        }

        // Make sure first_peak corresponds to the black peak.
        if first_peak > second_peak {
            (first_peak, second_peak) = (second_peak, first_peak);
        }

        // If there is too little contrast in the image to pick a meaningful black point, throw rather
        // than waste time trying to decode the image, and risk false positives.
        if second_peak - first_peak <= BUCKET_COUNT / 16 {
            return Err(Error::NotFound {
                message: "second_peak - first_peak <= numBuckets / 16 ".into(),
            }
            .into());
        }

        // Find a valley between them that is low and closer to the white peak.
        let mut best_valley = second_peak as isize - 1;
        let mut best_valley_score = -1;

        let mut x = second_peak as isize;
        while x > first_peak as isize {
            let from_first = x - first_peak as isize;
            let score = from_first
                * from_first
                * (second_peak as isize - x)
                * (max_bucket_count - buckets[x as usize]) as isize;
            if score as i32 > best_valley_score {
                best_valley = x;
                best_valley_score = score as i32;
            }
            x -= 1;
        }

        Ok((best_valley as u32) << LUMINANCE_SHIFT)
    }
}
