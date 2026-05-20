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

use super::FinderPattern;

/// Encapsulates information about finder patterns in an image, including the location of
/// the three finder patterns, and their estimated module size.
pub struct FinderPatternInfo {
    bottom_left: FinderPattern,
    top_left: FinderPattern,
    top_right: FinderPattern,
}

impl FinderPatternInfo {
    /// Expects the order to be [bottom_left, top_left, top_right]
    pub fn new(pattern_centers: [FinderPattern; 3]) -> Self {
        let [a, b, c] = pattern_centers;
        Self {
            bottom_left: a,
            top_left: b,
            top_right: c,
        }
    }

    pub fn get_bottom_left(&self) -> &FinderPattern {
        &self.bottom_left
    }

    pub fn get_top_left(&self) -> &FinderPattern {
        &self.top_left
    }

    pub fn get_top_right(&self) -> &FinderPattern {
        &self.top_right
    }
}
