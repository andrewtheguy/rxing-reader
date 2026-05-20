use anyhow::Result;

use crate::{Error, common::BitMatrix};

use super::BitMatrixCursorTrait;

pub struct FastEdgeToEdgeCounter<'a> {
    p: u32,
    stride: isize,
    steps_to_border: i32,
    _arr: isize,
    under_array: &'a BitMatrix,
}

impl FastEdgeToEdgeCounter<'_> {
    pub fn new<T: BitMatrixCursorTrait>(cur: &'_ T) -> Result<FastEdgeToEdgeCounter<'_>> {
        let stride = cur.d().y as isize * cur.img().width() as isize + cur.d().x as isize;
        // Cursor positions use the BitMatrix row-major index. Keep row/column
        // signed until after the bounds check so negative rows do not mirror to
        // a different row during reverse traversal.
        let width = cur.img().width() as isize;
        let height = cur.img().height() as isize;
        let p = cur.p().y as isize * width + cur.p().x as isize;
        let image_len = width
            .checked_mul(height)
            .ok_or_else(|| Error::InvalidArgument {
                message: format!("FastEdgeToEdgeCounter: image size overflow ({width} x {height})").into(),
            })?;
        if !(0..image_len).contains(&p) {
            return Err(Error::InvalidArgument {
                message: format!(
                    "FastEdgeToEdgeCounter: cursor index {p} is outside image of size {width}x{height} (cursor=({}, {}))",
                    cur.p().x,
                    cur.p().y,
                )
                .into(),
            }
            .into());
        }
        let p = p as u32;

        let max_steps_x: i32 = if cur.d().x != 0.0 {
            if cur.d().x > 0.0 {
                cur.img().width() as i32 - 1 - cur.p().x as i32
            } else {
                cur.p().x as i32
            }
        } else {
            i32::MAX
        };
        let max_steps_y: i32 = if cur.d().y != 0.0 {
            if cur.d().y > 0.0 {
                cur.img().height() as i32 - 1 - cur.p().y as i32
            } else {
                cur.p().y as i32
            }
        } else {
            i32::MAX
        };
        let steps_to_border = std::cmp::min(max_steps_x, max_steps_y);

        Ok(FastEdgeToEdgeCounter {
            p,
            stride,
            steps_to_border,
            _arr: cur.p().y as isize * stride,
            under_array: cur.img(),
        })
    }

    pub fn step_to_next_edge(&mut self, range: u32) -> u32 {
        let max_steps = std::cmp::min(self.steps_to_border, range as i32);
        let mut steps = 0;
        loop {
            steps += 1;
            if steps > max_steps {
                if max_steps == self.steps_to_border {
                    break;
                } else {
                    return 0;
                }
            }

            let Some(idx_pt) = self.get_array_check_index(steps) else {
                return 0;
            };

            if self.under_array.at_index(idx_pt) != self.under_array.at_index(self.p as usize) {
                break;
            }
        }

        // Saturate at 0 instead of wrapping via unsigned_abs. The loop above can
        // exit one step past the border (the `max_steps == steps_to_border` break path
        // increments `steps` to `max_steps + 1`), which can yield a negative index
        // for negative stride. Caller never re-reads `self.p` in that terminal case,
        // but clamping keeps `self.p` in a defined state.
        let new_pos = self.p as isize + (steps as isize * self.stride);
        self.p = new_pos.max(0) as u32;
        self.steps_to_border -= steps;

        steps as u32
    }

    #[inline(always)]
    fn get_array_check_index(&self, steps: i32) -> Option<usize> {
        let idx = self.p as isize + (steps as isize * self.stride);
        if idx < 0 {
            return None;
        }
        Some(idx as usize)
    }
}
