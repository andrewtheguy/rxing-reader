use anyhow::Result;
use multimap::MultiMap;

use crate::{
    Error,
    common::{
        DefaultGridSampler, GridSampler, SamplerControl,
        detect::{
            Matrix, append_bit, center_of_ring, find_concentric_pattern_corners,
            find_left_guard_by,
        },
    },
    point, point_i,
    qrcode::{Version, VersionRef},
};

use crate::{
    Point,
    common::{
        BitMatrix, PerspectiveTransform, Quadrilateral,
        detect::{
            BitMatrixCursorTrait, ConcentricPattern, Direction, EdgeTracer, FixedPattern,
            PatternRow, PatternType, PatternView, RegressionLine, RegressionLineTrait,
            intersect, is_pattern, locate_concentric_pattern, read_pattern_row,
            read_symmetric_pattern,
        },
    },
};

pub(super) struct DetectorResult {
    bits: BitMatrix,
}

impl DetectorResult {
    fn new(bits: BitMatrix) -> Self {
        Self { bits }
    }

    pub(super) fn bits(&self) -> &BitMatrix {
        &self.bits
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct FinderPatternSet {
    pub bl: ConcentricPattern,
    pub tl: ConcentricPattern,
    pub tr: ConcentricPattern,
}

pub type FinderPatterns = Vec<ConcentricPattern>;
pub type FinderPatternSets = Vec<FinderPatternSet>;

const LEN: usize = 5;
const SUM: usize = 7;
const PATTERN: FixedPattern<LEN, SUM, false> = FixedPattern::new([1, 1, 3, 1, 1]);
const E2E: bool = true;

/// Search-window multiplier applied to the finder-pattern row sum when locating
/// the concentric pattern. The window is widened to tolerate strongly skewed
/// samples where the on-row run lengths underestimate the true pattern extent.
const SKEW_TOLERANCE_MULTIPLIER: i32 = 3;

/// Step size (in module widths) for the 3x3 neighbour search around the initial
/// alignment-pattern estimate in [`locate_alignment_pattern`].
const ALIGNMENT_SEARCH_RADIUS_MULTIPLIER: f32 = 2.25;

fn find_pattern(view: PatternView<'_>) -> Result<PatternView<'_>> {
    find_left_guard_by::<LEN, _>(
        view,
        LEN,
        |view: &PatternView, space_in_pixel: Option<f32>| {
            // perform a fast plausibility test for 1:1:3:1:1 pattern
            if view[2] < 2u16 * std::cmp::max(view[0], view[4])
                || view[2] < std::cmp::max(view[1], view[3])
            {
                return false;
            }
            is_pattern::<E2E, 5, 7, false>(view, &PATTERN, space_in_pixel, 0.1, 0.0) != 0.0
        },
    )
}

/// Locate the finder patterns for the symbol.
pub fn find_finder_patterns(image: &BitMatrix, try_harder: bool) -> FinderPatterns {
    const MIN_SKIP: usize = 3; // 1 pixel/module times 3 modules/center
    const MAX_MODULES_FAST: usize = 20 * 4 + 17; // support up to version 20 for mobile clients

    // Let's assume that the maximum version QR Code we support takes up 1/4 the height of the
    // image, and then account for the center being 3 modules in size. This gives the smallest
    // number of pixels the center could be, so skip this often. When trying harder, look for all
    // QR versions regardless of how dense they are.
    let height = image.height();
    let mut skip = (3 * height) / (4 * MAX_MODULES_FAST);
    if skip < MIN_SKIP || try_harder {
        skip = MIN_SKIP;
    }

    let mut res: Vec<ConcentricPattern> = Vec::new();
    let mut y = skip - 1;

    while y < height {
        let mut row = PatternRow::default();
        read_pattern_row(image, y, &mut row, false);
        let mut next: PatternView = PatternView::new(&row);

        while {
            if let Ok(up_next) = find_pattern(next) {
                next = up_next;
                next.is_valid()
            } else {
                false
            }
        } {
            let p = point(
                next.pixels_in_front() as f32
                    + next[0] as f32
                    + next[1] as f32
                    + next[2] as f32 / 2.0,
                y as f32 + 0.5,
            );

            // make sure p is not 'inside' an already found pattern area
            if !res
                .iter()
                .any(|old| Point::distance(p, old.p) < (old.size as f32) / 2.0)
            {
                let pattern = locate_concentric_pattern::<E2E, 5, 7>(
                    image,
                    &PATTERN.into(),
                    p,
                    next.iter().sum::<u16>() as i32 * SKEW_TOLERANCE_MULTIPLIER,
                );
                if let Some(p) = pattern {
                    res.push(p);
                }
            }

            next.skip_pair();
            next.skip_pair();
            next.extend();
        }

        y += skip;
    }

    res
}

/// - `patterns`: list of ConcentricPattern objects, i.e. found finder pattern squares
///
/// Returns list of plausible finder pattern sets, sorted by decreasing plausibility.
pub fn generate_finder_pattern_sets(patterns: &mut FinderPatterns) -> FinderPatternSets {
    patterns.sort_by_key(|p| p.size);

    let mut sets: MultiMap<u64, FinderPatternSet> = MultiMap::new();
    let squared_distance = |a: ConcentricPattern, b: ConcentricPattern| {
        // The scaling of the distance by the b/a size ratio is a very coarse compensation for the shortening effect of
        // the camera projection on slanted symbols. The fact that the size of the finder pattern is proportional to the
        // distance from the camera is used here. This approximation only works if a < b < 2*a (see below).
        // Test image: fix-finderpattern-order.jpg
        ConcentricPattern::dot(a - b, a - b) as f64
            * (((b).size as f64) / ((a).size as f64)).powi(2)
    };

    let cos_upper: f64 = (45.0_f64 / 180.0 * std::f64::consts::PI).cos();
    let cos_lower: f64 = (135.0_f64 / 180.0 * std::f64::consts::PI).cos();

    let nb_patterns = (patterns).len();

    if nb_patterns < 2 {
        return FinderPatternSets::default();
    }

    for i in 0..(nb_patterns - 2) {
        for j in (i + 1)..(nb_patterns - 1) {
            for k in (j + 1)..nb_patterns {
                let mut a = &patterns[i];
                let mut b = &patterns[j];
                let mut c = &patterns[k];
                // if the pattern sizes are too different to be part of the same symbol, skip this
                // and the rest of the innermost loop (sorted list)
                if c.size > a.size * 2 {
                    break;
                }

                // Orders the three points in an order [A,B,C] such that AB is less than AC
                // and BC is less than AC, and the angle between BC and BA is less than 180 degrees.

                let mut dist_ab2 = squared_distance(*a, *b);
                let mut dist_bc2 = squared_distance(*b, *c);
                let mut dist_ac2 = squared_distance(*a, *c);

                if dist_bc2 >= dist_ab2 && dist_bc2 >= dist_ac2 {
                    (a, b) = (b, a);
                    (dist_bc2, dist_ac2) = (dist_ac2, dist_bc2);
                } else if dist_ab2 >= dist_ac2 && dist_ab2 >= dist_bc2 {
                    (b, c) = (c, b);
                    (dist_ab2, dist_ac2) = (dist_ac2, dist_ab2);
                }

                let dist_ab = dist_ab2.sqrt();
                let dist_bc = (dist_bc2).sqrt();

                // Make sure dist_ab and dist_bc don't differ more than reasonable
                // TODO: make sure the constant 2 is not to conservative for reasonably tilted symbols
                if dist_ab > 2.0 * dist_bc || dist_bc > 2.0 * dist_ab {
                    continue;
                }

                // Estimate the module count and ignore this set if it can not result in a valid decoding
                let module_count = (dist_ab + dist_bc)
                    / (2.0 * (a.size + b.size + c.size) as f64 / (3.0 * 7.0))
                    + 7.0;
                if !(21.0 * 0.9..=177.0 * 1.5).contains(&module_count)
                // module_count may be overestimated, see above
                {
                    continue;
                }

                // Make sure the angle between AB and BC does not deviate from 90° by more than 45°
                let cos_ab_bc = (dist_ab2 + dist_bc2 - dist_ac2) / (2.0 * dist_ab * dist_bc);
                if (cos_ab_bc.is_nan()) || cos_ab_bc > cos_upper || cos_ab_bc < cos_lower {
                    continue;
                }

                // a^2 + b^2 = c^2 (Pythagorean theorem), and a = b (isosceles triangle).
                // Since any right triangle satisfies the formula c^2 - b^2 - a^2 = 0,
                // we need to check both two equal sides separately.
                // The value of |c^2 - 2 * b^2| + |c^2 - 2 * a^2| increases as dissimilarity
                // from isosceles right triangle.
                let d: f64 = (dist_ac2 - 2.0 * dist_ab2).abs() + (dist_ac2 - 2.0 * dist_bc2).abs();

                // Use cross product to figure out whether A and C are correct or flipped.
                // This asks whether BC x BA has a positive z component, which is the arrangement
                // we want for A, B, C. If it's negative then swap A and C.
                if ConcentricPattern::cross(*c - *b, *a - *b) < 0.0 {
                    std::mem::swap(&mut a, &mut c);
                }

                // arbitrarily limit the number of potential sets
                // (this has performance implications while limiting the maximal number of detected symbols)
                sets.insert(
                    d.to_bits(),
                    FinderPatternSet {
                        bl: *a,
                        tl: *b,
                        tr: *c,
                    },
                );
            }
        }
    }

    // convert from multimap to vector
    let mut res: FinderPatternSets = Vec::with_capacity(sets.len());

    for (_, v) in sets {
        res.extend(v);
    }

    res.sort_by_key(|i| i.bl.size);

    res
}

fn estimate_module_size(
    image: &BitMatrix,
    a: ConcentricPattern,
    b: ConcentricPattern,
) -> Result<f64> {
    let mut cur = EdgeTracer::new(image, a.p, b.p - a.p);
    if !cur.is_black() {
        return Err(Error::NotFound {
            message: "QR pattern was not detected".into(),
        }
        .into());
    }

    let pattern = read_symmetric_pattern::<5, _>(&mut cur, a.size * 2).ok_or(Error::NotFound {
        message: "QR pattern was not detected".into(),
    })?;

    if !(is_pattern::<E2E, 5, 7, false>(
        &PatternView::from_bars(&pattern),
        &PATTERN,
        None,
        0.0,
        0.0,
    ) != 0.0)
    {
        return Err(Error::NotFound {
            message: "QR pattern was not detected".into(),
        }
        .into());
    }

    Ok(
        (2 * pattern.iter().sum::<PatternType>() - pattern[0] - pattern[4]) as f64 / 12.0
            * cur.d().length() as f64,
    )
}

struct DimensionEstimate {
    dim: i32,
    ms: f64,
    err: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FinderPatternEdge {
    Outer,
    Inner,
}

impl FinderPatternEdge {
    const fn nth(self) -> i32 {
        match self {
            Self::Outer => 2,
            Self::Inner => 3,
        }
    }

    const fn should_backup(self) -> bool {
        matches!(self, Self::Inner)
    }
}

fn estimate_dimension(
    image: &BitMatrix,
    a: ConcentricPattern,
    b: ConcentricPattern,
) -> Result<DimensionEstimate> {
    let ms_a = estimate_module_size(image, a, b)?;
    let ms_b = estimate_module_size(image, b, a)?;

    let module_size = (ms_a + ms_b) / 2.0;

    let dimension = (ConcentricPattern::distance(a, b) as f64 / module_size).round() as i32 + 7;
    let error = 1 - (dimension % 4);

    Ok(DimensionEstimate {
        dim: dimension + error,
        ms: module_size,
        err: (error).abs(),
    })
}

fn trace_line(
    image: &BitMatrix,
    p: Point,
    d: Point,
    edge: FinderPatternEdge,
) -> Result<impl RegressionLineTrait> {
    let mut cur = EdgeTracer::new(image, p, d - p);
    let mut line = RegressionLine::default();
    line.set_direction_inward(cur.back());

    // collect points inside the black line -> backup on 3rd edge
    cur.step_to_edge(edge.nth(), 0, edge.should_backup());
    if edge.should_backup() {
        cur.turn_back();
    }

    let mut cur_i = EdgeTracer::new(image, cur.p, Point::main_direction(cur.d()));
    // make sure cur_i positioned such that the white->black edge is directly behind
    // Test image: fix-traceline.jpg
    while !cur_i.edge_at_back().is_black() {
        if cur_i.edge_at_left().into() {
            cur_i.turn_right();
        } else if cur_i.edge_at_right().into() {
            cur_i.turn_left();
        } else {
            cur_i.step_by(-1.0);
        }
    }

    for dir in [Direction::Left, Direction::Right] {
        let mut c = EdgeTracer::new(image, cur_i.p, cur_i.direction(dir));
        let mut step_count = (Point::max_abs_component(cur.p - p)) as i32;
        loop {
            line.add(Point::centered(c.p))?;

            step_count -= 1;
            if !(step_count > 0 && c.step_along_edge_with_corner_skip(dir, true)) {
                break;
            }
        }
    }

    line.evaluate_max_distance_with(1.0, true);

    Ok(line)
}

// estimate how tilted the symbol is (return value between 1 and 2, see also above)
fn estimate_tilt(fp: &FinderPatternSet) -> f64 {
    let min = [fp.bl.size, fp.tl.size, fp.tr.size]
        .iter()
        .min()
        .copied()
        .unwrap_or(i32::MAX);
    let max = [fp.bl.size, fp.tl.size, fp.tr.size]
        .iter()
        .max()
        .copied()
        .unwrap_or(i32::MIN);

    (max as f64) / (min as f64)
}

fn mod2_pix(
    dimension: i32,
    br_offset: Point,
    pix: Quadrilateral,
) -> Result<PerspectiveTransform> {
    let mut quad = Quadrilateral::rectangle(dimension, dimension, Some(3.5));
    quad[2] -= br_offset;

    PerspectiveTransform::quadrilateral_to_quadrilateral(quad, pix)
}

fn locate_alignment_pattern(
    image: &BitMatrix,
    module_size: i32,
    estimate: Point,
) -> Option<Point> {
    for d in [
        point(0.0, 0.0),
        point(0.0, -1.0),
        point(0.0, 1.0),
        point(-1.0, 0.0),
        point(1.0, 0.0),
        point(-1.0, -1.0),
        point(1.0, -1.0),
        point(1.0, 1.0),
        point(-1.0, 1.0),
    ] {
        let Some(cor) = center_of_ring(
            image,
            (estimate + module_size as f32 * ALIGNMENT_SEARCH_RADIUS_MULTIPLIER * d).floor(),
            module_size * 3,
            1,
            false,
        ) else {
            continue;
        };

        // if we did not land on a black pixel the concentric pattern finder will fail
        if !image.at_point(cor) {
            continue;
        }

        if let Some(cor1) = center_of_ring(image, cor.floor(), module_size, 1, true)
            && let Some(cor2) = center_of_ring(image, cor.floor(), module_size * 3, -2, true)
            && Point::distance(cor1, cor2) < module_size as f32 / 2.0
        {
            let res = (cor1 + cor2) / 2.0;
            return Some(res);
        }
    }

    None
}

pub fn read_version(
    image: &BitMatrix,
    dimension: usize,
    mod2_pix: PerspectiveTransform,
) -> Result<VersionRef> {
    let mut bits = [None, None];

    for mirror in [false, true] {
        // Read top-right/bottom-left version info: 3 wide by 6 tall (depending on mirrored)
        let mut version_bits: u32 = 0;
        let mut valid = true;
        'read_version_bits: for y in (0..=5).rev() {
            for x in ((dimension - 11)..=(dimension - 9)).rev() {
                let module = if mirror {
                    point(y as f32, x as f32)
                } else {
                    point(x as f32, y as f32)
                };
                let Some(pixel) = mod2_pix.transform_point(module.centered()) else {
                    valid = false;
                    break 'read_version_bits;
                };
                if !image.is_in(pixel) {
                    valid = false;
                    break 'read_version_bits;
                }
                append_bit(&mut version_bits, image.at_point(pixel));
            }
        }
        if valid {
            bits[usize::from(mirror)] = Some(version_bits);
        }
    }

    Version::decode_version_information_pair(bits)
}

fn module_dimension(dimension: i32) -> Result<usize> {
    usize::try_from(dimension).map_err(|_| {
        Error::NotFound {
            message: "QR pattern was not detected".into(),
        }
        .into()
    })
}

pub fn sample_qr(image: &BitMatrix, fp: &FinderPatternSet) -> Result<DetectorResult> {
    let top = estimate_dimension(image, fp.tl, fp.tr).ok();
    let left = estimate_dimension(image, fp.tl, fp.bl).ok();

    let best = match (top, left) {
        (Some(top), Some(left)) => match top.err.cmp(&left.err) {
            std::cmp::Ordering::Less => top,
            std::cmp::Ordering::Equal if top.dim > left.dim => top,
            std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => left,
        },
        (Some(top), None) => top,
        (None, Some(left)) => left,
        (None, None) => {
            return Err(Error::NotFound {
                message: "QR pattern was not detected".into(),
            }
            .into());
        }
    };

    let mut dimension = best.dim;
    let mut dimension_usize = module_dimension(dimension)?;
    let module_size = (best.ms + 1.0) as i32;

    let mut br = ConcentricPattern {
        p: point(-1.0, -1.0),
        size: 0,
    };
    let mut br_offset = point_i(3, 3);

    // Everything except version 1 (21 modules) has an alignment pattern. Estimate the center of that by intersecting
    // line extensions of the 1 module wide square around the finder patterns. This could also help with detecting
    // slanted symbols of version 1.

    // generate 4 lines: outer and inner edge of the 1 module wide black line between the two outer and the inner
    // (tl) finder pattern
    let bl_outer = trace_line(image, fp.bl.p, fp.tl.p, FinderPatternEdge::Outer)?;
    let bl_inner = trace_line(image, fp.bl.p, fp.tl.p, FinderPatternEdge::Inner)?;
    let tr_outer = trace_line(image, fp.tr.p, fp.tl.p, FinderPatternEdge::Outer)?;
    let tr_inner = trace_line(image, fp.tr.p, fp.tl.p, FinderPatternEdge::Inner)?;

    if bl_outer.is_valid() && tr_outer.is_valid() && bl_inner.is_valid() && tr_inner.is_valid() {
        // intersect both outer and inner line pairs and take the center point between the two intersection points
        let br_inter = (intersect(&bl_outer, &tr_outer).ok_or(Error::NotFound {
            message: "QR pattern was not detected".into(),
        })? + intersect(&bl_inner, &tr_inner).ok_or(Error::NotFound {
            message: "QR pattern was not detected".into(),
        })?) / 2.0;

        if dimension > 21
            && let Some(br_cp) = locate_alignment_pattern(image, module_size, br_inter)
        {
            br = br_cp.into();
        }

        // if the symbol is tilted or the resolution of the RegressionLines is sufficient, use their intersection
        // as the best estimate (see discussion in #199 and test image estimate-tilt.jpg )
        if !image.is_in(br.p)
            && (estimate_tilt(fp) > 1.1
                || (bl_outer.is_high_res()
                    && bl_inner.is_high_res()
                    && tr_outer.is_high_res()
                    && tr_inner.is_high_res()))
        {
            br = br_inter.into();
        }
    }

    // otherwise the simple estimation used by upstream is used as a best guess fallback
    if !image.is_in(br.p) {
        br = fp.tr - fp.tl + fp.bl;
        br_offset = point_i(0, 0);
    }

    let mut mod_to_pix = mod2_pix(
        dimension,
        br_offset,
        Quadrilateral::from([fp.tl.p, fp.tr.p, br.p, fp.bl.p]),
    )?;

    if dimension_usize >= Version::dimension_for_number(7) {
        let version =
            read_version(image, dimension_usize, mod_to_pix).map_err(|_| Error::NotFound {
                message: "QR pattern was not detected".into(),
            })?;
        let version_dimension = version.dimension();
        let version_dimension_i32 = version_dimension as i32;

        // if the version bits are garbage -> discard the detection
        if (version_dimension_i32 - dimension).abs() > 8 {
            return Err(Error::NotFound {
                message: "QR pattern was not detected".into(),
            }
            .into());
        }
        if version_dimension_i32 != dimension {
            dimension = version_dimension_i32;
            dimension_usize = version_dimension;
            mod_to_pix = mod2_pix(
                dimension,
                br_offset,
                Quadrilateral::from([fp.tl.p, fp.tr.p, br.p, fp.bl.p]),
            )?;
        }
        let ap_m = version.alignment_pattern_centers(); // alignment pattern positions in modules
        let mut ap_p = Matrix::new(ap_m.len(), ap_m.len())?; // found/guessed alignment pattern positions in pixels
        let n = (ap_m.len()) - 1;

        // project the alignment pattern at module coordinates x/y to pixel coordinate based on current mod2_pix
        let project_m2_p = |x, y, mod2_pix: &PerspectiveTransform| -> Result<Point> {
            mod2_pix
                .transform_point(point(ap_m[x] as f32, ap_m[y] as f32).centered())
                .ok_or_else(|| {
                    Error::NotFound {
                        message: "QR pattern was not detected".into(),
                    }
                    .into()
                })
        };

        let mut find_inner_corner_of_concentric_pattern =
            |x, y, fp: ConcentricPattern| -> Result<()> {
                let pc = ap_p.set(x, y, project_m2_p(x, y, &mod_to_pix)?)?;
                if let Some(fp_quad) = find_concentric_pattern_corners(image, fp.p, fp.size, 2) {
                    for c in fp_quad.0 {
                        if Point::distance(c, pc) < (fp.size as f32) / 2.0 {
                            ap_p.set(x, y, c)?;
                        }
                    }
                }
                Ok(())
            };

        find_inner_corner_of_concentric_pattern(0, 0, fp.tl)?;
        find_inner_corner_of_concentric_pattern(0, n, fp.bl)?;
        find_inner_corner_of_concentric_pattern(n, 0, fp.tr)?;

        let best_guess_app = |x, y, ap_p: &Matrix<Point>| -> Result<Point> {
            if let Some(p) = ap_p.get(x, y) {
                return Ok(p);
            }
            project_m2_p(x, y, &mod_to_pix)
        };

        for y in 0..=n {
            for x in 0..=n {
                if ap_p.get(x, y).is_some() {
                    continue;
                }

                let guessed = if x * y == 0 {
                    best_guess_app(x, y, &ap_p)?
                } else {
                    best_guess_app(x - 1, y, &ap_p)? + best_guess_app(x, y - 1, &ap_p)?
                        - best_guess_app(x - 1, y - 1, &ap_p)?
                };
                if let Some(found) = locate_alignment_pattern(image, module_size, guessed) {
                    ap_p.set(x, y, found)?;
                }
            }
        }

        // go over the whole set of alignment patters again and try to fill any remaining gap by using available neighbors as guides
        for y in 0..=n {
            for x in 0..=n {
                if ap_p.get(x, y).is_some() {
                    continue;
                }

                // find the two closest valid alignment pattern pixel positions both horizontally and vertically.
                // The offset walks outward in alternating directions: i=2→+1, 3→-1, 4→+2, 5→-2, ...
                let mut hori = Vec::new();
                let mut verti = Vec::new();
                let mut i = 2;
                while i < 2 * n + 2 && hori.len() < 2 {
                    let xi = x as isize + i as isize / 2 * (if i % 2 != 0 { 1 } else { -1 });
                    if 0 <= xi
                        && xi <= n as isize
                        && let Some(point) = ap_p.get(xi as usize, y)
                    {
                        hori.push(point);
                    }
                    i += 1;
                }
                let mut i = 2;
                while i < 2 * n + 2 && verti.len() < 2 {
                    let yi = y as isize + i as isize / 2 * (if i % 2 != 0 { 1 } else { -1 });
                    if 0 <= yi
                        && yi <= n as isize
                        && let Some(point) = ap_p.get(x, yi as usize)
                    {
                        verti.push(point);
                    }
                    i += 1;
                }

                // if we found 2 each, intersect the two lines that are formed by connecting the point pairs
                if (hori.len()) == 2 && (verti.len()) == 2 {
                    let guessed = intersect(
                        &RegressionLine::with_two_points(hori[0], hori[1]),
                        &RegressionLine::with_two_points(verti[0], verti[1]),
                    )
                    .ok_or(Error::InvalidState {
                        message: "required internal state is missing".into(),
                    })?;
                    let found = locate_alignment_pattern(image, module_size, guessed);
                    // search again near that intersection and if the search fails, use the intersection
                    ap_p.set(x, y, if let Some(f) = found { f } else { guessed })?;
                }
            }
        }

        if let Some(c) = ap_p.get(n, n) {
            mod_to_pix = mod2_pix(
                dimension,
                point_i(3, 3),
                Quadrilateral::from([fp.tl.p, fp.tr.p, c, fp.bl.p]),
            )?;
        }

        // go over the whole set of alignment patters again and fill any remaining gaps by a projection based on an updated mod2_pix
        // projection. This works if the symbol is flat, wich is a reasonable fall-back assumption.
        for y in 0..=n {
            for x in 0..=n {
                if ap_p.get(x, y).is_some() {
                    continue;
                }

                ap_p.set(x, y, project_m2_p(x, y, &mod_to_pix)?)?;
            }
        }

        // assemble a list of region-of-interests based on the found alignment pattern pixel positions

        let mut rois = Vec::new();
        for y in 0..n {
            for x in 0..n {
                let x0 = ap_m[x];
                let x1 = ap_m[x + 1];
                let y0 = ap_m[y];
                let y1 = ap_m[y + 1];
                let module_left = x0 - usize::from(x == 0) * 6;
                let module_top = y0 - usize::from(y == 0) * 6;
                let module_right = x1 + usize::from(x == n - 1) * 7;
                let module_bottom = y1 + usize::from(y == n - 1) * 7;
                rois.push(SamplerControl {
                    p0: point(module_left as f32, module_top as f32),
                    p1: point(module_right as f32, module_bottom as f32),
                    transform: PerspectiveTransform::quadrilateral_to_quadrilateral(
                        Quadrilateral::rectangle_from_xy(
                            x0 as f32, x1 as f32, y0 as f32, y1 as f32, None,
                        ),
                        Quadrilateral::from([
                            ap_p.get(x, y).ok_or(Error::InvalidState {
                                message: "required internal state is missing".into(),
                            })?,
                            ap_p.get(x + 1, y).ok_or(Error::InvalidState {
                                message: "required internal state is missing".into(),
                            })?,
                            ap_p.get(x + 1, y + 1).ok_or(Error::InvalidState {
                                message: "required internal state is missing".into(),
                            })?,
                            ap_p.get(x, y + 1).ok_or(Error::InvalidState {
                                message: "required internal state is missing".into(),
                            })?,
                        ]),
                    )?,
                });
            }
        }
        let grid_sampler = DefaultGridSampler;
        let sampled = grid_sampler.sample_grid(image, dimension_usize, dimension_usize, &rois)?;
        let result = DetectorResult::new(sampled);
        return Ok(result);
    }

    let grid_sampler = DefaultGridSampler;
    let sampled = grid_sampler.sample_grid(
        image,
        dimension_usize,
        dimension_usize,
        &[SamplerControl {
            p1: point(dimension_usize as f32, dimension_usize as f32),
            p0: point_i(0, 0),
            transform: mod_to_pix,
        }],
    )?;
    let result = DetectorResult::new(sampled);
    Ok(result)
}
