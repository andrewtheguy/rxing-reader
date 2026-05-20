use crate::{
    Point,
    common::{
        BitMatrix, Quadrilateral,
        cpp_essentials::{
            Direction, FixedPattern, PatternRow, PatternType, PatternView, is_pattern,
        },
    },
    point,
};

use super::{
    BitMatrixCursorTrait, EdgeTracer, FastEdgeToEdgeCounter, Pattern, RegressionLine,
    RegressionLineTrait, update_min_max, update_min_max_float,
};

pub fn read_symmetric_pattern<const N: usize, Cursor: BitMatrixCursorTrait>(
    cur: &mut Cursor,
    range: i32,
) -> Option<Pattern<N>> {
    if N % 2 != 1 || range <= 0 {
        return None;
    }

    let mut range = range;

    let mut res: Pattern<N> = [0; N];
    let s_2 = res.len() as isize / 2;
    let mut cuo = cur.turned_back();

    let mut next = |cur: &mut Cursor, i: isize| {
        let v = cur.step_to_edge(Some(1), Some(range), None);
        res[(s_2 + i) as usize] = (res[(s_2 + i) as usize] as i32 + v) as u16;
        if range != 0 {
            range -= v;
        }

        v
    };

    for i in 0..=s_2 {
        if next(cur, i) == 0 || next(&mut cuo, -i) == 0 {
            return None;
        }
    }
    res[s_2 as usize] = res[s_2 as usize].saturating_sub(1); // the starting pixel has been counted twice, fix this

    Some(res)
}

// default for RELAXED_THRESHOLD should be false
pub fn check_symmetric_pattern<
    const E2E: bool,
    const LEN: usize,
    const SUM: usize,
    T: BitMatrixCursorTrait,
>(
    cur: &mut T,
    pattern: &Pattern<LEN>,
    range: i32,
    update_position: bool,
) -> i32 {
    let mut range = range;

    let Ok(mut cur_fwd) = FastEdgeToEdgeCounter::new(cur) else {
        return 0;
    };
    let binding = cur.turned_back();
    let Ok(mut cur_bwd) = FastEdgeToEdgeCounter::new(&binding) else {
        return 0;
    };

    let center_fwd = cur_fwd.step_to_next_edge(range as u32) as i32;
    if center_fwd == 0 {
        return 0;
    }
    let center_bwd = cur_bwd.step_to_next_edge(range as u32) as i32;
    if center_bwd == 0 {
        return 0;
    }

    if range <= 0 {
        return 0;
    }
    let mut res: PatternRow = PatternRow::new(vec![0; LEN]);
    let s_2 = (res.len()) / 2;
    res[s_2] = (center_fwd + center_bwd - 1) as u16; // -1 because the starting pixel is counted twice
    range -= res[s_2] as i32;

    let mut next = |cur: &mut FastEdgeToEdgeCounter, i: isize| {
        let v = cur.step_to_next_edge(range as u32) as i32;
        res[(s_2 as isize + i) as usize] = v as u16;
        range -= v;

        v
    };

    for i in 1..=s_2 {
        if next(&mut cur_fwd, i as isize) == 0 || next(&mut cur_bwd, -(i as isize)) == 0 {
            return 0;
        }
    }

    if is_pattern::<E2E, LEN, SUM, false>(
        &PatternView::new(&res),
        &FixedPattern::<LEN, SUM, false>::with_reference(pattern),
        None,
        0.0,
        0.0,
    ) == 0.0
    {
        return 0;
    }

    if update_position {
        cur.step(Some((res[s_2] as i32 / 2 - (center_bwd - 1)) as f32));
    }

    res.into_iter().sum::<PatternType>() as i32
}

pub fn average_edge_pixels<T: BitMatrixCursorTrait>(
    cur: &mut T,
    range: i32,
    num_of_edges: u32,
) -> Option<Point> {
    let mut sum = Point::default();

    for _i in 0..num_of_edges {
        if !cur.is_in_self() {
            return None;
        }
        cur.step_to_edge(Some(1), Some(range), None);
        sum += cur.p().centered() + (cur.p() + cur.back()).centered();
    }
    Some(sum / (2 * num_of_edges) as f32)
}

pub fn center_of_double_cross(
    image: &BitMatrix,
    center: Point,
    range: i32,
    num_of_edges: u32,
) -> Option<Point> {
    let mut sum = Point::default();

    for d in [
        point(0.0, 1.0),
        point(1.0, 0.0),
        point(1.0, 1.0),
        point(1.0, -1.0),
    ] {
        let avr1 =
            average_edge_pixels(&mut EdgeTracer::new(image, center, d), range, num_of_edges)?;
        let avr2 =
            average_edge_pixels(&mut EdgeTracer::new(image, center, -d), range, num_of_edges)?;

        sum += avr1 + avr2;
    }
    Some(sum / 8.0)
}

pub fn center_of_ring(
    image: &BitMatrix,
    center: Point,
    range: i32,
    nth: i32,
    require_circle: bool,
) -> Option<Point> {
    // range is the approximate width/height of the nth ring, if nth>1 then it would be plausible to limit the search radius
    // to approximately range / 2 * sqrt(2) == range * 0.75 but it turned out to be too limiting with realworld/noisy data.
    let radius = range;
    let inner = nth < 0;
    let nth = nth.abs();
    let mut cur = EdgeTracer::new(image, center, point(0.0, 1.0));
    if cur.step_to_edge(Some(nth), Some(radius), Some(inner)) == 0 {
        return None;
    }
    cur.turn_right(); // move clock wise and keep edge on the right/left depending on backup
    let edge_dir = if inner {
        Direction::Left
    } else {
        Direction::Right
    };

    let mut neighbour_mask = 0;
    let start = cur.p();
    let mut sum = Point::default();
    let mut n = 0;
    loop {
        sum += cur.p().centered();
        n += 1;

        // find out if we come full circle around the center. 8 bits have to be set in the end.
        neighbour_mask |= 1
            << (4.0
                + Point::dot(
                    Point::floor(Point::bresenham_direction(cur.p() - center)),
                    point(1.0, 3.0),
                )) as u32;

        if !cur.step_along_edge(edge_dir, None) {
            return None;
        }

        // use L-inf norm, simply because it is a lot faster than L2-norm and sufficiently accurate
        if Point::max_abs_component(cur.p - center) > radius as f32
            || center == cur.p
            || n > 4 * 2 * range
        {
            return None;
        }

        if !(cur.p != start) {
            break;
        }
    }

    if require_circle && neighbour_mask != 0b111101111 {
        return None;
    }

    Some(sum / n as f32)
}

pub fn center_of_rings(
    image: &BitMatrix,
    center: Point,
    range: i32,
    num_of_rings: u32,
) -> Option<Point> {
    let mut n = 1;
    let mut sum = center;
    for i in 2..(num_of_rings + 1) {
        let c = center_of_ring(image, center.floor(), range, i as i32, true)?;

        if c == Point::default() {
            if n == 1 {
                return None;
            } else {
                return Some(sum / n as f32);
            }
        } else if Point::distance(c, center) > range as f32 / num_of_rings as f32 / 2.0 {
            return None;
        }

        sum += c;
        n += 1;
    }
    Some(sum / n as f32)
}

pub fn collect_ring_points(
    image: &BitMatrix,
    center: Point,
    range: i32,
    edge_index: i32,
    backup: bool,
) -> Vec<Point> {
    let center_i = center.floor();
    let radius = range;
    let mut cur = EdgeTracer::new(image, center_i, point(0.0, 1.0));
    if cur.step_to_edge(Some(edge_index), Some(radius), Some(backup)) == 0 {
        return Vec::default();
    }
    cur.turn_right(); // move clock wise and keep edge on the right/left depending on backup
    let edge_dir = if backup {
        Direction::Left
    } else {
        Direction::Right
    };

    let mut neighbour_mask = 0;
    let start = cur.p();
    let mut points = Vec::<Point>::with_capacity(4 * range as usize);

    loop {
        points.push(cur.p().centered());

        // find out if we come full circle around the center. 8 bits have to be set in the end.
        neighbour_mask |= 1
            << (4.0
                + Point::dot(
                    Point::round(Point::bresenham_direction(cur.p - center_i)),
                    point(1.0, 3.0),
                )) as u32;

        if !cur.step_along_edge(edge_dir, None) {
            return Vec::default();
        }

        // use L-inf norm, simply because it is a lot faster than L2-norm and sufficiently accurate
        if Point::max_abs_component(cur.p - center_i) > radius as f32
            || center_i == cur.p
            || (points).len() > 4 * 2 * range as usize
        {
            return Vec::default();
        }

        if !(cur.p != start) {
            break;
        }
    }

    if neighbour_mask != 0b111101111 {
        return Vec::default();
    }

    points
}

pub fn fit_quadrilateral_to_points(center: Point, points: &mut [Point]) -> Option<Quadrilateral> {
    // rotate points such that the first one is the furthest away from the center (hence, a corner)
    let max_by_pred = |a: &&Point, b: &&Point| {
        let da = Point::distance(**a, center);
        let db = Point::distance(**b, center);
        da.total_cmp(&db)
    };

    let max = points.iter().max_by(max_by_pred)?;

    let pos = points.iter().position(|e| e == max)?;

    points.rotate_left(pos);

    let mut corners = [Point::default(); 4];
    corners[0] = points[0];
    // find the oposite corner by looking for the farthest point near the oposite point
    corners[2] = *points[(points.len() * 3 / 8)..=(points.len() * 5 / 8)]
        .iter()
        .max_by(max_by_pred)?;
    // find the two in between corners by looking for the points farthest from the long diagonal
    let l = RegressionLine::with_two_points(corners[0], corners[2]);

    let diagonal_max_by_pred = |p1: &Point, p2: &Point| {
        let d1 = l.distance_single(*p1);
        let d2 = l.distance_single(*p2);
        d1.total_cmp(&d2)
    };
    corners[1] = points[(points.len() / 8)..=(points.len() * 3 / 8)]
        .iter()
        .copied()
        .max_by(diagonal_max_by_pred)?;
    corners[3] = points[(points.len() * 5 / 8)..=(points.len() * 7 / 8)]
        .iter()
        .copied()
        .max_by(diagonal_max_by_pred)?;

    let corner_positions = [
        0,
        points.iter().position(|p| *p == corners[1])?,
        points.iter().position(|p| *p == corners[2])?,
        points.iter().position(|p| *p == corners[3])?,
    ];

    let try_get_range = |a: usize, b: usize| -> Option<&[Point]> {
        if a > b {
            None
        }
        // Added for Issue #36 where array is sometimes out of bounds
        else if a + 1 >= points.len() || b >= points.len() {
            if a + 1 >= points.len() {
                None
            } else {
                Some(&points[a..])
            }
        }
        // Added for Issue #36 where a sometimes equals b
        else if a == b {
            Some(&points[a..b])
        } else {
            Some(&points[a + 1..b])
        }
    };

    let lines = [
        RegressionLine::with_point_slice(try_get_range(corner_positions[0], corner_positions[1])?),
        RegressionLine::with_point_slice(try_get_range(corner_positions[1], corner_positions[2])?),
        RegressionLine::with_point_slice(try_get_range(corner_positions[2], corner_positions[3])?),
        RegressionLine::with_point_slice(try_get_range(corner_positions[3], points.len())?),
    ];

    if lines.iter().any(|line| !line.is_valid()) {
        return None;
    }

    let beg: [usize; 4] = [
        corner_positions[0] + 1,
        corner_positions[1] + 1,
        corner_positions[2] + 1,
        corner_positions[3] + 1,
    ];
    let end: [usize; 4] = [
        corner_positions[1],
        corner_positions[2],
        corner_positions[3],
        points.len(),
    ];

    // check if all points belonging to each line segment are sufficiently close to that line
    for i in 0..4 {
        for p in &points[beg[i]..end[i]] {
            let len = (end[i] - beg[i]) as f64;
            if len > 3.0 && (lines[i].distance_single(*p) as f64) > (len / 8.0).clamp(1.0, 8.0) {
                return None;
            }
        }
    }

    let mut res = Quadrilateral::default();
    for i in 0..4 {
        res[i] = RegressionLine::intersect(&lines[i], &lines[(i + 1) % 4])?;
    }

    Some(res)
}

pub fn quadrilateral_is_plausible_square(q: &Quadrilateral, line_index: usize) -> bool {
    let mut min_side_length;

    min_side_length = Point::distance(q[0], q[3]) as f64;
    let mut max_side_length = min_side_length;

    for i in 1..4 {
        update_min_max_float(
            &mut min_side_length,
            &mut max_side_length,
            Point::distance(q[i - 1], q[i]) as f64,
        );
    }

    min_side_length >= (line_index * 2) as f64 && min_side_length > max_side_length / 3.0
}

pub fn fit_square_to_points(
    image: &BitMatrix,
    center: Point,
    range: i32,
    line_index: i32,
    backup: bool,
) -> Option<Quadrilateral> {
    let mut points = collect_ring_points(image, center, range, line_index, backup);
    if points.is_empty() {
        return None;
    }

    let res = fit_quadrilateral_to_points(center, &mut points)?;
    if !quadrilateral_is_plausible_square(&res, (line_index - i32::from(backup)) as usize) {
        return None;
    }

    Some(res)
}

pub fn find_concentric_pattern_corners(
    image: &BitMatrix,
    center: Point,
    range: i32,
    line_index: i32,
) -> Option<Quadrilateral> {
    let inner_corners = fit_square_to_points(image, center, range, line_index, false)?;

    let outer_corners = fit_square_to_points(image, center, range, line_index + 1, true)?;

    let res = Quadrilateral::blend(&inner_corners, &outer_corners);

    Some(res)
}

#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub struct ConcentricPattern {
    pub p: Point,
    pub size: i32,
}

impl std::ops::Sub for ConcentricPattern {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let new_p = self.p - rhs.p;
        Self {
            p: new_p,
            size: self.size,
        }
    }
}

impl std::ops::Add for ConcentricPattern {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let new_p = self.p + rhs.p;
        Self {
            p: new_p,
            size: self.size,
        }
    }
}

impl From<Point> for ConcentricPattern {
    fn from(value: Point) -> Self {
        Self { p: value, size: 0 }
    }
}

impl ConcentricPattern {
    pub fn dot(self, other: ConcentricPattern) -> f32 {
        Point::dot(self.p, other.p)
    }

    pub fn cross(self, other: ConcentricPattern) -> f32 {
        Point::cross(self.p, other.p)
    }

    pub fn distance(self, other: ConcentricPattern) -> f32 {
        Point::distance(self.p, other.p)
    }
}

pub fn locate_concentric_pattern<const E2E: bool, const LEN: usize, const SUM: usize>(
    image: &BitMatrix,
    pattern: &Pattern<LEN>,
    center: Point,
    range: i32,
) -> Option<ConcentricPattern> {
    let mut cur = EdgeTracer::new(image, center.floor(), Point::default());
    let mut min_spread = image.get_width() as i32;
    let mut max_spread = 0_i32;

    // TODO: setting max_error to 1 can subtantially help with detecting symbols with low print quality resulting in damaged
    // finder patterns, but it sutantially increases the runtime (approx. 20% slower for the falsepositive images).
    let mut max_error = 0;
    for d in [point(0.0, 1.0), point(1.0, 0.0)] {
        cur.set_direction(d); // THIS COULD POSSIBLY BE WRONG, WE MIGHT MEAN TO CLONE cur EACH RUN?

        let spread = check_symmetric_pattern::<E2E, LEN, SUM, _>(&mut cur, pattern, range, true);
        if spread != 0 {
            update_min_max(&mut min_spread, &mut max_spread, spread);
        } else {
            max_error -= 1;
            if max_error < 0 {
                return None;
            }
        }
    }

    for d in [point(1.0, 1.0), point(1.0, -1.0)] {
        cur.set_direction(d); // THIS COULD POSSIBLY BE WRONG, WE MIGHT MEAN TO CLONE cur EACH RUN?
        let spread =
            check_symmetric_pattern::<E2E, LEN, SUM, _>(&mut cur, pattern, range * 2, false);
        if spread != 0 {
            update_min_max(&mut min_spread, &mut max_spread, spread);
        } else {
            max_error -= 1;
            if max_error < 0 {
                return None;
            }
        }
    }

    if max_spread > 5 * min_spread {
        return None;
    }

    let new_center =
        finetune_concentric_pattern_center(image, cur.p(), range, pattern.len() as u32)?;

    Some(ConcentricPattern {
        p: new_center,
        size: (max_spread + min_spread) / 2,
    })
}

pub fn finetune_concentric_pattern_center(
    image: &BitMatrix,
    center: Point,
    range: i32,
    finder_pattern_size: u32,
) -> Option<Point> {
    // make sure we have at least one path of white around the center
    if let Some(res1) = center_of_ring(image, center.floor(), range, 1, true) {
        if !image.get_point(res1) {
            return None;
        }
        // and then either at least one more ring around that
        if let Some(res2) = center_of_rings(image, res1, range, finder_pattern_size / 2) {
            return if image.get_point(res2) {
                Some(res2)
            } else {
                None
            };
        }
        // or the center can be approximated by a square
        if fit_square_to_points(image, res1, range, 1, false).is_some() {
            return Some(res1);
        }
        // TODO: this is currently only keeping #258 alive, evaluate if still worth it
        if let Some(res2) =
            center_of_double_cross(image, res1.floor(), range, finder_pattern_size / 2 + 1)
        {
            return if image.get_point(res2) {
                Some(res2)
            } else {
                None
            };
        }
    }
    None
}
