use crate::{Point, common::BitMatrix};

use super::{Direction, Value, util::opposite};

/// The current position and direction are `PointT<T>` values. Depending on the
/// concrete point type, this can traverse the image in Bresenham style
/// (`PointF`) or in discrete horizontal, vertical, and diagonal steps (`PointI`).
pub trait BitMatrixCursorTrait {
    fn test_at(&self, p: Point) -> Value;

    fn black_at(&self, pos: Point) -> bool {
        self.test_at(pos).is_black()
    }
    fn white_at(&self, pos: Point) -> bool {
        self.test_at(pos).is_white()
    }

    fn is_in(&self, p: Point) -> bool;
    fn is_in_self(&self) -> bool;
    fn is_black(&self) -> bool;
    fn is_white(&self) -> bool;

    fn front(&self) -> &Point;
    fn back(&self) -> Point;
    fn left(&self) -> Point;
    fn right(&self) -> Point;
    fn direction(&self, dir: Direction) -> Point {
        self.right() * Into::<i32>::into(dir)
    }

    fn turn_back(&mut self);
    fn turn_left(&mut self);
    fn turn_right(&mut self);
    fn turn(&mut self, dir: Direction);

    fn edge_at_point(&self, d: Point) -> Value;

    fn edge_at_front(&self) -> Value {
        self.edge_at_point(*self.front())
    }
    fn edge_at_back(&self) -> Value {
        self.edge_at_point(self.back())
    }
    fn edge_at_left(&self) -> Value {
        self.edge_at_point(self.left())
    }
    fn edge_at_right(&self) -> Value {
        self.edge_at_point(self.right())
    }
    fn edge_at_direction(&self, dir: Direction) -> Value {
        self.edge_at_point(self.direction(dir))
    }

    fn set_direction(&mut self, dir: Point);

    fn step(&mut self, s: Option<f32>) -> bool;

    fn moved_by(self, d: Point) -> Self;
    fn turned_back(&self) -> Self;

    /// - `nth`: number of edges to pass
    /// - `range`: max number of steps to take
    /// - `backup`: whether or not to backup one step so we land in front of the edge
    ///
    /// Returns number of steps taken or 0 if moved outside of range/image.
    fn step_to_edge(&mut self, nth: Option<i32>, range: Option<i32>, backup: Option<bool>) -> i32;

    fn step_along_edge(&mut self, dir: Direction, skip_corner: Option<bool>) -> bool {
        let skip_corner = skip_corner.unwrap_or_default();

        if !self.edge_at_direction(dir).is_valid() {
            self.turn(dir);
        } else if self.edge_at_front().is_valid() {
            self.turn(opposite(dir));
            if self.edge_at_front().is_valid() {
                self.turn(opposite(dir));
                if self.edge_at_front().is_valid() {
                    return false;
                }
            }
        }

        let mut ret = self.step(None);

        if ret && skip_corner && !self.edge_at_direction(dir).is_valid() {
            self.turn(dir);
            ret = self.step(None);
        }

        ret
    }

    fn count_edges(&mut self, range: i32) -> i32 {
        let mut res = 0;
        let mut range = range;

        let mut steps;

        while {
            steps = if range == 0 {
                0
            } else {
                self.step_to_edge(Some(1), Some(range), None)
            };
            steps > 0
        } {
            range -= steps;
            res += 1;
        }

        res
    }

    fn p(&self) -> Point;

    fn d(&self) -> Point;

    fn img(&self) -> &BitMatrix;

    fn read_pattern<const LEN: usize, T: TryFrom<i32> + Default + Copy>(
        &mut self,
        range: Option<i32>,
    ) -> Option<[T; LEN]> {
        let range = range.unwrap_or(0);
        let mut res = [T::default(); LEN];
        for i in res.iter_mut() {
            *i = self
                .step_to_edge(Some(1), Some(range), None)
                .try_into()
                .ok()?;
        }
        Some(res)
    }

    fn read_pattern_from_black<const LEN: usize, T: TryFrom<i32> + Default + Copy>(
        &mut self,
        max_white_prefix: i32,
        range: Option<i32>,
    ) -> Option<[T; LEN]> {
        let range = range.unwrap_or(0);
        if max_white_prefix != 0
            && self.is_white()
            && self.step_to_edge(Some(1), Some(max_white_prefix), None) == 0
        {
            return None;
        }
        self.read_pattern::<LEN, _>(Some(range))
    }
}
