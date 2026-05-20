#![allow(deprecated)]
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

use crate::{Error, Point, common::BitMatrix, point};

/**
 * <p>A somewhat generic detector that looks for a barcode-like rectangular region within an image.
 * It looks within a mostly white region of an image for a region of black and white, but mostly
 * black. It returns the four corners of the region, as best it can determine.</p>
 *
 * @author Sean Owen
 * @deprecated without replacement since 3.3.0
 */
const MAX_MODULES: i32 = 32;
#[deprecated]
pub struct MonochromeRectangleDetector<'a> {
    image: &'a BitMatrix,
}

impl<'a> MonochromeRectangleDetector<'_> {
    pub const fn new(image: &'a BitMatrix) -> MonochromeRectangleDetector<'a> {
        MonochromeRectangleDetector { image }
    }

    /**
     * <p>Detects a rectangular region of black and white -- mostly black -- with a region of mostly
     * white, in an image.</p>
     *
     * @return {@link Point}[] describing the corners of the rectangular region. The first and
     *  last points are opposed on the diagonal, as are the second and third. The first point will be
     *  the topmost point and the last, the bottommost. The second point will be leftmost and the
     *  third, the rightmost
     * Returns a not-found error if no Data Matrix Code can be found
     */
    pub fn detect(&self) -> Result<[Point; 4]> {
        let height = self.image.get_height() as i32;
        let width = self.image.get_width() as i32;
        let half_height = height / 2;
        let half_width = width / 2;
        let delta_y = 1.max(height / (MAX_MODULES * 8));
        let delta_x = 1.max(width / (MAX_MODULES * 8));

        let mut top = 0;
        let mut bottom = height;
        let mut left = 0;
        let mut right = width;
        let mut point_a = self.find_corner_from_center(
            half_width,
            0,
            left,
            right,
            half_height,
            -delta_y,
            top,
            bottom,
            half_width / 2,
        )?;
        top = (point_a.y - 1f32) as i32;
        let point_b = self.find_corner_from_center(
            half_width,
            -delta_x,
            left,
            right,
            half_height,
            0,
            top,
            bottom,
            half_height / 2,
        )?;
        left = (point_b.x - 1f32) as i32;
        let point_c = self.find_corner_from_center(
            half_width,
            delta_x,
            left,
            right,
            half_height,
            0,
            top,
            bottom,
            half_height / 2,
        )?;
        right = (point_c.x + 1f32) as i32;
        let point_d = self.find_corner_from_center(
            half_width,
            0,
            left,
            right,
            half_height,
            delta_y,
            top,
            bottom,
            half_width / 2,
        )?;
        bottom = (point_d.y + 1f32) as i32;

        // Go try to find point A again with better information -- might have been off at first.
        point_a = self.find_corner_from_center(
            half_width,
            0,
            left,
            right,
            half_height,
            -delta_y,
            top,
            bottom,
            half_width / 4,
        )?;

        Ok([point_a, point_b, point_c, point_d])
    }

    /**
     * Attempts to locate a corner of the barcode by scanning up, down, left or right from a center
     * point which should be within the barcode.
     *
     * @param center_x center's x component (horizontal)
     * @param delta_x same as delta_y but change in x per step instead
     * @param left minimum value of x
     * @param right maximum value of x
     * @param center_y center's y component (vertical)
     * @param delta_y change in y per step. If scanning up this is negative; down, positive;
     *  left or right, 0
     * @param top minimum value of y to search through (meaningless when di == 0)
     * @param bottom maximum value of y
     * @param max_white_run maximum run of white pixels that can still be considered to be within
     *  the barcode
     * @return a {@link Point} encapsulating the corner that was found
     * Returns a not-found error if such a point cannot be found
     */
    #[allow(clippy::too_many_arguments)]
    fn find_corner_from_center(
        &self,
        center_x: i32,
        delta_x: i32,
        left: i32,
        right: i32,
        center_y: i32,
        delta_y: i32,
        top: i32,
        bottom: i32,
        max_white_run: i32,
    ) -> Result<Point> {
        let mut last_range_z: Option<[i32; 2]> = None;
        let mut y: i32 = center_y;
        let mut x: i32 = center_x;
        while y < bottom && y >= top && x < right && x >= left {
            let range: Option<[i32; 2]> = if delta_x == 0 {
                // horizontal slices, up and down
                self.black_white_range(y, max_white_run, left, right, true)
            } else {
                // vertical slices, left and right
                self.black_white_range(x, max_white_run, top, bottom, false)
            };
            if let Some(r) = range {
                last_range_z = Some(r);
                y += delta_y;
                x += delta_x;
            } else if let Some(last_range) = last_range_z {
                // last_range was found
                if delta_x == 0 {
                    let last_y = y - delta_y;
                    if last_range[0] < center_x {
                        if last_range[1] > center_x {
                            // straddle, choose one or the other based on direction
                            return Ok(point(
                                last_range[usize::from(delta_y <= 0)] as f32,
                                last_y as f32,
                            ));
                        }
                        return Ok(point(last_range[0] as f32, last_y as f32));
                    } else {
                        return Ok(point(last_range[1] as f32, last_y as f32));
                    }
                } else {
                    let last_x = x - delta_x;
                    if last_range[0] < center_y {
                        if last_range[1] > center_y {
                            return Ok(point(
                                last_x as f32,
                                last_range[usize::from(delta_x >= 0)] as f32,
                            ));
                        }
                        return Ok(point(last_x as f32, last_range[0] as f32));
                    } else {
                        return Ok(point(last_x as f32, last_range[1] as f32));
                    }
                }
            } else {
                return Err(Error::NotFound {
                    message: "barcode pattern was not detected".to_owned(),
                }
                .into());
            }
        }
        Err(Error::NotFound {
            message: "barcode pattern was not detected".to_owned(),
        }
        .into())
    }

    /**
     * Computes the start and end of a region of pixels, either horizontally or vertically, that could
     * be part of a Data Matrix barcode.
     *
     * @param fixed_dimension if scanning horizontally, this is the row (the fixed vertical location)
     *  where we are scanning. If scanning vertically it's the column, the fixed horizontal location
     * @param max_white_run largest run of white pixels that can still be considered part of the
     *  barcode region
     * @param min_dim minimum pixel location, horizontally or vertically, to consider
     * @param max_dim maximum pixel location, horizontally or vertically, to consider
     * @param horizontal if true, we're scanning left-right, instead of up-down
     * @return int[] with start and end of found range, or null if no such range is found
     *  (e.g. only white was found)
     */
    fn black_white_range(
        &self,
        fixed_dimension: i32,
        max_white_run: i32,
        min_dim: i32,
        max_dim: i32,
        horizontal: bool,
    ) -> Option<[i32; 2]> {
        let center = (min_dim + max_dim) / 2;

        // Scan left/up first
        let mut start = center;
        while start >= min_dim {
            if if horizontal {
                self.image.get(start as u32, fixed_dimension as u32)
            } else {
                self.image.get(fixed_dimension as u32, start as u32)
            } {
                start -= 1;
            } else {
                let white_run_start = start;
                start -= 1;
                while start >= min_dim
                    && !(if horizontal {
                        self.image.get(start as u32, fixed_dimension as u32)
                    } else {
                        self.image.get(fixed_dimension as u32, start as u32)
                    })
                {
                    start -= 1;
                }
                let white_run_size = white_run_start - start;
                if start < min_dim || white_run_size > max_white_run {
                    start = white_run_start;
                    break;
                }
            }
        }
        start += 1;

        // Then try right/down
        let mut end = center;
        while end < max_dim {
            if if horizontal {
                self.image.get(end as u32, fixed_dimension as u32)
            } else {
                self.image.get(fixed_dimension as u32, end as u32)
            } {
                end += 1;
            } else {
                let white_run_start = end;
                end += 1;
                while end < max_dim
                    && !(if horizontal {
                        self.image.get(end as u32, fixed_dimension as u32)
                    } else {
                        self.image.get(fixed_dimension as u32, end as u32)
                    })
                {
                    end += 1;
                }
                let white_run_size = end - white_run_start;
                if end >= max_dim || white_run_size > max_white_run {
                    end = white_run_start;
                    break;
                }
            }
        }
        end -= 1;

        if end > start {
            Some([start, end])
        } else {
            None
        }
    }
}
