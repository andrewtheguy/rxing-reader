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

use crate::{
    DecodeHints, Error, Point, PointCallback,
    common::{
        BitMatrix, DefaultGridSampler, GridSampler, PerspectiveTransform, Quadrilateral,
        SamplerControl,
    },
    point,
    qrcode::common::Version,
};

use super::{
    AlignmentPattern, AlignmentPatternFinder, FinderPatternFinder, FinderPatternInfo,
    QRCodeDetectorResult,
};

/// Encapsulates logic that can detect a QR Code in an image, even if the QR Code
/// is rotated or skewed, or partially obscured.
pub struct Detector<'a> {
    image: &'a BitMatrix,
}

impl<'a> Detector<'a> {
    pub fn new(image: &'a BitMatrix) -> Detector<'a> {
        Detector { image }
    }

    pub fn get_image(&self) -> &BitMatrix {
        self.image
    }

    /// Detects a QR Code in an image.
    ///
    /// Returns [`QRCodeDetectorResult`] encapsulating results of detecting a QR Code.
    /// Returns a not-found error if QR Code cannot be found
    /// Returns an invalid-format error if a QR Code cannot be decoded
    pub fn detect(&mut self) -> Result<QRCodeDetectorResult> {
        self.detect_with_hints(&DecodeHints::default())
    }

    /// Detects a QR Code in an image.
    ///
    /// - `hints`: optional hints to detector
    ///
    /// Returns [`QRCodeDetectorResult`] encapsulating results of detecting a QR Code.
    /// Returns a not-found error if QR Code cannot be found
    /// Returns an invalid-format error if a QR Code cannot be decoded
    pub fn detect_with_hints(&mut self, hints: &DecodeHints) -> Result<QRCodeDetectorResult> {
        let result_point_callback = hints.need_result_point_callback.as_ref();

        let mut finder = FinderPatternFinder::with_callback(self.image, result_point_callback);
        let info = finder.find(hints)?;

        self.process_finder_pattern_info(info, result_point_callback)
    }

    pub fn process_finder_pattern_info(
        &self,
        info: FinderPatternInfo,
        result_point_callback: Option<&PointCallback>,
    ) -> Result<QRCodeDetectorResult> {
        let top_left = info.get_top_left();
        let top_right = info.get_top_right();
        let bottom_left = info.get_bottom_left();

        let module_size = self.calculate_module_size(top_left, top_right, bottom_left);
        if module_size < 1.0 {
            return Err(Error::NotFound {
                message: "barcode pattern was not detected".to_owned(),
            }
            .into());
        }
        let dimension = Self::compute_dimension(top_left, top_right, bottom_left, module_size)?;
        let provisional_version = Version::get_provisional_version_for_dimension(dimension)?;
        let modules_between_fpcenters = provisional_version.get_dimension_for_version() - 7;

        let mut alignment_pattern = None;
        // Anything above version 1 has an alignment pattern
        if !provisional_version
            .get_alignment_pattern_centers()
            .is_empty()
        {
            // Guess where a "bottom right" finder pattern would have been
            let bottom_right_x = top_right.point.x - top_left.point.x + bottom_left.point.x;
            let bottom_right_y = top_right.point.y - top_left.point.y + bottom_left.point.y;

            // Estimate that alignment pattern is closer by 3 modules
            // from "bottom right" to known top left location
            let correction_to_top_left = 1.0 - (3.0 / modules_between_fpcenters as f32);
            let est_alignment_x = (top_left.point.x
                + correction_to_top_left * (bottom_right_x - top_left.point.x))
                as u32;
            let est_alignment_y = (top_left.point.y
                + correction_to_top_left * (bottom_right_y - top_left.point.y))
                as u32;

            // Kind of arbitrary -- expand search radius before giving up
            let mut i = 4;
            while i <= 16 {
                if let Ok(ap) = self.find_alignment_in_region(
                    module_size,
                    est_alignment_x,
                    est_alignment_y,
                    i as f32,
                    result_point_callback,
                ) {
                    alignment_pattern = Some(ap);
                    break;
                }
                i <<= 1;
            }
            // If we didn't find alignment pattern... well try anyway without it
        }

        let transform = Self::create_transform(
            top_left,
            top_right,
            bottom_left,
            alignment_pattern.as_ref(),
            dimension,
        )
        .ok_or(Error::NotFound {
            message: "barcode pattern was not detected".to_owned(),
        })?;

        let bits = Detector::sample_grid(self.image, transform, dimension)?;

        let mut points = vec![
            Point::from(bottom_left),
            Point::from(top_left),
            Point::from(top_right),
        ];

        if alignment_pattern.is_some() {
            points.push(
                alignment_pattern
                    .ok_or(Error::NotFound {
                        message: "barcode pattern was not detected".to_owned(),
                    })?
                    .into(),
            )
        }

        Ok(QRCodeDetectorResult::new(bits, points))
    }

    fn create_transform<T: Into<Point>, X: Into<Point>>(
        top_left: T,
        top_right: T,
        bottom_left: T,
        alignment_pattern: Option<X>,
        dimension: u32,
    ) -> Option<PerspectiveTransform> {
        let top_left: Point = top_left.into();
        let top_right: Point = top_right.into();
        let bottom_left: Point = bottom_left.into();
        let alignment_pattern: Option<Point> = alignment_pattern.map(Into::into);

        let dim_minus_three = dimension as f32 - 3.5;
        let bottom_right_x: f32;
        let bottom_right_y: f32;
        let source_bottom_right_x: f32;
        let source_bottom_right_y: f32;
        if alignment_pattern.is_some() {
            let alignment_pattern = alignment_pattern?;
            bottom_right_x = alignment_pattern.x;
            bottom_right_y = alignment_pattern.y;
            source_bottom_right_x = dim_minus_three - 3.0;
            source_bottom_right_y = source_bottom_right_x;
        } else {
            // Don't have an alignment pattern, just make up the bottom-right point
            bottom_right_x = (top_right.x - top_left.x) + bottom_left.x;
            bottom_right_y = (top_right.y - top_left.y) + bottom_left.y;
            source_bottom_right_x = dim_minus_three;
            source_bottom_right_y = dim_minus_three;
        }

        let dst = Quadrilateral::new(
            point(3.5, 3.5),
            point(dim_minus_three, 3.5),
            point(source_bottom_right_x, source_bottom_right_y),
            point(3.5, dim_minus_three),
        );
        let src = Quadrilateral::new(
            top_left,
            top_right,
            point(bottom_right_x, bottom_right_y),
            bottom_left,
        );

        PerspectiveTransform::quadrilateral_to_quadrilateral(dst, src).ok()
    }

    fn sample_grid(
        image: &BitMatrix,
        transform: PerspectiveTransform,
        dimension: u32,
    ) -> Result<BitMatrix> {
        let sampler = DefaultGridSampler;
        let (res, _) = sampler.sample_grid(
            image,
            dimension,
            dimension,
            &[SamplerControl::new(dimension, dimension, transform)],
        )?;
        Ok(res)
    }

    /// Computes the dimension (number of modules on a size) of the QR Code based on the position
    /// of the finder patterns and estimated module size.
    fn compute_dimension<T: Into<Point> + Copy>(
        top_left: T,
        top_right: T,
        bottom_left: T,
        module_size: f32,
    ) -> Result<u32> {
        let tltr_centers_dimension =
            (Point::distance(top_left.into(), top_right.into()) / module_size).round() as i32;
        let tlbl_centers_dimension =
            (Point::distance(top_left.into(), bottom_left.into()) / module_size).round() as i32;
        let mut dimension = ((tltr_centers_dimension + tlbl_centers_dimension) / 2) + 7;
        match dimension & 0x03 {
            0 => dimension += 1,
            2 => dimension -= 1,
            3 => {
                return Err(Error::NotFound {
                    message: "barcode pattern was not detected".to_owned(),
                }
                .into());
            }
            _ => {}
        }
        Ok(dimension as u32)
    }

    /// Computes an average estimated module size based on estimated derived from the positions
    /// of the three finder patterns.
    ///
    /// - `top_left`: detected top-left finder pattern center
    /// - `top_right`: detected top-right finder pattern center
    /// - `bottom_left`: detected bottom-left finder pattern center
    ///
    /// Returns estimated module size.
    pub fn calculate_module_size<T: Into<Point> + Copy>(
        &self,
        top_left: T,
        top_right: T,
        bottom_left: T,
    ) -> f32 {
        // Take the average
        (self.calculate_module_size_one_way(top_left, top_right)
            + self.calculate_module_size_one_way(top_left, bottom_left))
            / 2.0
    }

    /// Estimates module size based on two finder patterns -- it uses
    /// [`int, int, int)`] to figure the
    /// width of each, measuring along the axis between their centers.
    fn calculate_module_size_one_way<T: Into<Point>>(&self, pattern: T, other_pattern: T) -> f32 {
        let pattern: Point = pattern.into();
        let other_pattern: Point = other_pattern.into();

        let module_size_est1 = self.size_of_black_white_black_run_both_ways(
            pattern.x.floor() as u32,
            pattern.y.floor() as u32,
            other_pattern.x.floor() as u32,
            other_pattern.y.floor() as u32,
        );
        let module_size_est2 = self.size_of_black_white_black_run_both_ways(
            other_pattern.x.floor() as u32,
            other_pattern.y.floor() as u32,
            pattern.x.floor() as u32,
            pattern.y.floor() as u32,
        );
        if module_size_est1.is_nan() {
            return module_size_est2 / 7.0;
        }
        if module_size_est2.is_nan() {
            return module_size_est1 / 7.0;
        }
        // Average them, and divide by 7 since we've counted the width of 3 black modules,
        // and 1 white and 1 black module on either side. Ergo, divide sum by 14.
        (module_size_est1 + module_size_est2) / 14.0
    }

    /// See [`int, int, int)`]; computes the total width of
    /// a finder pattern by looking for a black-white-black run from the center in the direction
    /// of another point (another finder pattern center), and in the opposite direction too.
    fn size_of_black_white_black_run_both_ways(
        &self,
        from_x: u32,
        from_y: u32,
        to_x: u32,
        to_y: u32,
    ) -> f32 {
        let mut result = self.size_of_black_white_black_run(from_x, from_y, to_x, to_y);

        // Now count other way -- don't run off image though of course
        let mut scale = 1.0;
        let mut other_to_x = from_x as i32 - (to_x as i32 - from_x as i32);
        if other_to_x < 0 {
            scale = from_x as f32 / (from_x as i32 - other_to_x) as f32;
            other_to_x = 0;
        } else if other_to_x as u32 >= self.image.get_width() {
            scale = (self.image.get_width() as i32 - 1 - from_x as i32) as f32
                / (other_to_x - from_x as i32) as f32;
            other_to_x = self.image.get_width() as i32 - 1;
        }
        let mut other_to_y = (from_y as f32 - (to_y as f32 - from_y as f32) * scale).floor() as i32;

        scale = 1.0;
        if other_to_y < 0 {
            scale = from_y as f32 / (from_y as i32 - other_to_y) as f32;
            other_to_y = 0;
        } else if other_to_y as u32 >= self.image.get_height() {
            scale = (self.image.get_height() as i32 - 1 - from_y as i32) as f32
                / (other_to_y - from_y as i32) as f32;
            other_to_y = self.image.get_height() as i32 - 1;
        }
        other_to_x = (from_x as f32 + (other_to_x as f32 - from_x as f32) * scale).floor() as i32;

        result += self.size_of_black_white_black_run(
            from_x,
            from_y,
            other_to_x as u32,
            other_to_y as u32,
        );

        // Middle pixel is double-counted this way; subtract 1
        result - 1.0
    }

    /// This method traces a line from a point in the image, in the direction towards another point.
    /// It begins in a black region, and keeps going until it finds white, then black, then white again.
    /// It reports the distance from the start to this point.
    ///
    /// This is used when figuring out how wide a finder pattern is, when the finder pattern
    /// may be skewed or rotated.
    fn size_of_black_white_black_run(&self, from_x: u32, from_y: u32, to_x: u32, to_y: u32) -> f32 {
        let mut from_x = from_x;
        let mut from_y = from_y;
        let mut to_x = to_x;
        let mut to_y = to_y;
        // Mild variant of Bresenham's algorithm;
        // see http://en.wikipedia.org/wiki/Bresenham's_line_algorithm
        let steep = (to_y as i64 - from_y as i64).abs() > (to_x as i64 - from_x as i64).abs();
        if steep {
            std::mem::swap(&mut from_x, &mut from_y);
            std::mem::swap(&mut to_x, &mut to_y);
        }

        let dx: i32 = (to_x as i64 - from_x as i64).abs() as i32;
        let dy: i32 = (to_y as i64 - from_y as i64).abs() as i32;
        let mut error = -dx / 2;
        let xstep: i32 = if from_x < to_x { 1 } else { -1 };
        let ystep: i32 = if from_y < to_y { 1 } else { -1 };

        // In black pixels, looking for white, first or second time.
        let mut state = 0;
        // Loop up until x == to_x, but not beyond
        let x_limit = to_x as i32 + xstep;

        let mut x: i32 = from_x as i32;
        let mut y: i32 = from_y as i32;
        while x != x_limit {
            let real_x = if steep { y } else { x };
            let real_y = if steep { x } else { y };

            // Does current pixel mean we have moved white to black or vice versa?
            // Scanning black in state 0,2 and white in state 1, so if we find the wrong
            // color, advance to next state or end if we are in state 2 already
            if (state == 1) == self.image.get(real_x as u32, real_y as u32) {
                if state == 2 {
                    return Point::distance(
                        point(x as f32, y as f32),
                        point(from_x as f32, from_y as f32),
                    );
                }
                state += 1;
            }

            error += dy;
            if error > 0 {
                if y == to_y as i32 {
                    break;
                }
                y += ystep;
                error -= dx;
            }

            x += xstep;
        }
        // Found black-white-black; give the benefit of the doubt that the next pixel outside the image
        // is "white" so this last point at (to_x+xStep,to_y) is the right ending. This is really a
        // small approximation; (to_x+xStep,to_y+yStep) might be really correct. Ignore this.
        if state == 2 {
            return Point::distance(
                point((to_x as i32 + xstep) as f32, to_y as f32),
                point(from_x as f32, from_y as f32),
            );
        }
        // else we didn't find even black-white-black; no estimate is really possible
        f32::NAN
    }

    /// Attempts to locate an alignment pattern in a limited region of the image, which is
    /// guessed to contain it. This method uses [`AlignmentPattern`].
    ///
    /// - `overall_est_module_size`: estimated module size so far
    /// - `est_alignment_x`: x coordinate of center of area probably containing alignment pattern
    /// - `est_alignment_y`: y coordinate of above
    /// - `allowance_factor`: number of pixels in all directions to search from the center
    ///
    /// Returns the alignment pattern if one is found.
    ///
    /// Returns a not-found error if an unexpected error occurs during detection.
    pub fn find_alignment_in_region(
        &self,
        overall_est_module_size: f32,
        est_alignment_x: u32,
        est_alignment_y: u32,
        allowance_factor: f32,
        result_point_callback: Option<&PointCallback>,
    ) -> Result<AlignmentPattern> {
        // Look for an alignment pattern (3 modules in size) around where it
        // should be
        let allowance = (allowance_factor * overall_est_module_size) as u32;
        let alignment_area_left_x = 0.max(est_alignment_x as i32 - allowance as i32) as u32;
        let alignment_area_right_x = (self.image.get_width() - 1).min(est_alignment_x + allowance);
        let alignment_area_width = alignment_area_right_x
            .checked_sub(alignment_area_left_x)
            .ok_or(Error::NotFound {
                message: "barcode pattern was not detected".to_owned(),
            })?;

        if (alignment_area_width as f32) < overall_est_module_size * 3.0 {
            return Err(Error::NotFound {
                message: "barcode pattern was not detected".to_owned(),
            }
            .into());
        }

        let alignment_area_top_y = 0.max(est_alignment_y as i32 - allowance as i32) as u32;
        let alignment_area_bottom_y =
            (self.image.get_height() - 1).min(est_alignment_y + allowance);
        let alignment_area_height = alignment_area_bottom_y
            .checked_sub(alignment_area_top_y)
            .ok_or(Error::NotFound {
                message: "barcode pattern was not detected".to_owned(),
            })?;

        if alignment_area_height < overall_est_module_size as u32 * 3 {
            return Err(Error::NotFound {
                message: "barcode pattern was not detected".to_owned(),
            }
            .into());
        }

        let mut alignment_finder = AlignmentPatternFinder::new(
            self.image,
            alignment_area_left_x,
            alignment_area_top_y,
            alignment_area_width,
            alignment_area_height,
            overall_est_module_size,
            result_point_callback,
        );
        alignment_finder.find()
    }
}
