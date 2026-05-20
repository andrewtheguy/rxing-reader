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

use std::ops::Div;

use anyhow::Result;

use crate::{DecodeHints, Error, Point, PointCallback, common::BitMatrix, result_point_utils};

use super::{FinderPattern, FinderPatternInfo};

/// Finds the square finder patterns at three corners of a QR Code.
///
/// The finder borrows its input image and stores scan state, so use one finder
/// per detection attempt.
pub struct FinderPatternFinder<'a> {
    image: &'a BitMatrix,
    possible_centers: Vec<FinderPattern>,
    has_skipped: bool,
    // cross_check_state_count: [u32; 5],
    result_point_callback: Option<&'a PointCallback>,
}
impl<'a> FinderPatternFinder<'a> {
    pub const CENTER_QUORUM: usize = 2;
    pub const MIN_SKIP: u32 = 3; // 1 pixel/module times 3 modules/center
    pub const MAX_MODULES: u32 = 97; // support up to version 20 for mobile clients

    /// Creates a finder that will search the image for three finder patterns.
    ///
    /// - `image`: image to search
    pub fn new(image: &'a BitMatrix) -> FinderPatternFinder<'a> {
        Self::with_callback(image, None)
    }

    pub fn with_callback(
        image: &'a BitMatrix,
        result_point_callback: Option<&'a PointCallback>,
    ) -> FinderPatternFinder<'a> {
        FinderPatternFinder {
            image,
            possible_centers: Vec::new(),
            has_skipped: false,
            // cross_check_state_count: [0u32; 5],
            result_point_callback,
        }
    }

    pub fn get_image(&self) -> &BitMatrix {
        self.image
    }

    pub fn get_possible_centers(&self) -> &Vec<FinderPattern> {
        &self.possible_centers
    }

    pub fn find(&mut self, hints: &DecodeHints) -> Result<FinderPatternInfo> {
        let try_harder = matches!(hints.try_harder, Some(true));
        let max_i = self.image.get_height();
        let max_j = self.image.get_width();
        // We are looking for black/white/black/white/black modules in
        // 1:1:3:1:1 ratio; this tracks the number of such modules seen so far

        // Let's assume that the maximum version QR Code we support takes up 1/4 the height of the
        // image, and then account for the center being 3 modules in size. This gives the smallest
        // number of pixels the center could be, so skip this often. When trying harder, look for all
        // QR versions regardless of how dense they are.
        let mut i_skip = (3 * max_i) / (4 * Self::MAX_MODULES);
        if i_skip < Self::MIN_SKIP || try_harder {
            i_skip = Self::MIN_SKIP;
        }

        let mut done = false;
        let mut state_count = [0u32; 5];
        let mut i = i_skip as i32 - 1;
        while i < max_i as i32 && !done {
            // Get a row of black/white values
            FinderPatternFinder::do_clear_counts(&mut state_count);
            let mut current_state = 0;
            let mut j = 0;
            while j < max_j {
                if self.image.get(j, i as u32) {
                    // Black pixel
                    if (current_state & 1) == 1 {
                        // Counting white pixels
                        current_state += 1;
                    }
                    state_count[current_state] += 1;
                } else {
                    // White pixel
                    if (current_state & 1) == 0 {
                        // Counting black pixels
                        if current_state == 4 {
                            // A winner?
                            if FinderPatternFinder::found_pattern_cross(&state_count) {
                                // Yes
                                let confirmed =
                                    self.handle_possible_center(&state_count, i as u32, j);
                                if confirmed {
                                    // Start examining every other line. Checking each line turned out to be too
                                    // expensive and didn't improve performance.
                                    i_skip = 2;
                                    if self.has_skipped {
                                        done = self.have_multiply_confirmed_centers();
                                    } else {
                                        let row_skip = self.find_row_skip();
                                        if row_skip > state_count[2] {
                                            // Skip rows between row of lower confirmed center
                                            // and top of presumed third confirmed center
                                            // but back up a bit to get a full chance of detecting
                                            // it, entire width of center of finder pattern

                                            // Skip by row_skip, but back off by state_count[2] (size of last center
                                            // of pattern we saw) to be conservative, and also back off by i_skip which
                                            // is about to be re-added
                                            i += row_skip as i32
                                                - state_count[2] as i32
                                                - i_skip as i32;
                                            // i += row_skip  - state_count[2]  - i_skip ;
                                            j = max_j - 1;
                                        }
                                    }
                                } else {
                                    FinderPatternFinder::do_shift_counts2(&mut state_count);
                                    current_state = 3;
                                    j += 1;
                                    continue;
                                }
                                // Clear state to start looking again
                                current_state = 0;
                                FinderPatternFinder::do_clear_counts(&mut state_count);
                            } else {
                                // No, shift counts back by two
                                FinderPatternFinder::do_shift_counts2(&mut state_count);
                                current_state = 3;
                            }
                        } else {
                            current_state += 1;
                            state_count[current_state] += 1;
                        }
                    } else {
                        // Counting white pixels
                        state_count[current_state] += 1;
                    }
                }
                j += 1;
            }
            if FinderPatternFinder::found_pattern_cross(&state_count) {
                let confirmed = self.handle_possible_center(&state_count, i as u32, max_j);
                if confirmed {
                    i_skip = state_count[0];
                    if self.has_skipped {
                        // Found a third one
                        done = self.have_multiply_confirmed_centers();
                    }
                }
            }

            i += i_skip as i32;
        }

        let mut pattern_info = self.select_best_patterns()?;
        result_point_utils::order_best_patterns(&mut pattern_info);

        Ok(FinderPatternInfo::new(pattern_info))
    }

    /// Given a count of black/white/black/white/black pixels just seen and an end position,
    /// figures the location of the center of this run.
    fn center_from_end(state_count: &[u32], end: u32) -> f32 {
        (end - state_count[4] - state_count[3]) as f32 - ((state_count[2] as f32) / 2.0)
    }

    /// - `state_count`: count of black/white/black/white/black pixels just read
    ///
    /// Returns `true` when the counts are close enough to the 1/1/3/1/1 finder-pattern ratio.
    pub fn found_pattern_cross(state_count: &[u32]) -> bool {
        let mut total_module_size = 0;
        for count in state_count.iter().take(5) {
            if *count == 0 {
                return false;
            }
            total_module_size += *count;
        }
        if total_module_size < 7 {
            return false;
        }
        let module_size = total_module_size as f64 / 7.0;
        let max_variance = module_size / 2.0;
        // Allow less than 50% variance from 1-1-3-1-1 proportions
        ((module_size - state_count[0] as f64).abs()) < max_variance
            && ((module_size - state_count[1] as f64).abs()) < max_variance
            && ((3.0 * module_size - state_count[2] as f64).abs()) < 3.0 * max_variance
            && (module_size - state_count[3] as f64).abs() < max_variance
            && (module_size - state_count[4] as f64).abs() < max_variance
    }

    /// - `state_count`: count of black/white/black/white/black pixels just read
    ///
    /// Returns `true` when the diagonal counts are close enough to the 1/1/3/1/1 finder-pattern ratio.
    pub fn found_pattern_diagonal(state_count: &[u32]) -> bool {
        let mut total_module_size = 0;
        for count in state_count.iter().take(5) {
            if *count == 0 {
                return false;
            }
            total_module_size += *count;
        }
        if total_module_size < 7 {
            return false;
        }
        let module_size = total_module_size as f64 / 7.0;
        let max_variance = module_size / 1.333;
        // Allow less than 75% variance from 1-1-3-1-1 proportions
        (module_size - state_count[0] as f64).abs() < max_variance
            && (module_size - state_count[1] as f64).abs() < max_variance
            && (3.0 * module_size - state_count[2] as f64).abs() < 3.0 * max_variance
            && (module_size - state_count[3] as f64).abs() < max_variance
            && (module_size - state_count[4] as f64).abs() < max_variance
    }

    pub fn do_clear_counts(counts: &mut [u32; 5]) {
        counts.fill(0)
    }

    pub fn do_shift_counts2(state_count: &mut [u32]) {
        state_count[0] = state_count[2];
        state_count[1] = state_count[3];
        state_count[2] = state_count[4];
        state_count[3] = 1;
        state_count[4] = 0;
    }

    /// After a vertical and horizontal scan finds a potential finder pattern, this method
    /// "cross-cross-cross-checks" by scanning down diagonally through the center of the possible
    /// finder pattern to see if the same proportion is detected.
    ///
    /// - `center_i`: row where a finder pattern was detected
    /// - `center_j`: center of the section that appears to cross a finder pattern
    ///
    /// Returns true if proportions are withing expected limits.
    fn cross_check_diagonal(&self, center_i: u32, center_j: u32) -> bool {
        let mut cross_check_state_count = [0u32; 5];

        // Start counting up, left from center finding black center mass
        let mut i = 0;
        while center_i >= i && center_j >= i && self.image.get(center_j - i, center_i - i) {
            cross_check_state_count[2] += 1;
            i += 1;
        }
        if cross_check_state_count[2] == 0 {
            return false;
        }

        // Continue up, left finding white space
        while center_i >= i && center_j >= i && !self.image.get(center_j - i, center_i - i) {
            cross_check_state_count[1] += 1;
            i += 1;
        }
        if cross_check_state_count[1] == 0 {
            return false;
        }

        // Continue up, left finding black border
        while center_i >= i && center_j >= i && self.image.get(center_j - i, center_i - i) {
            cross_check_state_count[0] += 1;
            i += 1;
        }
        if cross_check_state_count[0] == 0 {
            return false;
        }

        let max_i = self.image.get_height();
        let max_j = self.image.get_width();

        // Now also count down, right from center
        i = 1;
        while center_i + i < max_i
            && center_j + i < max_j
            && self.image.get(center_j + i, center_i + i)
        {
            cross_check_state_count[2] += 1;
            i += 1;
        }

        while center_i + i < max_i
            && center_j + i < max_j
            && !self.image.get(center_j + i, center_i + i)
        {
            cross_check_state_count[3] += 1;
            i += 1;
        }
        if cross_check_state_count[3] == 0 {
            return false;
        }

        while center_i + i < max_i
            && center_j + i < max_j
            && self.image.get(center_j + i, center_i + i)
        {
            cross_check_state_count[4] += 1;
            i += 1;
        }
        if cross_check_state_count[4] == 0 {
            return false;
        }

        Self::found_pattern_diagonal(&cross_check_state_count)
    }

    /// After a horizontal scan finds a potential finder pattern, this method
    /// "cross-checks" by scanning down vertically through the center of the possible
    /// finder pattern to see if the same proportion is detected.
    ///
    /// - `start_i`: row where a finder pattern was detected
    /// - `center_j`: center of the section that appears to cross a finder pattern
    /// - `max_count`: maximum reasonable number of modules that should be
    ///   observed in any reading state, based on the results of the horizontal scan
    ///
    /// Returns vertical center of finder pattern, or [`Float::NaN`] if not found.
    fn cross_check_vertical(
        &self,
        start_i: u32,
        center_j: u32,
        max_count: u32,
        original_state_count_total: u32,
    ) -> f32 {
        let max_i = self.image.get_height() as i32;
        let mut cross_check_state_count = [0u32; 5];

        // Start counting up from center
        let mut i = start_i as i32;
        while i >= 0 && self.image.get(center_j, i as u32) {
            cross_check_state_count[2] += 1;
            i -= 1;
        }
        if i < 0 {
            return f32::NAN;
        }
        while i >= 0
            && !self.image.get(center_j, i as u32)
            && cross_check_state_count[1] <= max_count
        {
            cross_check_state_count[1] += 1;
            i -= 1;
        }
        // If already too many modules in this state or ran off the edge:
        if i < 0 || cross_check_state_count[1] > max_count {
            return f32::NAN;
        }
        while i >= 0
            && self.image.get(center_j, i as u32)
            && cross_check_state_count[0] <= max_count
        {
            cross_check_state_count[0] += 1;
            i -= 1;
        }
        if cross_check_state_count[0] > max_count {
            return f32::NAN;
        }

        // Now also count down from center
        i = start_i as i32 + 1;
        while i < max_i && self.image.get(center_j, i as u32) {
            cross_check_state_count[2] += 1;
            i += 1;
        }
        if i == max_i {
            return f32::NAN;
        }
        while i < max_i
            && !self.image.get(center_j, i as u32)
            && cross_check_state_count[3] < max_count
        {
            cross_check_state_count[3] += 1;
            i += 1;
        }
        if i == max_i || cross_check_state_count[3] >= max_count {
            return f32::NAN;
        }
        while i < max_i
            && self.image.get(center_j, i as u32)
            && cross_check_state_count[4] < max_count
        {
            cross_check_state_count[4] += 1;
            i += 1;
        }
        if cross_check_state_count[4] >= max_count {
            return f32::NAN;
        }

        // If we found a finder-pattern-like section, but its size is more than 40% different than
        // the original, assume it's a false positive
        let state_count_total = cross_check_state_count.iter().sum::<u32>();

        if 5 * (state_count_total as i64 - original_state_count_total as i64)
            >= 2 * original_state_count_total as i64
        {
            return f32::NAN;
        }

        if Self::found_pattern_cross(&cross_check_state_count) {
            Self::center_from_end(&cross_check_state_count, i as u32)
        } else {
            f32::NAN
        }
    }

    /// Like [`int, int, int)`], and in fact is basically identical,
    /// except it reads horizontally instead of vertically. This is used to cross-cross
    /// check a vertical cross check and locate the real center of the alignment pattern.
    fn cross_check_horizontal(
        &self,
        start_j: u32,
        center_i: u32,
        max_count: u32,
        original_state_count_total: u32,
    ) -> f32 {
        let max_j = self.image.get_width();
        let mut cross_check_state_count = [0u32; 5];

        let mut j = start_j as i32;
        while j >= 0 && self.image.get(j as u32, center_i) {
            cross_check_state_count[2] += 1;
            j -= 1;
        }
        if j < 0 {
            return f32::NAN;
        }

        while j >= 0
            && !self.image.get(j as u32, center_i)
            && cross_check_state_count[1] <= max_count
        {
            cross_check_state_count[1] += 1;
            j -= 1;
        }
        if j < 0 || cross_check_state_count[1] > max_count {
            return f32::NAN;
        }

        while j >= 0
            && self.image.get(j as u32, center_i)
            && cross_check_state_count[0] <= max_count
        {
            cross_check_state_count[0] += 1;
            j -= 1;
        }
        if cross_check_state_count[0] > max_count {
            return f32::NAN;
        }

        j = start_j as i32 + 1;
        while j < (max_j as i32) && self.image.get(j as u32, center_i) {
            cross_check_state_count[2] += 1;
            j += 1;
        }
        if j == max_j as i32 {
            return f32::NAN;
        }

        while j < max_j as i32
            && !self.image.get(j as u32, center_i)
            && cross_check_state_count[3] < max_count
        {
            cross_check_state_count[3] += 1;
            j += 1;
        }
        if j == (max_j as i32) || cross_check_state_count[3] >= max_count {
            return f32::NAN;
        }

        while j < (max_j as i32)
            && self.image.get(j as u32, center_i)
            && cross_check_state_count[4] < max_count
        {
            cross_check_state_count[4] += 1;
            j += 1;
        }
        if cross_check_state_count[4] >= max_count {
            return f32::NAN;
        }

        // If we found a finder-pattern-like section, but its size is significantly different than
        // the original, assume it's a false positive
        let state_count_total = cross_check_state_count.iter().sum::<u32>();

        if 5 * (state_count_total as i64 - original_state_count_total as i64)
            >= original_state_count_total as i64
        {
            return f32::NAN;
        }

        if Self::found_pattern_cross(&cross_check_state_count) {
            Self::center_from_end(&cross_check_state_count, j as u32)
        } else {
            f32::NAN
        }
    }

    /// This is called when a horizontal scan finds a possible alignment pattern. It will
    /// cross check with a vertical scan, and if successful, will, ah, cross-cross-check
    /// with another horizontal scan. This is needed primarily to locate the real horizontal
    /// center of the pattern in cases of extreme skew.
    /// And then we cross-cross-cross check with another diagonal scan.
    ///
    /// If that succeeds the finder pattern location is added to a list that tracks
    /// the number of times each location has been nearly-matched as a finder pattern.
    /// Each additional find is more evidence that the location is in fact a finder
    /// pattern center
    ///
    /// - `state_count`: reading state module counts from horizontal scan
    /// - `i`: row where finder pattern may be found
    /// - `j`: end of possible finder pattern in row
    ///
    /// Returns true if a finder pattern candidate was found this time.
    pub fn handle_possible_center(&mut self, state_count: &[u32], i: u32, j: u32) -> bool {
        let state_count_total =
            state_count[0] + state_count[1] + state_count[2] + state_count[3] + state_count[4];
        let mut center_j = Self::center_from_end(state_count, j);
        let center_i = self.cross_check_vertical(
            i,
            center_j.floor() as u32,
            state_count[2],
            state_count_total,
        );
        if !center_i.is_nan() {
            // Re-cross check
            center_j = self.cross_check_horizontal(
                center_j.floor() as u32,
                center_i.floor() as u32,
                state_count[2],
                state_count_total,
            );
            if !center_j.is_nan()
                && self.cross_check_diagonal(center_i.floor() as u32, center_j.floor() as u32)
            {
                let estimated_module_size = state_count_total as f32 / 7.0;
                let mut found = false;
                for center in self.possible_centers.iter_mut() {
                    // Look for about the same center and module size:
                    if center.about_equals(estimated_module_size, center_i, center_j) {
                        *center =
                            center.combine_estimate(center_i, center_j, estimated_module_size);
                        found = true;
                        break;
                    }
                }
                if !found {
                    let point = FinderPattern::new(center_j, center_i, estimated_module_size);
                    self.possible_centers.push(point);
                    if let Some(rpc) = self.result_point_callback {
                        rpc((&point).into());
                    }
                }
                return true;
            }
        }
        false
    }

    /// Returns number of rows we could safely skip during scanning, based on the first.
    /// two finder patterns that have been located. In some cases their position will
    /// allow us to infer that the third pattern must lie below a certain point farther
    /// down in the image.
    fn find_row_skip(&mut self) -> u32 {
        let max = self.possible_centers.len();
        if max <= 1 {
            return 0;
        }
        let mut first_confirmed_center: Option<&FinderPattern> = None;
        for center in &self.possible_centers {
            if center.get_count() >= Self::CENTER_QUORUM {
                if let Some(fnp) = first_confirmed_center {
                    // We have two confirmed centers
                    // How far down can we skip before resuming looking for the next
                    // pattern? In the worst case, only the difference between the
                    // difference in the x / y coordinates of the two centers.
                    // This is the case where you find top left last.
                    self.has_skipped = true;

                    return (Point::from(fnp) - Point::from(center))
                        .abs()
                        .fold(|x, y| x - y)
                        .div(2.0)
                        .floor() as u32;
                } else {
                    first_confirmed_center.replace(center);
                }
            }
        }
        0
    }

    /// Returns `true` when at least three finder patterns have been confirmed
    /// often enough and their estimated module sizes are similar.
    fn have_multiply_confirmed_centers(&self) -> bool {
        let mut confirmed_count = 0;
        let mut total_module_size = 0.0;
        let max = self.possible_centers.len();
        for pattern in &self.possible_centers {
            if pattern.get_count() >= Self::CENTER_QUORUM {
                confirmed_count += 1;
                total_module_size += pattern.get_estimated_module_size();
            }
        }
        if confirmed_count < 3 {
            return false;
        }
        // OK, we have at least 3 confirmed centers, but, it's possible that one is a "false positive"
        // and that we need to keep looking. We detect this by asking if the estimated module sizes
        // vary too much. We arbitrarily say that when the total deviation from average exceeds
        // 5% of the total module size estimates, it's too much.
        let average = total_module_size / max as f32;
        let total_deviation = self.possible_centers.iter().fold(0.0, |acc, pattern| {
            acc + (pattern.get_estimated_module_size() - average).abs()
        });

        total_deviation <= 0.05 * total_module_size
    }

    /// Get square of distance between a and b.
    fn squared_distance(a: &FinderPattern, b: &FinderPattern) -> f64 {
        Point::from(a).squared_distance(Point::from(b)) as f64
    }

    /// Returns the three best finder-pattern candidates.
    ///
    /// The best candidates have similar module sizes and form a shape close to
    /// an isosceles right triangle. Returns a not-found error if three such
    /// patterns do not exist.
    fn select_best_patterns(&mut self) -> Result<[FinderPattern; 3]> {
        let start_size = self.possible_centers.len();
        if start_size < 3 {
            // Couldn't find enough finder patterns
            return Err(Error::NotFound {
                message: "barcode pattern was not detected".to_owned(),
            }
            .into());
        }

        self.possible_centers
            .retain(|fp| fp.get_count() >= Self::CENTER_QUORUM);

        self.possible_centers.sort_unstable_by(|x, y| {
            x.get_estimated_module_size()
                .partial_cmp(&y.get_estimated_module_size())
                .unwrap_or(std::cmp::Ordering::Less) // we are making a weird assumption that uncomparable items are result in Less
        });

        let mut distortion = f64::MAX;
        let mut best_patterns = [None; 3];

        for i in 0..self.possible_centers.len() {
            let fpi = &self.possible_centers[i];
            let min_module_size = fpi.get_estimated_module_size();

            for j in (i + 1)..(self.possible_centers.len() - 1) {
                let fpj = &self.possible_centers[j];
                let squares0 = Self::squared_distance(fpi, fpj);

                for k in (j + 1)..(self.possible_centers.len()) {
                    let fpk = &self.possible_centers[k];
                    let max_module_size = fpk.get_estimated_module_size();
                    if max_module_size > min_module_size * 1.4 {
                        // module size is not similar
                        continue;
                    }

                    let mut sides = [
                        squares0,
                        Self::squared_distance(fpj, fpk),
                        Self::squared_distance(fpi, fpk),
                    ];
                    sides.sort_unstable_by(|x, y| {
                        x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Less)
                    });
                    let [a, b, c] = sides;

                    // a^2 + b^2 = c^2 (Pythagorean theorem), and a = b (isosceles triangle).
                    // Since any right triangle satisfies the formula c^2 - b^2 - a^2 = 0,
                    // we need to check both two equal sides separately.
                    // The value of |c^2 - 2 * b^2| + |c^2 - 2 * a^2| increases as dissimilarity
                    // from isosceles right triangle.
                    let d = (c - 2.0 * b).abs() + (c - 2.0 * a).abs();
                    if d < distortion {
                        distortion = d;
                        best_patterns = [Some(*fpi), Some(*fpj), Some(*fpk)];
                    }
                }
            }
        }

        if distortion == f64::MAX {
            return Err(Error::NotFound {
                message: "barcode pattern was not detected".to_owned(),
            }
            .into());
        }

        let p1 = best_patterns[0].ok_or(Error::NotFound {
            message: "barcode pattern was not detected".to_owned(),
        })?;
        let p2 = best_patterns[1].ok_or(Error::NotFound {
            message: "barcode pattern was not detected".to_owned(),
        })?;
        let p3 = best_patterns[2].ok_or(Error::NotFound {
            message: "barcode pattern was not detected".to_owned(),
        })?;

        Ok([p1, p2, p3])
    }
}
