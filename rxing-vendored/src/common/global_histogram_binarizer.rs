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
// import com.google.zxing.NotFoundException;

use std::borrow::Cow;

use once_cell::sync::OnceCell;

use crate::common::Result;
use crate::{Binarizer, Exceptions, LuminanceSource};

use super::{BitArray, BitMatrix, LineOrientation};

const LUMINANCE_BITS: usize = 5;
const LUMINANCE_SHIFT: usize = 8 - LUMINANCE_BITS;
const LUMINANCE_BUCKETS: usize = 1 << LUMINANCE_BITS;

/**
 * This Binarizer implementation uses the old ZXing global histogram approach. It is suitable
 * for low-end mobile devices which don't have enough CPU or memory to use a local thresholding
 * algorithm. However, because it picks a global black point, it cannot handle difficult shadows
 * and gradients.
 *
 * Faster mobile devices and all desktop applications should probably use HybridBinarizer instead.
 *
 * @author dswitkin@google.com (Daniel Switkin)
 * @author Sean Owen
 */
pub struct GlobalHistogramBinarizer<LS: LuminanceSource> {
    //_luminances: Vec<u8>,
    width: usize,
    height: usize,
    source: LS,
    black_matrix: OnceCell<BitMatrix>,
    black_row_cache: Vec<OnceCell<BitArray>>,
    black_column_cache: Vec<OnceCell<BitArray>>,
}

impl<LS: LuminanceSource> Binarizer for GlobalHistogramBinarizer<LS> {
    type Source = LS;

    fn get_luminance_source(&self) -> &Self::Source {
        &self.source
    }

    // Applies simple sharpening to the row data to improve performance of the 1D Readers.
    fn get_black_row(&'_ self, y: usize) -> Result<Cow<'_, BitArray>> {
        let row = self.black_row_cache[y].get_or_try_init(|| {
            let source = self.get_luminance_source();
            let width = source.get_width();
            let mut row = BitArray::with_size(width);

            let local_luminances = source
                .get_row(y)
                .ok_or(Exceptions::index_out_of_bounds_with("row out of bounds"))?;
            let mut local_buckets = [0; LUMINANCE_BUCKETS];
            for x in 0..width {
                local_buckets[((local_luminances[x]) >> LUMINANCE_SHIFT) as usize] += 1;
            }
            let black_point = Self::estimate_black_point(&local_buckets)?;

            if width < 3 {
                // Special case for very small images
                for (x, &lum) in local_luminances.iter().enumerate().take(width) {
                    if (lum as u32) < black_point {
                        row.set(x);
                    }
                }
            } else {
                let mut left = local_luminances[0];
                let mut center = local_luminances[1];
                for x in 1..width - 1 {
                    let right = local_luminances[x + 1];
                    // A simple -1 4 -1 box filter with a weight of 2.
                    if ((center as i64 * 4) - left as i64 - right as i64) / 2 < black_point as i64 {
                        row.set(x);
                    }
                    left = center;
                    center = right;
                }
            }

            Ok(row)
        })?;

        Ok(Cow::Borrowed(row))
    }

    fn get_black_line(&'_ self, l: usize, lt: LineOrientation) -> Result<Cow<'_, BitArray>> {
        if lt == LineOrientation::Row {
            self.get_black_row(l)
        } else {
            let col = self.black_column_cache[l].get_or_try_init(|| {
                let source = self.get_luminance_source();
                let height = source.get_height();
                let mut col = BitArray::with_size(height);

                let local_luminances = source.get_column(l);
                let mut local_buckets = [0; LUMINANCE_BUCKETS];
                for x in 0..height {
                    local_buckets[((local_luminances[x]) >> LUMINANCE_SHIFT) as usize] += 1;
                }
                let black_point = Self::estimate_black_point(&local_buckets)?;

                if height < 3 {
                    // Special case for very small images
                    for (x, lum) in local_luminances.iter().enumerate().take(height) {
                        if (*lum as u32) < black_point {
                            col.set(x);
                        }
                    }
                } else {
                    let mut left = local_luminances[0];
                    let mut center = local_luminances[1];
                    for x in 1..height - 1 {
                        let right = local_luminances[x + 1];
                        // A simple -1 4 -1 box filter with a weight of 2.
                        if ((center as i64 * 4) - left as i64 - right as i64) / 2
                            < black_point as i64
                        {
                            col.set(x);
                        }
                        left = center;
                        center = right;
                    }
                }

                Ok(col)
            })?;

            Ok(Cow::Borrowed(col))
        }
    }

    // Does not sharpen the data, as this call is intended to only be used by 2D Readers.
    fn get_black_matrix(&self) -> Result<&BitMatrix> {
        let matrix = self
            .black_matrix
            .get_or_try_init(|| Self::build_black_matrix(&self.source))?;
        Ok(matrix)
    }

    fn create_binarizer(&self, source: LS) -> Self {
        Self::new(source)
    }

    fn get_width(&self) -> usize {
        self.width
    }

    fn get_height(&self) -> usize {
        self.height
    }

    fn get_black_row_from_matrix(&'_ self, y: usize) -> Result<Cow<'_, BitArray>> {
        if let Some(matrix) = self.black_matrix.get() {
            Ok(Cow::Owned(matrix.get_row(y as u32)))
        } else {
            self.get_black_row(y)
        }
    }
}

impl<LS: LuminanceSource> GlobalHistogramBinarizer<LS> {
    pub fn new(source: LS) -> Self {
        Self {
            width: source.get_width(),
            height: source.get_height(),
            black_matrix: OnceCell::new(),
            black_row_cache: vec![OnceCell::default(); source.get_height()],
            black_column_cache: vec![OnceCell::default(); source.get_width()],
            source,
        }
    }

    fn build_black_matrix(source: &LS) -> Result<BitMatrix> {
        let width = source.get_width();
        let height = source.get_height();
        let mut matrix = BitMatrix::new(width as u32, height as u32)?;

        // Quickly calculates the histogram by sampling four rows from the image. This proved to be
        // more robust on the blackbox tests than sampling a diagonal as we used to do.
        let mut local_buckets = [0; LUMINANCE_BUCKETS];
        for y in 1..5 {
            let row = height * y / 5;
            let local_luminances = source
                .get_row(row)
                .ok_or(Exceptions::index_out_of_bounds_with("row out of bounds"))?;
            let right = (width * 4) / 5;
            for pixel in &local_luminances[(width / 5)..right] {
                local_buckets[(pixel >> LUMINANCE_SHIFT) as usize] += 1;
            }
        }
        let black_point = Self::estimate_black_point(&local_buckets)?;

        // We delay reading the entire image luminance until the black point estimation succeeds.
        // Although we end up reading four rows twice, it is consistent with our motto of
        // "fail quickly" which is necessary for continuous scanning.
        let local_luminances = source.get_matrix();
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

    fn estimate_black_point<const BUCKET_COUNT: usize>(buckets: &[u32; BUCKET_COUNT]) -> Result<u32> {
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
            std::mem::swap(&mut first_peak, &mut second_peak);
        }

        // If there is too little contrast in the image to pick a meaningful black point, throw rather
        // than waste time trying to decode the image, and risk false positives.
        if second_peak - first_peak <= BUCKET_COUNT / 16 {
            return Err(Exceptions::not_found_with(
                "second_peak - first_peak <= numBuckets / 16 ",
            ));
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
