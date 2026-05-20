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

use crate::{Error, PointCallback, common::BitMatrix};

use super::AlignmentPattern;

/**
 * <p>This class attempts to find alignment patterns in a QR Code. Alignment patterns look like finder
 * patterns but are smaller and appear at regular intervals throughout the image.</p>
 *
 * <p>At the moment this only looks for the bottom-right alignment pattern.</p>
 *
 * <p>This is mostly a simplified copy of {@link FinderPatternFinder}. It is copied,
 * pasted and stripped down here for maximum performance but does unfortunately duplicate
 * some code.</p>
 *
 * <p>This class is thread-safe but not reentrant. Each thread must allocate its own object.</p>
 *
 * @author Sean Owen
 */
pub struct AlignmentPatternFinder<'a> {
    image: &'a BitMatrix,
    possible_centers: Vec<AlignmentPattern>,
    start_x: u32,
    start_y: u32,
    width: u32,
    height: u32,
    module_size: f32,
    result_point_callback: Option<&'a PointCallback>,
}

impl<'a> AlignmentPatternFinder<'a> {
    /**
     * <p>Creates a finder that will look in a portion of the whole image.</p>
     *
     * @param image image to search
     * @param start_x left column from which to start searching
     * @param start_y top row from which to start searching
     * @param width width of region to search
     * @param height height of region to search
     * @param module_size estimated module size so far
     */
    pub fn new(
        image: &'a BitMatrix,
        start_x: u32,
        start_y: u32,
        width: u32,
        height: u32,
        module_size: f32,
        result_point_callback: Option<&'a PointCallback>,
    ) -> Self {
        Self {
            image,
            possible_centers: Vec::with_capacity(5),
            start_x,
            start_y,
            width,
            height,
            module_size,
            result_point_callback,
        }
    }

    /**
     * <p>This method attempts to find the bottom-right alignment pattern in the image. It is a bit messy since
     * it's pretty performance-critical and so is written to be fast foremost.</p>
     *
     * @return {@link AlignmentPattern} if found
     * Returns a not-found error if not found
     */
    pub fn find(&mut self) -> Result<AlignmentPattern> {
        let start_x = self.start_x;
        let height = self.height;
        let max_j = start_x + self.width;
        let middle_i = self.start_y + (height / 2);
        // We are looking for black/white/black modules in 1:1:1 ratio;
        // this tracks the number of black/white/black modules seen so far
        let mut state_count = [0u32; 3];
        for i_gen in 0..height {
            // Search from middle outwards
            let i = (middle_i as i32
                + (if (i_gen & 0x01) == 0 {
                    (i_gen as i32 + 1) / 2
                } else {
                    -((i_gen as i32 + 1) / 2)
                })) as u32;

            state_count.fill(0);

            let mut j = start_x;
            // Burn off leading white pixels before anything else; if we start in the middle of
            // a white run, it doesn't make sense to count its length, since we don't know if the
            // white run continued to the left of the start point
            while j < max_j && !self.image.get(j, i) {
                j += 1;
            }
            let mut current_state = 0;
            while j < max_j {
                if self.image.get(j, i) {
                    // Black pixel
                    if current_state == 1 {
                        // Counting black pixels
                        state_count[1] += 1;
                    } else {
                        // Counting white pixels
                        if current_state == 2 {
                            // A winner?
                            if self.found_pattern_cross(&state_count) {
                                // Yes
                                if let Some(confirmed) =
                                    self.handle_possible_center(&state_count, i, j)
                                {
                                    return Ok(confirmed);
                                }
                            }
                            state_count[0] = state_count[2];
                            state_count[1] = 1;
                            state_count[2] = 0;
                            current_state = 1;
                        } else {
                            current_state += 1;
                            state_count[current_state] += 1;
                        }
                    }
                } else {
                    // White pixel
                    if current_state == 1 {
                        // Counting black pixels
                        current_state += 1;
                    }
                    state_count[current_state] += 1;
                }
                j += 1;
            }
            if self.found_pattern_cross(&state_count)
                && let Some(confirmed) = self.handle_possible_center(&state_count, i, max_j)
            {
                return Ok(confirmed);
            }
        }

        // Hmm, nothing we saw was observed and confirmed twice. If we had
        // any guess at all, return it.
        if !self.possible_centers.is_empty() {
            Ok(self.possible_centers[0])
        } else {
            Err(Error::NotFound {
                message: "barcode pattern was not detected".to_owned(),
            }
            .into())
        }
    }

    /**
     * Given a count of black/white/black pixels just seen and an end position,
     * figures the location of the center of this black/white/black run.
     */
    #[inline]
    fn center_from_end(state_count: &[u32], end: u32) -> f32 {
        (end as f32 - state_count[2] as f32) - state_count[1] as f32 / 2.0
    }

    /**
     * @param state_count count of black/white/black pixels just read
     * @return true iff the proportions of the counts is close enough to the 1/1/1 ratios
     *         used by alignment patterns to be considered a match
     */
    fn found_pattern_cross(&self, state_count: &[u32]) -> bool {
        let module_size = self.module_size;
        let max_variance = module_size / 2.0;
        for state in state_count.iter().take(3) {
            if (module_size - *state as f32).abs() >= max_variance {
                return false;
            }
        }
        true
    }

    /**
     * <p>After a horizontal scan finds a potential alignment pattern, this method
     * "cross-checks" by scanning down vertically through the center of the possible
     * alignment pattern to see if the same proportion is detected.</p>
     *
     * @param start_i row where an alignment pattern was detected
     * @param center_j center of the section that appears to cross an alignment pattern
     * @param max_count maximum reasonable number of modules that should be
     * observed in any reading state, based on the results of the horizontal scan
     * @return vertical center of alignment pattern, or {@link Float#NaN} if not found
     */
    fn cross_check_vertical(
        &self,
        start_i: u32,
        center_j: u32,
        max_count: u32,
        original_state_count_total: u32,
    ) -> f32 {
        let image = &self.image;

        let max_i = image.get_height();
        let mut cross_check_state_count = [0u32; 3];

        // Start counting up from center
        let mut i = start_i as i32;
        while i >= 0 && image.get(center_j, i as u32) && cross_check_state_count[1] <= max_count {
            cross_check_state_count[1] += 1;
            i -= 1;
        }
        // If already too many modules in this state or ran off the edge:
        if i < 0 || cross_check_state_count[1] > max_count {
            return f32::NAN;
        }
        while i >= 0 && !image.get(center_j, i as u32) && cross_check_state_count[0] <= max_count {
            cross_check_state_count[0] += 1;
            i -= 1;
        }
        if cross_check_state_count[0] > max_count {
            return f32::NAN;
        }

        // Now also count down from center
        i = start_i as i32 + 1;
        while i < max_i as i32
            && image.get(center_j, i as u32)
            && cross_check_state_count[1] <= max_count
        {
            cross_check_state_count[1] += 1;
            i += 1;
        }
        if i == max_i as i32 || cross_check_state_count[1] > max_count {
            return f32::NAN;
        }
        while i < max_i as i32
            && !image.get(center_j, i as u32)
            && cross_check_state_count[2] <= max_count
        {
            cross_check_state_count[2] += 1;
            i += 1;
        }
        if cross_check_state_count[2] > max_count {
            return f32::NAN;
        }

        let state_count_total =
            cross_check_state_count[0] + cross_check_state_count[1] + cross_check_state_count[2];
        let diff: u64 = if state_count_total >= original_state_count_total {
            (state_count_total - original_state_count_total) as u64
        } else {
            (original_state_count_total - state_count_total) as u64
        };
        if 5 * diff >= 2 * original_state_count_total as u64 {
            return f32::NAN;
        }

        if self.found_pattern_cross(&cross_check_state_count) {
            Self::center_from_end(&cross_check_state_count, i as u32)
        } else {
            f32::NAN
        }
    }

    /**
     * <p>This is called when a horizontal scan finds a possible alignment pattern. It will
     * cross check with a vertical scan, and if successful, will see if this pattern had been
     * found on a previous horizontal scan. If so, we consider it confirmed and conclude we have
     * found the alignment pattern.</p>
     *
     * @param state_count reading state module counts from horizontal scan
     * @param i row where alignment pattern may be found
     * @param j end of possible alignment pattern in row
     * @return {@link AlignmentPattern} if we have found the same pattern twice, or null if not
     */
    fn handle_possible_center(
        &mut self,
        state_count: &[u32],
        i: u32,
        j: u32,
    ) -> Option<AlignmentPattern> {
        let state_count_total = state_count[0] + state_count[1] + state_count[2];
        let center_j = Self::center_from_end(state_count, j);
        let center_i = self.cross_check_vertical(
            i,
            center_j.floor() as u32,
            2 * state_count[1],
            state_count_total,
        );

        if !center_i.is_nan() {
            let estimated_module_size =
                (state_count[0] + state_count[1] + state_count[2]) as f32 / 3.0;
            for center in &self.possible_centers {
                // Look for about the same center and module size:
                if center.about_equals(estimated_module_size, center_i, center_j) {
                    return Some(center.combine_estimate(
                        center_i,
                        center_j,
                        estimated_module_size,
                    ));
                }
            }
            // Hadn't found this before; save it
            let point = AlignmentPattern::new(center_j, center_i, estimated_module_size);
            if let Some(rpc) = self.result_point_callback {
                rpc((&point).into());
            }

            self.possible_centers.push(point);
        }

        None
    }
}
