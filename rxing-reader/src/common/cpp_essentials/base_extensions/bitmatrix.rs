use crate::Point;
use crate::common::BitMatrix;
use crate::point;
use anyhow::Result;

impl BitMatrix {
    pub fn deflate(
        &self,
        width: u32,
        height: u32,
        top: f32,
        left: f32,
        sub_sampling: f32,
    ) -> Result<Self> {
        let mut result = BitMatrix::new(width, height)?;

        for y in 0..result.height() {
            let y_offset = top + y as f32 * sub_sampling;
            for x in 0..result.width() {
                let sample_x = left + x as f32 * sub_sampling;
                if sample_x >= 0.0
                    && sample_x < self.width() as f32
                    && y_offset >= 0.0
                    && y_offset < self.height() as f32
                    && self.at_point(point(sample_x, y_offset))
                {
                    result.set(x, y);
                }
            }
        }

        Ok(result)
    }

    pub fn find_bounding_box(
        &self,
        left: u32,
        top: u32,
        width: u32,
        height: u32,
        min_size: u32,
    ) -> (bool, u32, u32, u32, u32) {
        let mut left = left;
        let mut top = top;
        let mut width = width;
        let mut height = height;

        let (Some(Point { x: left_on_bit, y: top_on_bit }), Some(Point { x: right_on_bit, y: bottom_on_bit })) =
            (self.top_left_on_bit(), self.bottom_right_on_bit())
        else {
            return (false, left, top, width, height);
        };
        left = left_on_bit as u32;
        top = top_on_bit as u32;
        let mut right = right_on_bit as u32;
        let bottom = bottom_on_bit as u32;
        if bottom - top + 1 < min_size || right - left + 1 < min_size {
            return (false, left, top, width, height);
        }

        for y in top..=bottom {
            for x in 0..left {
                if self.get(x, y) {
                    left = x;
                    break;
                }
            }
            for x in ((right + 1)..self.width()).rev() {
                if self.get(x, y) {
                    right = x;
                    break;
                }
            }
        }

        width = right - left + 1;
        height = bottom - top + 1;

        (
            width >= min_size && height >= min_size,
            left,
            top,
            width,
            height,
        )
    }
}
