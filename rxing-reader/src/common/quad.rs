use crate::{Error, Point, point};

#[derive(Clone, Copy, Debug)]
pub struct Quadrilateral(pub [Point; 4]);

impl Quadrilateral {
    #[allow(dead_code)]
    pub const fn new(tl: Point, tr: Point, br: Point, bl: Point) -> Self {
        Self([tl, tr, br, bl])
    }

    pub const fn with_points(tl: Point, tr: Point, br: Point, bl: Point) -> Self {
        Self([tl, tr, br, bl])
    }

    pub const fn top_left(&self) -> &Point {
        &self.0[0]
    }
    pub const fn top_right(&self) -> &Point {
        &self.0[1]
    }
    pub const fn bottom_right(&self) -> &Point {
        &self.0[2]
    }
    pub const fn bottom_left(&self) -> &Point {
        &self.0[3]
    }

    #[allow(dead_code)]
    pub fn orientation(&self) -> f64 {
        let center_line =
            (*self.top_right() + *self.bottom_right()) - (*self.top_left() + *self.bottom_left());
        if center_line == Point::default() {
            return 0.0;
        }
        let center_line_f = Point::normalized(center_line);
        f32::atan2(center_line_f.y, center_line_f.x).into()
    }
    pub const fn points(&self) -> &[Point] {
        &self.0
    }
}

impl Quadrilateral {
    #[allow(dead_code)]
    pub fn rectangle(width: i32, height: i32, margin: Option<f32>) -> Quadrilateral {
        let margin = margin.unwrap_or(0.0);

        Quadrilateral([
            Point {
                x: margin,
                y: margin,
            },
            Point {
                x: width as f32 - margin,
                y: margin,
            },
            Point {
                x: width as f32 - margin,
                y: height as f32 - margin,
            },
            Point {
                x: margin,
                y: height as f32 - margin,
            },
        ])
    }

    pub fn rectangle_from_xy(x0: f32, x1: f32, y0: f32, y1: f32, o: Option<f32>) -> Self {
        let o = o.unwrap_or(0.5);
        Quadrilateral::from([
            point(x0 + o, y0 + o),
            point(x1 + o, y0 + o),
            point(x1 + o, y1 + o),
            point(x0 + o, y1 + o),
        ])
    }

    #[allow(dead_code)]
    pub fn centered_square(size: i32) -> Quadrilateral {
        Self::scale(
            &Quadrilateral([
                Point { x: -1.0, y: -1.0 },
                Point { x: 1.0, y: -1.0 },
                Point { x: 1.0, y: 1.0 },
                Point { x: -1.0, y: 1.0 },
            ]),
            size / 2,
        )
    }

    #[allow(dead_code)]
    pub fn line(y: i32, x_start: i32, x_stop: i32) -> Quadrilateral {
        Quadrilateral([
            Point {
                x: x_start as f32,
                y: y as f32,
            },
            Point {
                x: x_stop as f32,
                y: y as f32,
            },
            Point {
                x: x_stop as f32,
                y: y as f32,
            },
            Point {
                x: x_start as f32,
                y: y as f32,
            },
        ])
    }

    #[allow(dead_code)]
    pub fn is_convex(&self) -> bool {
        let n = self.0.len();
        let mut sign = false;

        let mut m = f32::INFINITY;
        let mut max_cross = 0.0_f32;

        for i in 0..n {
            let d1 = self.0[(i + 2) % n] - self.0[(i + 1) % n];
            let d2 = self.0[i] - self.0[(i + 1) % n];
            let cp = d1.cross(d2);

            m = f32::min(m, cp.abs());
            max_cross = f32::max(max_cross, cp.abs());

            if i == 0 {
                sign = cp > 0.0;
            } else if sign != (cp > 0.0) {
                return false;
            }
        }

        // It turns out being convex is not enough to prevent a "numerical instability"
        // that can cause the corners being projected inside the image boundaries but
        // some points near the corners being projected outside. This has been observed
        // where one corner is almost in line with two others. The M/m ratio is below 2
        // for the complete existing sample set. For very "skewed" QRCodes a value of
        // around 3 is realistic. A value of 14 has been observed to trigger the
        // instability.
        if !m.is_finite() || m <= f32::EPSILON {
            return false;
        }

        max_cross / m < 4.0
    }

    #[allow(dead_code)]
    pub fn scale(&self, factor: i32) -> Quadrilateral {
        Quadrilateral([
            self.0[0] * factor as f32,
            self.0[1] * factor as f32,
            self.0[2] * factor as f32,
            self.0[3] * factor as f32,
        ])
    }

    #[allow(dead_code)]
    pub fn center(&self) -> Point {
        let reduced: Point = self.0.iter().sum();
        let size = self.0.len() as f32;
        reduced / size
    }

    #[allow(dead_code)]
    pub fn rotated_corners(&self, n: Option<i32>, mirror: Option<bool>) -> Quadrilateral {
        let n = n.unwrap_or(1);

        let mirror = mirror.unwrap_or_default();

        let mut res = *self;
        res.0.rotate_left(((n + 4) % 4) as usize);
        if mirror {
            res.0.swap(1, 3);
        }
        res
    }

    #[allow(dead_code)]
    pub fn is_inside(&self, p: Point) -> bool {
        // Test if p is on the same side (right or left) of all polygon segments
        let mut pos = 0;
        let mut neg = 0;
        for i in 0..self.0.len() {
            if Point::cross(p - self.0[i], self.0[(i + 1) % self.0.len()] - self.0[i]) < 0.0 {
                neg += 1;
            } else {
                pos += 1;
            }
        }

        pos == 0 || neg == 0
    }

    #[allow(dead_code)]
    pub const fn have_intersecting_bounding_boxes(&self, b: &Quadrilateral) -> bool {
        // TODO: this is only a quick and dirty approximation that works for the trivial standard cases
        let x = b.top_right().x < self.top_left().x || b.top_left().x > self.top_right().x;
        let y = b.bottom_left().y < self.top_left().y || b.top_left().y > self.bottom_left().y;

        !(x || y)
    }

    pub fn blend(a: &Quadrilateral, b: &Quadrilateral) -> Self {
        let c = a[0];
        let dist2_first = |a, b| Point::distance(a, c) < Point::distance(b, c);
        // rotate points such that the the two top_left points are closest to each other
        let min_element =
            b.0.iter()
                .copied()
                .min_by(|a, b| match dist2_first(*a, *b) {
                    true => std::cmp::Ordering::Less,
                    false => std::cmp::Ordering::Greater,
                })
                .unwrap_or_default();
        let offset =
            b.0.iter()
                .position(|v| *v == min_element)
                .unwrap_or_default();

        let mut res = Quadrilateral::default();
        for i in 0..4 {
            res[i] = (a[i] + b[(i + offset) % 4]) / 2.0;
        }

        res
    }
}

impl Default for Quadrilateral {
    fn default() -> Self {
        Self([Point { x: 0.0, y: 0.0 }; 4])
    }
}

impl std::ops::Index<usize> for Quadrilateral {
    type Output = Point;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Quadrilateral {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl From<[Point; 4]> for Quadrilateral {
    fn from(value: [Point; 4]) -> Self {
        Self(value)
    }
}

impl TryFrom<&Vec<Point>> for Quadrilateral {
    type Error = anyhow::Error;

    fn try_from(value: &Vec<Point>) -> Result<Self, Self::Error> {
        if value.len() == 4 {
            Ok(Self([value[0], value[1], value[2], value[3]]))
        } else {
            Err(Error::InvalidArgument {
                message: format!(
                    "quadrilateral requires exactly 4 points, got {}",
                    value.len()
                ),
            }
            .into())
        }
    }
}
