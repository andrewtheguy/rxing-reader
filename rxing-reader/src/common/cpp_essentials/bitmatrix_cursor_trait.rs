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

    fn is_in(&self, p: Point) -> bool;
    fn is_in_self(&self) -> bool;
    fn is_black(&self) -> bool;

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

    fn p(&self) -> Point;

    fn d(&self) -> Point;

    fn img(&self) -> &BitMatrix;
}
