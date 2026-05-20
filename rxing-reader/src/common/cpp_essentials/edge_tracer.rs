use crate::{Point, common::BitMatrix};

use super::{BitMatrixCursorTrait, Direction, Value};

#[derive(Clone, Debug)]
pub struct EdgeTracer<'a> {
    pub(crate) img: &'a BitMatrix,

    pub(crate) p: Point, // current position
    d: Point,            // current direction
}

impl BitMatrixCursorTrait for EdgeTracer<'_> {
    fn test_at(&self, p: Point) -> Value {
        if self.img.is_in_with_border(p, 0) {
            Value::from(self.img.at_point(p))
        } else {
            Value::Invalid
        }
    }

    fn is_in(&self, p: Point) -> bool {
        self.img.is_in_with_border(p, 0)
    }

    fn is_in_self(&self) -> bool {
        self.is_in(self.p)
    }

    fn is_black(&self) -> bool {
        self.black_at(self.p)
    }

    fn front(&self) -> &Point {
        &self.d
    }

    fn back(&self) -> Point {
        Point {
            x: -self.d.x,
            y: -self.d.y,
        }
    }

    fn left(&self) -> Point {
        Point {
            x: self.d.y,
            y: -self.d.x,
        }
    }

    fn right(&self) -> Point {
        Point {
            x: -self.d.y,
            y: self.d.x,
        }
    }

    fn turn_back(&mut self) {
        self.d = self.back()
    }

    fn turn_left(&mut self) {
        self.d = self.left()
    }

    fn turn_right(&mut self) {
        self.d = self.right()
    }

    fn turn(&mut self, dir: Direction) {
        self.d = self.direction(dir)
    }

    fn edge_at_point(&self, d: Point) -> Value {
        let v = self.test_at(self.p);
        if self.test_at(self.p + d) != v {
            v
        } else {
            Value::Invalid
        }
    }

    fn set_direction(&mut self, dir: Point) {
        self.d = dir.bresenham_direction();
    }

    fn step_by(&mut self, distance: f32) -> bool {
        self.p += self.d * distance;
        self.is_in(self.p)
    }

    fn turned_back(&self) -> Self {
        Self {
            img: self.img,
            p: self.p,
            d: self.back(),
        }
    }

    /// - `nth`: number of edges to pass
    /// - `range`: max number of steps to take
    /// - `backup`: whether or not to backup one step so we land in front of the edge
    ///
    /// Returns number of steps taken or 0 if moved outside of range/image.
    fn step_to_edge(&mut self, nth: i32, range: i32, backup: bool) -> i32 {
        let mut nth = nth;
        // TODO: provide an alternative and faster out-of-bounds check than is_in() inside test_at()
        let mut steps = 0;
        let mut lv = self.test_at(self.p);

        while nth > 0 && (range <= 0 || steps < range) && lv.is_valid() {
            steps += 1;
            let v = self.test_at(self.p + steps * self.d);
            if lv != v {
                lv = v;
                nth -= 1;
            }
        }
        if backup {
            steps -= 1;
        }
        self.p += self.d * steps;
        steps * i32::from(nth == 0)
    }

    fn p(&self) -> Point {
        self.p
    }

    fn d(&self) -> Point {
        self.d
    }

    fn img(&self) -> &BitMatrix {
        self.img
    }
}

impl<'a> EdgeTracer<'_> {
    pub fn new(image: &'a BitMatrix, p: Point, d: Point) -> EdgeTracer<'a> {
        EdgeTracer {
            img: image,
            p,
            d: Point::bresenham_direction(d),
        }
    }
}
