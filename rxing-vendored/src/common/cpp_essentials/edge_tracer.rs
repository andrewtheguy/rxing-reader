use std::sync::{Arc, RwLock};

use anyhow::Result;

use crate::{Error, Point, common::BitMatrix};

use crate::common::cpp_essentials::ByteMatrix;

use super::{BitMatrixCursorTrait, Direction, RegressionLineTrait, StepResult, Value};

#[derive(Clone, Debug)]
pub struct EdgeTracer<'a> {
    pub(crate) img: &'a BitMatrix,

    pub(crate) p: Point, // current position
    d: Point,            // current direction

    pub history: Option<Arc<RwLock<ByteMatrix>>>,
    pub state: i32,
}

impl BitMatrixCursorTrait for EdgeTracer<'_> {
    fn test_at(&self, p: Point) -> Value {
        if self.img.is_in_with_border(p, 0) {
            Value::from(self.img.get_point(p))
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

    fn is_white(&self) -> bool {
        self.white_at(self.p)
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

    fn step(&mut self, s: Option<f32>) -> bool {
        let s = s.unwrap_or(1.0);
        self.p += self.d * s;
        self.is_in(self.p)
    }

    fn moved_by(self, d: Point) -> Self {
        let mut res = self;
        res.p += d;

        res
    }

    fn turned_back(&self) -> Self {
        let mut res = self.clone();
        res.d = res.back();

        res
    }

    /**
     * @brief step_to_edge advances cursor to one step behind the next (or n-th) edge.
     * @param nth number of edges to pass
     * @param range max number of steps to take
     * @param backup whether or not to backup one step so we land in front of the edge
     * @return number of steps taken or 0 if moved outside of range/image
     */
    fn step_to_edge(&mut self, nth: Option<i32>, range: Option<i32>, backup: Option<bool>) -> i32 {
        let mut nth = nth.unwrap_or(1);
        let range = range.unwrap_or(0);
        let backup = backup.unwrap_or(false);
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
            history: None,
            state: 0,
        }
    }

    fn trace_step(
        &mut self,
        d_edge: Point,
        max_step_size: i32,
        good_direction: bool,
    ) -> Result<StepResult> {
        let d_edge = Point::main_direction(d_edge);
        for breadth in 1..=(if max_step_size == 1 {
            2
        } else if good_direction {
            1
        } else {
            3
        }) {
            for step in 1..=max_step_size {
                for i in 0..=(2 * (step / 4 + 1) * breadth) {
                    let mut p_edge = self.p
                        + step * self.d
                        + (if i & 1 > 0 { (i + 1) / 2 } else { -i / 2 }) * d_edge;

                    if !self.black_at(p_edge + d_edge) {
                        continue;
                    }

                    // found black pixel -> go 'outward' until we hit the b/w border
                    for _j in 0..(std::cmp::max(max_step_size, 3)) {
                        if self.white_at(p_edge) {
                            // if we are not making any progress, we still have another endless loop bug
                            if self.p == p_edge.centered() {
                                return Err(Error::InvalidState { message: "required internal state is missing".to_owned() }.into());
                            }
                            self.p = p_edge.centered();

                            if let Some(history) = &self.history
                                && max_step_size == 1
                            {
                                if history
                                    .read()
                                    .map_err(|_| {
                                        Error::InvalidState { message: "Failed to acquire read lock".to_owned() }
                                    })?
                                    .get(self.p.x as u32, self.p.y as u32)
                                    == self.state as u8
                                {
                                    return Ok(StepResult::ClosedEnd);
                                }
                                history
                                    .write()
                                    .map_err(|_| {
                                        Error::InvalidState { message: "Failed to acquire write lock".to_owned() }
                                    })?
                                    .set(self.p.x as u32, self.p.y as u32, self.state as u8);
                            }

                            return Ok(StepResult::Found);
                        }
                        p_edge -= d_edge;
                        if self.black_at(p_edge - self.d) {
                            p_edge -= self.d;
                        }

                        if !self.is_in(p_edge) {
                            break;
                        }
                    }
                    // no valid b/w border found within reasonable range
                    return Ok(StepResult::ClosedEnd);
                }
            }
        }
        Ok(StepResult::OpenEnd)
    }

    pub fn update_direction_from_origin(&mut self, origin: Point) -> bool {
        let old_d = self.d;
        self.set_direction(self.p - origin);
        // if the new direction is pointing "backward", i.e. angle(new, old) > 90 deg -> break
        if Point::dot(self.d, old_d) < 0.0 {
            return false;
        }
        // make sure d stays in the same quadrant to prevent an infinite loop
        if (self.d.x).abs() == (self.d.y).abs() {
            self.d = Point::main_direction(old_d) + 0.99 * (self.d - Point::main_direction(old_d));
        } else if Point::main_direction(self.d) != Point::main_direction(old_d) {
            self.d = Point::main_direction(old_d) + 0.99 * Point::main_direction(self.d);
        }
        true
    }

    pub fn trace_line<T: RegressionLineTrait>(
        &mut self,
        d_edge: Point,
        line: &mut T,
    ) -> Result<bool> {
        line.set_direction_inward(d_edge);
        loop {
            line.add(self.p)?;
            if line.points().len() % 50 == 10 {
                if !line.evaluate_max_distance(None, None) {
                    return Ok(false);
                }
                let first_point = line.points().first().copied().ok_or_else(|| {
                    Error::InvalidState { message: "trace line has no anchor point".to_owned() }
                })?;
                if !self.update_direction_from_origin(
                    self.p - line.project(self.p) + first_point,
                ) {
                    return Ok(false);
                }
            }
            let step_result = self.trace_step(d_edge, 1, line.is_valid())?;
            if step_result != StepResult::Found {
                return Ok(step_result == StepResult::OpenEnd && line.points().len() > 1);
            }
        }
    }

    pub fn trace_gaps<T: RegressionLineTrait>(
        &mut self,
        d_edge: Point,
        line: &mut T,
        max_step_size: i32,
        finish_line: &mut T,
    ) -> Result<bool> {
        let mut max_step_size = max_step_size;
        let max_steps_per_gap = max_step_size;
        let mut steps = 0;
        line.set_direction_inward(d_edge);
        let mut gaps = 0;
        let mut last_p = Point { x: 0.0, y: 0.0 };
        loop {
            // detect an endless loop (lack of progress). if encountered, please report.
            // this fixes a deadlock in falsepositives-1/#570.png and the regression in #574
            steps += 1;
            if self.p == std::mem::replace(&mut last_p, self.p)
                || steps > (if gaps == 0 { 2 } else { gaps + 1 }) * max_steps_per_gap
            {
                return Ok(false);
            }

            if line.points().last().is_some_and(|last| self.p == *last) {
                return Ok(false);
            }

            // if we drifted too far outside of the code, break
            if line.is_valid()
                && line.signed_distance(self.p) < -5.0
                && (!line.evaluate_max_distance(None, None) || line.signed_distance(self.p) < -5.0)
            {
                return Ok(false);
            }

            // if we are drifting towards the inside of the code, pull the current position back out onto the line
            if line.is_valid() && line.signed_distance(self.p) > 3.0 {
                // The current direction d and the line we are tracing are supposed to be roughly parallel.
                // In case the 'go outward' step in trace_step lead us astray, we might end up with a line
                // that is almost perpendicular to d. Then the back-projection below can result in an
                // endless loop. Break if the angle between d and line is greater than 45 deg.
                if (Point::dot(Point::normalized(self.d), line.normal())).abs() > 0.7
                // thresh is approx. sin(45 deg)
                {
                    return Ok(false);
                }

                // re-evaluate line with all the points up to here before projecting
                if !line.evaluate_max_distance(Some(1.5), None) {
                    return Ok(false);
                }

                let mut np = line.project(self.p);
                // make sure we are making progress even when back-projecting:
                // consider a 90deg corner, rotated 45deg. we step away perpendicular from the line and get
                // back projected where we left off the line.
                // The 'while' instead of 'if' was introduced to fix the issue with #245. It turns out that
                // np can actually be behind the projection of the last line point and we need 2 steps in d
                // to prevent a dead lock. see #245.png
                let mut last_point = line.points().last().copied().ok_or_else(|| {
                    Error::InvalidState { message: "trace line lost its trailing point".to_owned() }
                })?;
                while Point::distance(
                    np,
                    line.project(last_point),
                ) < 1.0
                {
                    np += self.d;
                    last_point = line.points().last().copied().ok_or_else(|| {
                        Error::InvalidState { message: "trace line lost its trailing point".to_owned() }
                    })?;
                }
                self.p = Point::centered(np);
            } else {
                let step_length_in_main_dir = if line.points().is_empty() {
                    0.0
                } else {
                    Point::dot(
                        Point::main_direction(self.d),
                        self.p
                            - line.points().last().copied().ok_or_else(|| {
                                Error::InvalidState { message: "trace line lost its trailing point".to_owned() }
                            })?,
                    )
                };
                line.add(self.p)?;

                if step_length_in_main_dir > 1.0 {
                    gaps += 1;
                    if gaps >= 2 || line.points().len() > 5 {
                        if !line.evaluate_max_distance(Some(1.5), None) {
                            return Ok(false);
                        }
                        let first_point = line.points().first().copied().ok_or_else(|| {
                            Error::InvalidState { message: "trace line has no anchor point".to_owned() }
                        })?;
                        if !self.update_direction_from_origin(
                            self.p - line.project(self.p) + first_point,
                        ) {
                            return Ok(false);
                        }
                        // check if the first half of the top-line trace is complete.
                        // the minimum code size is 10x10 -> every code has at least 4 gaps
                        //TODO: maybe switch to termination condition based on bottom line length to get a better
                        // finish_line for the right line trace
                        if !finish_line.is_valid() && gaps == 4 {
                            // undo the last insert, it will be inserted again after the restart
                            line.pop_back();
                            return Ok(true);
                        }
                    }
                } else if gaps == 0 && line.points().len() >= (2 * max_step_size) as usize {
                    return Ok(false);
                } // no point in following a line that has no gaps
            }

            if finish_line.is_valid() {
                max_step_size =
                    std::cmp::min(max_step_size, (finish_line.signed_distance(self.p)) as i32);
            }

            let step_result = self.trace_step(d_edge, max_step_size, line.is_valid())?;

            if step_result != StepResult::Found
            // we are successful iff we found an open end across a valid finish_line
            {
                return Ok(step_result == StepResult::OpenEnd
                    && finish_line.is_valid()
                    && (finish_line.signed_distance(self.p)) as i32 <= max_step_size + 1);
            }
        }
    }

    pub fn trace_corner(&mut self, dir: &mut Point, corner: &mut Point) -> Result<bool> {
        if !self.step(None) {
            return Ok(false);
        }
        *corner = self.p;
        std::mem::swap(&mut self.d, dir);
        self.trace_step(-1.0 * (*dir), 2, false)?;
        Ok(self.is_in(*corner) && self.is_in(self.p))
    }
}
