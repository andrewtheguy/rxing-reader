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

use crate::{Point, point};

/// Encapsulates an alignment pattern, which are the smaller square patterns found in
/// all but the simplest QR Codes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignmentPattern {
    estimated_module_size: f32,
    point: Point,
}

impl From<&AlignmentPattern> for Point {
    fn from(value: &AlignmentPattern) -> Self {
        value.point
    }
}

impl From<AlignmentPattern> for Point {
    fn from(value: AlignmentPattern) -> Self {
        value.point
    }
}

impl AlignmentPattern {
    pub fn new(pos_x: f32, pos_y: f32, estimated_module_size: f32) -> Self {
        Self {
            estimated_module_size,
            point: point(pos_x, pos_y),
        }
    }

    /// Determines if this alignment pattern "about equals" an alignment pattern at the stated
    /// position and size -- meaning, it is at nearly the same center with nearly the same size.
    pub fn about_equals(&self, module_size: f32, i: f32, j: f32) -> bool {
        if (i - self.point.y).abs() <= module_size && (j - self.point.x).abs() <= module_size {
            let module_size_diff = (module_size - self.estimated_module_size).abs();
            return module_size_diff <= 1.0 || module_size_diff <= self.estimated_module_size;
        }
        false
    }

    /// Combines this object's current estimate of an alignment pattern position and module size
    /// with a new estimate. It returns a new `AlignmentPattern` containing an average of the two.
    pub fn combine_estimate(&self, i: f32, j: f32, new_module_size: f32) -> AlignmentPattern {
        let combined_x = (self.point.x + j) / 2.0;
        let combined_y = (self.point.y + i) / 2.0;
        let combined_module_size = (self.estimated_module_size + new_module_size) / 2.0;
        AlignmentPattern::new(combined_x, combined_y, combined_module_size)
    }
}
