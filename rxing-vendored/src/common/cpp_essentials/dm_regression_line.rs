use crate::{Error, Point};
use anyhow::Result;

use super::RegressionLineTrait;

#[derive(Clone)]
pub struct DMRegressionLine {
    points: Vec<Point>,
    direction_inward: Point,
    pub(super) a: f32,
    pub(super) b: f32,
    pub(super) c: f32,
}

impl Default for DMRegressionLine {
    fn default() -> Self {
        Self {
            points: Default::default(),
            direction_inward: Default::default(),
            a: f32::NAN,
            b: f32::NAN,
            c: f32::NAN,
        }
    }
}

impl RegressionLineTrait for DMRegressionLine {
    fn points(&self) -> &[Point] {
        &self.points
    }

    fn length(&self) -> u32 {
        match (self.points.first(), self.points.last()) {
            (Some(first), Some(last)) if self.points.len() >= 2 => {
                Point::distance(*first, *last) as u32
            }
            _ => 0,
        }
    }

    fn is_valid(&self) -> bool {
        !self.a.is_nan()
    }

    fn normal(&self) -> Point {
        if self.is_valid() {
            Point {
                x: self.a,
                y: self.b,
            }
        } else {
            self.direction_inward
        }
    }

    fn signed_distance(&self, p: Point) -> f32 {
        Point::dot(self.normal(), p) - self.c
    }

    fn distance_single(&self, p: Point) -> f32 {
        (self.signed_distance(p)).abs()
    }

    fn reset(&mut self) {
        self.points.clear();
        self.direction_inward = Point { x: 0.0, y: 0.0 };
        self.a = f32::NAN;
        self.b = f32::NAN;
        self.c = f32::NAN;
    }

    fn add(&mut self, p: Point) -> Result<()> {
        if self.direction_inward == Point::default() {
            return Err(Error::InvalidState {
                message: "required internal state is missing".to_owned(),
            }
            .into());
        }
        self.points.push(p);
        if self.points.len() == 1 {
            self.c = Point::dot(self.normal(), p);
        }
        Ok(())
    }

    fn pop_back(&mut self) {
        self.points.pop();
    }

    fn set_direction_inward(&mut self, d: Point) {
        self.direction_inward = Point::normalized(d);
    }

    fn evaluate_max_distance(
        &mut self,
        max_signed_dist: Option<f64>,
        update_points: Option<bool>,
    ) -> bool {
        let max_signed_dist = max_signed_dist.unwrap_or(-1.0);
        let update_points = update_points.unwrap_or_default();

        let mut ret = self.evaluate_self();
        if max_signed_dist > 0.0 {
            let mut points = self.points.clone();
            loop {
                let old_points_size = points.len();
                // remove points that are further 'inside' than maxSignedDist or further 'outside' than 2 x maxSignedDist
                points.retain(|&p| {
                    let sd = self.signed_distance(p) as f64;
                    !(sd > max_signed_dist || sd < -2.0 * max_signed_dist)
                });
                if old_points_size == points.len() {
                    break;
                }
                ret = self.evaluate(&points);
            }

            if update_points {
                self.points = points;
            }
        }
        ret
    }

    fn is_high_res(&self) -> bool {
        let Some(mut min) = self.points.first().copied() else {
            return false;
        };
        let Some(mut max) = self.points.first().copied() else {
            return false;
        };
        for p in &self.points {
            min.x = f32::min(min.x, p.x);
            min.y = f32::min(min.y, p.y);
            max.x = f32::max(max.x, p.x);
            max.y = f32::max(max.y, p.y);
        }
        let diff = max - min;
        let len = diff.max_abs_component();
        let steps = f32::min(diff.x.abs(), diff.y.abs());
        // due to aliasing we get bad extrapolations if the line is short and too close to vertical/horizontal
        steps > 2.0 || len > 50.0
    }

    fn evaluate(&mut self, points: &[Point]) -> bool {
        if points.is_empty() {
            return false;
        }
        let mean = points.iter().sum::<Point>() / points.len() as f32;

        let mut sum_xx = 0.0;
        let mut sum_yy = 0.0;
        let mut sum_xy = 0.0;
        for p in points {
            let d = *p - mean;
            sum_xx += d.x * d.x;
            sum_yy += d.y * d.y;
            sum_xy += d.x * d.y;
        }
        if sum_yy >= sum_xx {
            let l = (sum_yy * sum_yy + sum_xy * sum_xy).sqrt();
            self.a = sum_yy / l;
            self.b = -sum_xy / l;
        } else {
            let l = (sum_xx * sum_xx + sum_xy * sum_xy).sqrt();
            self.a = sum_xy / l;
            self.b = -sum_xx / l;
        }
        if Point::dot(self.direction_inward, self.normal()) < 0.0 {
            self.a = -self.a;
            self.b = -self.b;
        }
        self.c = Point::dot(self.normal(), mean);
        // angle between original and new direction is at most 60 degree
        Point::dot(self.direction_inward, self.normal()) > 0.5
    }

    fn evaluate_self(&mut self) -> bool {
        let points = std::mem::take(&mut self.points);
        let result = self.evaluate(&points);
        self.points = points;
        result
    }

    fn a(&self) -> f32 {
        self.a
    }

    fn b(&self) -> f32 {
        self.b
    }

    fn c(&self) -> f32 {
        self.c
    }
}

impl DMRegressionLine {
    pub fn new(point_1: Point, point_2: Point) -> Self {
        let mut new = Self::default();
        RegressionLineTrait::evaluate(&mut new, &[point_1, point_2]);
        new
    }

    fn average<T>(c: &[f64], f: T) -> Option<f64>
    where
        T: Fn(f64) -> bool,
    {
        let mut sum: f64 = 0.0;
        let mut num = 0;
        for v in c {
            if f(*v) {
                sum += *v;
                num += 1;
            }
        }
        if num == 0 {
            None
        } else {
            Some(sum / num as f64)
        }
    }

    pub fn reverse(&mut self) {
        self.points.reverse();
    }

    pub fn modules(&mut self, beg: Point, end: Point) -> Result<f64> {
        if self.points.len() <= 3 {
            return Err(Error::InvalidState {
                message: "required internal state is missing".to_owned(),
            }
            .into());
        }

        // re-evaluate and filter out all points too far away. required for the gap_sizes calculation.
        self.evaluate_max_distance(Some(1.0), Some(true));

        let mut gap_sizes: Vec<f64> = Vec::new();
        let mut mod_sizes = Vec::new();

        gap_sizes.reserve(self.points.len());

        // calculate the distance between the points projected onto the regression line
        for i in 1..self.points.len() {
            gap_sizes.push(Point::distance(
                self.project(self.points[i]),
                self.project(self.points[i - 1]),
            ) as f64);
        }

        // calculate the (expected average) distance of two adjacent pixels
        let first = self.points[0];
        let last = self.points[self.points.len() - 1];
        let unit_pixel_dist = Point::length(Point::bresenham_direction(last - first)) as f64;

        // calculate the width of 2 modules (first black pixel to first black pixel)
        let mut sum_front: f64 =
            Point::distance(beg, self.project(self.points[0])) as f64 - unit_pixel_dist;
        let mut sum_back: f64 = 0.0; // (last black pixel to last black pixel)
        for dist in gap_sizes {
            if dist > 1.9 * unit_pixel_dist {
                mod_sizes.push(std::mem::take(&mut sum_back));
            }
            sum_front += dist;
            sum_back += dist;
            if dist > 1.9 * unit_pixel_dist {
                mod_sizes.push(std::mem::take(&mut sum_front));
            }
        }

        mod_sizes.push(sum_front + Point::distance(end, self.project(last)) as f64);
        mod_sizes[0] = 0.0; // the first element is an invalid sum_back value, would be pop_front() if vector supported this
        let line_length = Point::distance(beg, end) as f64 - unit_pixel_dist;
        let mut mean_mod_size =
            Self::average(&mod_sizes, |_: f64| true).ok_or(Error::InvalidState {
                message: "required internal state is missing".to_owned(),
            })?;
        for i in 0..2 {
            if let Some(next) = Self::average(&mod_sizes, |dist: f64| {
                (dist - mean_mod_size).abs() < mean_mod_size / (2 + i) as f64
            }) {
                mean_mod_size = next;
            }
        }
        Ok(line_length / mean_mod_size)
    }
}
