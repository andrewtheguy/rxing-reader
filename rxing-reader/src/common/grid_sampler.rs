/*
 * Copyright 2007 ZXing authors
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

use crate::{Error, Point};

use super::{BitMatrix, PerspectiveTransform};

/// Samples a perspective-corrected grid from an image.
///
/// Implementations use finder-pattern locations to reconstruct a QR Code's
/// module grid while accounting for perspective distortion. The abstraction
/// allows callers to swap in optimized sampling implementations.
pub trait GridSampler {
    fn sample_grid(
        &self,
        image: &BitMatrix,
        dimension_x: u32,
        dimension_y: u32,
        controls: &[SamplerControl],
    ) -> Result<BitMatrix> {
        if dimension_x == 0 || dimension_y == 0 {
            return Err(Error::NotFound {
                message: "barcode pattern was not detected".to_owned(),
            }
            .into());
        }
        let mut bits = BitMatrix::new(dimension_x, dimension_y)?;
        let mut points = vec![Point::default(); dimension_x as usize];
        for y in 0..dimension_y {
            let i_value = y as f32 + 0.5;

            for (x, point) in points.iter_mut().enumerate() {
                point.x = (x as f32) + 0.5;
                point.y = i_value;
            }

            for control in controls {
                control.transform.transform_points_single(&mut points)?;
            }
            // Quick check to see if points transformed to something inside the image;
            // sufficient to check the endpoints
            self.check_and_nudge_points(image, &mut points)?;
            for (x, point) in points.iter().enumerate() {
                if image
                    .try_get(point.x as u32, point.y as u32)
                    .ok_or(Error::NotFound {
                        message: "grid point transformed outside image bounds".to_owned(),
                    })?
                {
                    // Black(-ish) pixel
                    bits.set(x as u32, y);
                }
            }
        }

        Ok(bits)
    }

    /// Checks a set of points that have been transformed to sample points on an image against
    /// the image's dimensions to see if the point are even within the image.
    ///
    /// This method will actually "nudge" the endpoints back onto the image if they are found to be
    /// barely (less than 1 pixel) off the image. This accounts for imperfect detection of finder
    /// patterns in an image where the QR Code runs all the way to the image border.
    ///
    /// For efficiency, the method will check points from either end of the line until one is found
    /// to be within the image. Because the set of points are assumed to be linear, this is valid.
    ///
    /// - `image`: image into which the points should map
    /// - `points`: actual points in x1,y1,...,xn,yn form
    ///
    /// Returns a not-found error if an endpoint is lies outside the image boundaries
    fn check_and_nudge_points(&self, image: &BitMatrix, points: &mut [Point]) -> Result<()> {
        let width = image.get_width();
        let height = image.get_height();
        // Check and nudge points from start until we see some that are OK:
        let mut nudged;
        if points.is_empty() {
            return Ok(());
        }
        let max_offset = points.len() - 1; // points.length must be even
        for point in points.iter_mut().take(max_offset) {
            let (x, y) = (point.x as i32, point.y as i32);
            if x < -1 || x > width as i32 || y < -1 || y > height as i32 {
                return Err(Error::NotFound {
                    message: "barcode pattern was not detected".to_owned(),
                }
                .into());
            }
            nudged = false;
            if x == -1 {
                point.x = 0.0;
                nudged = true;
            } else if x == width as i32 {
                point.x = width as f32 - 1.0;
                nudged = true;
            }
            if y == -1 {
                point.y = 0.0;
                nudged = true;
            } else if y == height as i32 {
                point.y = height as f32 - 1.0;
                nudged = true;
            }
            if !nudged {
                break;
            }
        }
        // Check and nudge points from end:
        for point in points.iter_mut().rev().take(max_offset).rev() {
            let (x, y) = (point.x as i32, point.y as i32);
            if x < -1 || x > width as i32 || y < -1 || y > height as i32 {
                return Err(Error::NotFound {
                    message: "barcode pattern was not detected".to_owned(),
                }
                .into());
            }
            nudged = false;
            if x == -1 {
                point.x = 0.0;
                nudged = true;
            } else if x == width as i32 {
                point.x = width as f32 - 1.0;
                nudged = true;
            }
            if y == -1 {
                point.y = 0.0;
                nudged = true;
            } else if y == height as i32 {
                point.y = height as f32 - 1.0;
                nudged = true;
            }
            if !nudged {
                break;
            }
        }
        Ok(())
    }
}

pub struct SamplerControl {
    pub p0: Point,
    pub p1: Point,
    pub transform: PerspectiveTransform,
}
