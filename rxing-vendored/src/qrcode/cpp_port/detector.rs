use crate::{
    Exceptions,
    common::{
        DefaultGridSampler, GridSampler, Result, SamplerControl,
        cpp_essentials::{
            append_bit, center_of_ring, DMRegressionLine, find_concentric_pattern_corners,
            find_left_guard_by, Matrix, Value,
        },
    },
    point, point_i,
    qrcode::{
        common::{FormatInformation, Version, VersionRef},
        detector::QRCodeDetectorResult,
    },
};
use multimap::MultiMap;

use crate::{
    Point,
    common::{
        BitMatrix, PerspectiveTransform, Quadrilateral,
        cpp_essentials::{
            BitMatrixCursorTrait, ConcentricPattern, Direction, EdgeTracer, FixedPattern,
            get_pattern_row_tp, is_pattern, locate_concentric_pattern, PatternRow, PatternType,
            PatternView, read_symmetric_pattern, RegressionLine, RegressionLineTrait,
        },
    },
};

use super::Type;

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

fn find_pattern(view: PatternView<'_>) -> Result<PatternView<'_>> {
    find_left_guard_by::<LEN, _>(
        view,
        LEN,
        |view: &PatternView, space_in_pixel: Option<f32>| {
            // perform a fast plausibility test for 1:1:3:1:1 pattern
            if view[2] < 2 as PatternType * std::cmp::max(view[0], view[4])
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
    const MIN_SKIP: u32 = 3; // 1 pixel/module times 3 modules/center
    const MAX_MODULES_FAST: u32 = 20 * 4 + 17; // support up to version 20 for mobile clients

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
        get_pattern_row_tp(image, y, &mut row, false);
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
                    next.iter().sum::<u16>() as i32 * 3,
                ); // 3 for very skewed samples
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

/**
 * @brief generate_finder_pattern_sets
 * @param patterns list of ConcentricPattern objects, i.e. found finder pattern squares
 * @return list of plausible finder pattern sets, sorted by decreasing plausibility
 */
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
                    std::mem::swap(&mut a, &mut b);
                    std::mem::swap(&mut dist_bc2, &mut dist_ac2);
                } else if dist_ab2 >= dist_ac2 && dist_ab2 >= dist_bc2 {
                    std::mem::swap(&mut b, &mut c);
                    std::mem::swap(&mut dist_ab2, &mut dist_ac2);
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

pub fn estimate_module_size(
    image: &BitMatrix,
    a: ConcentricPattern,
    b: ConcentricPattern,
) -> Result<f64> {
    let mut cur = EdgeTracer::new(image, a.p, b.p - a.p);
    if !cur.is_black() {
        return Err(Exceptions::NOT_FOUND);
    }

    let pattern = read_symmetric_pattern::<5, _>(&mut cur, a.size * 2)
        .ok_or(Exceptions::NOT_FOUND)?;

    if !(is_pattern::<E2E, 5, 7, false>(
        &PatternView::new(&PatternRow::new(pattern.to_vec())),
        &PATTERN,
        None,
        0.0,
        0.0,
    ) != 0.0)
    {
        return Err(Exceptions::NOT_FOUND);
    }

    Ok(
        (2 * pattern.iter().sum::<PatternType>() - pattern[0] - pattern[4]) as f64 / 12.0
            * cur.d().length() as f64,
    )
}

pub struct DimensionEstimate {
    dim: i32,
    ms: f64,
    err: i32,
}

impl Default for DimensionEstimate {
    fn default() -> Self {
        Self {
            dim: 0,
            ms: 0.0,
            err: 4,
        }
    }
}

pub fn estimate_dimension(
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

pub fn trace_line(
    image: &BitMatrix,
    p: Point,
    d: Point,
    edge: i32,
) -> Result<impl RegressionLineTrait> {
    let mut cur = EdgeTracer::new(image, p, d - p);
    let mut line = RegressionLine::default();
    line.set_direction_inward(cur.back());

    // collect points inside the black line -> backup on 3rd edge
    cur.step_to_edge(Some(edge), Some(0), Some(edge == 3));
    if edge == 3 {
        cur.turn_back();
    }

    let mut cur_i = EdgeTracer::new(image, cur.p, Point::main_direction(cur.d()));
    // make sure cur_i positioned such that the white->black edge is directly behind
    // Test image: fix-traceline.jpg
    while !bool::from(cur_i.edge_at_back()) {
        if cur_i.edge_at_left().into() {
            cur_i.turn_right();
        } else if cur_i.edge_at_right().into() {
            cur_i.turn_left();
        } else {
            cur_i.step(Some(-1.0));
        }
    }

    for dir in [Direction::Left, Direction::Right] {
        let mut c = EdgeTracer::new(image, cur_i.p, cur_i.direction(dir));
        let mut step_count = (Point::max_abs_component(cur.p - p)) as i32;
        loop {
            line.add(Point::centered(c.p))?;

            step_count -= 1;
            if !(step_count > 0 && c.step_along_edge(dir, Some(true))) {
                break;
            }
        }
    }

    line.evaluate_max_distance(Some(1.0), Some(true));

    Ok(line)
}

// estimate how tilted the symbol is (return value between 1 and 2, see also above)
pub fn estimate_tilt(fp: &FinderPatternSet) -> f64 {
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

pub fn mod2_pix(
    dimension: i32,
    br_offset: Point,
    pix: Quadrilateral,
) -> Result<PerspectiveTransform> {
    let mut quad = Quadrilateral::rectangle(dimension, dimension, Some(3.5));
    quad[2] -= br_offset;

    PerspectiveTransform::quadrilateral_to_quadrilateral(quad, pix)
}

pub fn locate_alignment_pattern(
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
            (estimate + module_size as f32 * 2.25 * d).floor(),
            module_size * 3,
            1,
            false,
        ) else {
            continue;
        };

        // if we did not land on a black pixel the concentric pattern finder will fail
        if !image.get_point(cor) {
            continue;
        }

        if let Some(cor1) = center_of_ring(image, cor.floor(), module_size, 1, true)
            && let Some(cor2) =
                center_of_ring(image, cor.floor(), module_size * 3, -2, true)
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
    dimension: u32,
    mod2_pix: PerspectiveTransform,
) -> Result<VersionRef> {
    let mut bits = [0; 2]; //

    for mirror in [false, true] {
        // Read top-right/bottom-left version info: 3 wide by 6 tall (depending on mirrored)
        let mut version_bits = 0;
        for y in (0..=5).rev() {
            for x in ((dimension - 11)..=(dimension - 9)).rev() {
                let mod_ = if mirror { point_i(y, x) } else { point_i(x, y) };
                let Some(pix) = mod2_pix.transform_point((mod_).centered()) else {
                    version_bits = -1;
                    continue;
                };
                if !image.is_in(pix) {
                    version_bits = -1;
                } else {
                    append_bit(&mut version_bits, image.get_point(pix));
                }
            }
            bits[usize::from(mirror)] = version_bits;
        }
    }

    Version::decode_version_information_pair(bits[0], bits[1])
}

pub fn sample_qr(image: &BitMatrix, fp: &FinderPatternSet) -> Result<QRCodeDetectorResult> {
    // Tolerate one estimator failing — pick the surviving estimate via the
    // existing err-based comparison below. Failure (Err) maps to the
    // `DimensionEstimate::default()` (dim=0, err=4), preserving the prior
    // sentinel-based control flow.
    let top = estimate_dimension(image, fp.tl, fp.tr).unwrap_or_default();
    let left = estimate_dimension(image, fp.tl, fp.bl).unwrap_or_default();

    if top.dim == 0 && left.dim == 0 {
        return Err(Exceptions::NOT_FOUND);
    }

    let best = match top.err.cmp(&left.err) {
        std::cmp::Ordering::Less => top,
        std::cmp::Ordering::Equal => {
            if top.dim > left.dim {
                top
            } else {
                left
            }
        }
        std::cmp::Ordering::Greater => left,
    };

    let mut dimension = best.dim;
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
    let bl2 = trace_line(image, fp.bl.p, fp.tl.p, 2)?;
    let bl3 = trace_line(image, fp.bl.p, fp.tl.p, 3)?;
    let tr2 = trace_line(image, fp.tr.p, fp.tl.p, 2)?;
    let tr3 = trace_line(image, fp.tr.p, fp.tl.p, 3)?;

    if bl2.is_valid() && tr2.is_valid() && bl3.is_valid() && tr3.is_valid() {
        // intersect both outer and inner line pairs and take the center point between the two intersection points
        let br_inter = (DMRegressionLine::intersect(&bl2, &tr2).ok_or(Exceptions::NOT_FOUND)?
            + DMRegressionLine::intersect(&bl3, &tr3).ok_or(Exceptions::NOT_FOUND)?)
            / 2.0;

        if dimension > 21
            && let Some(br_cp) = locate_alignment_pattern(image, module_size, br_inter)
        {
            br = br_cp.into();
        }

        // if the symbol is tilted or the resolution of the RegressionLines is sufficient, use their intersection
        // as the best estimate (see discussion in #199 and test image estimate-tilt.jpg )
        if !image.is_in(br.p)
            && (estimate_tilt(fp) > 1.1
                || (bl2.is_high_res() && bl3.is_high_res() && tr2.is_high_res() && tr3.is_high_res()))
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

    if dimension >= Version::symbol_size(7, Type::Model2).x {
        let version =
            read_version(image, dimension as u32, mod_to_pix).map_err(|_| Exceptions::NOT_FOUND)?;
        let version_dimension = version.get_dimension_for_version() as i32;

        // if the version bits are garbage -> discard the detection
        if (version_dimension - dimension).abs() > 8 {
            return Err(Exceptions::NOT_FOUND);
        }
        if version_dimension != dimension {
            dimension = version_dimension;
            mod_to_pix = mod2_pix(
                dimension,
                br_offset,
                Quadrilateral::from([fp.tl.p, fp.tr.p, br.p, fp.bl.p]),
            )?;
        }
        let ap_m = version.get_alignment_pattern_centers(); // alignment pattern positions in modules
        let mut ap_p = Matrix::new(ap_m.len(), ap_m.len())?; // found/guessed alignment pattern positions in pixels
        let n = (ap_m.len()) - 1;

        // project the alignment pattern at module coordinates x/y to pixel coordinate based on current mod2_pix
        let project_m2_p = |x, y, mod2_pix: &PerspectiveTransform| -> Result<Point> {
            mod2_pix
                .transform_point(Point::centered(point_i(ap_m[x], ap_m[y])))
                .ok_or(Exceptions::NOT_FOUND)
        };

        let mut find_inner_corner_of_concentric_pattern = |x, y, fp: ConcentricPattern| -> Result<()> {
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

                // find the two closest valid alignment pattern pixel positions both horizontally and vertically
                let mut hori = Vec::new();
                let mut verti = Vec::new();
                let mut i = 2;
                while i < 2 * n + 2 && hori.len() < 2 {
                    let xi = x as isize + i as isize / 2 * (if i % 2 != 0 { 1 } else { -1 });
                    if 0 <= xi && xi <= n as isize && ap_p.get(xi as usize, y).is_some() {
                        hori.push(
                            ap_p.get(xi as usize, y)
                                .ok_or(Exceptions::INDEX_OUT_OF_BOUNDS)?,
                        );
                    }
                    i += 1;
                }
                let mut i = 2;
                while i < 2 * n + 2 && verti.len() < 2 {
                    let yi = y as isize + i as isize / 2 * (if i % 2 != 0 { 1 } else { -1 });
                    if 0 <= yi && yi <= n as isize && ap_p.get(x, yi as usize).is_some() {
                        verti.push(
                            ap_p.get(x, yi as usize)
                                .ok_or(Exceptions::INDEX_OUT_OF_BOUNDS)?,
                        );
                    }
                    i += 1;
                }

                // if we found 2 each, intersect the two lines that are formed by connecting the point pairs
                if (hori.len()) == 2 && (verti.len()) == 2 {
                    let guessed = RegressionLine::intersect(
                        &DMRegressionLine::new(hori[0], hori[1]),
                        &DMRegressionLine::new(verti[0], verti[1]),
                    )
                    .ok_or(Exceptions::ILLEGAL_STATE)?;
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
                rois.push(SamplerControl {
                    p0: point_i(x0 - u32::from(x == 0) * 6, y0 - u32::from(y == 0) * 6),
                    p1: point_i(
                        x1 + u32::from(x == n - 1) * 7,
                        y1 + u32::from(y == n - 1) * 7,
                    ),
                    transform: PerspectiveTransform::quadrilateral_to_quadrilateral(
                        Quadrilateral::rectangle_from_xy(
                            x0 as f32, x1 as f32, y0 as f32, y1 as f32, None,
                        ),
                        Quadrilateral::from([
                            ap_p.get(x, y).ok_or(Exceptions::ILLEGAL_STATE)?,
                            ap_p.get(x + 1, y).ok_or(Exceptions::ILLEGAL_STATE)?,
                            ap_p.get(x + 1, y + 1).ok_or(Exceptions::ILLEGAL_STATE)?,
                            ap_p.get(x, y + 1).ok_or(Exceptions::ILLEGAL_STATE)?,
                        ]),
                    )?,
                });
            }
        }
        let grid_sampler = DefaultGridSampler;
        let (sampled, rp) =
            grid_sampler.sample_grid(image, dimension as u32, dimension as u32, &rois)?;
        let result = QRCodeDetectorResult::new(sampled, rp.to_vec());
        return Ok(result);
    }

    let grid_sampler = DefaultGridSampler;
    let (sampled, rps) = grid_sampler.sample_grid(
        image,
        dimension as u32,
        dimension as u32,
        &[SamplerControl {
            p1: point_i(dimension as u32, dimension as u32),
            p0: point_i(0, 0),
            transform: mod_to_pix,
        }],
    )?;
    let result = QRCodeDetectorResult::new(sampled, rps.to_vec());
    Ok(result)
}

pub fn sample_mqr(image: &BitMatrix, fp: ConcentricPattern) -> Result<QRCodeDetectorResult> {
    let Some(fp_quad) = find_concentric_pattern_corners(image, fp.p, fp.size, 2) else {
        return Err(Exceptions::NOT_FOUND);
    };

    let src_quad = Quadrilateral::rectangle(7, 7, Some(0.5));

    let format_info_coords: [Point; 17] = [
        point_i(0, 8),
        point_i(1, 8),
        point_i(2, 8),
        point_i(3, 8),
        point_i(4, 8),
        point_i(5, 8),
        point_i(6, 8),
        point_i(7, 8),
        point_i(8, 8),
        point_i(8, 7),
        point_i(8, 6),
        point_i(8, 5),
        point_i(8, 4),
        point_i(8, 3),
        point_i(8, 2),
        point_i(8, 1),
        point_i(8, 0),
    ];

    let mut best_fi = FormatInformation::default();
    let mut best_pt = PerspectiveTransform::quadrilateral_to_quadrilateral(
        src_quad,
        fp_quad.rotated_corners(Some(0), None),
    )?;
    let cur = EdgeTracer::new(image, Point::default(), Point::default());

    for i in 0..4 {
        let mod2_pix = PerspectiveTransform::quadrilateral_to_quadrilateral(
            src_quad,
            fp_quad.rotated_corners(Some(i), None),
        )?;

        let check = |i, check_one: bool| {
            mod2_pix
                .transform_point(Point::centered(format_info_coords[i]))
                .is_some_and(|p| image.is_in(p) && (!check_one || image.get_point(p)))
        };

        // check that we see both innermost timing pattern modules
        if !check(0, true) || !check(8, false) || !check(16, true) {
            continue;
        }

        let mut format_info_bits = 0;
        for info_coord in format_info_coords.iter().take(15 + 1).skip(1) {
            append_bit(
                &mut format_info_bits,
                mod2_pix
                    .transform_point(Point::centered(*info_coord))
                    .is_some_and(|p| cur.black_at(p)),
            );
        }

        let fi = FormatInformation::decode_mqr(format_info_bits as u32);
        if fi.hamming_distance < best_fi.hamming_distance {
            best_fi = fi;
            best_pt = mod2_pix;
        }
    }

    if !best_fi.is_valid() {
        return Err(Exceptions::NOT_FOUND);
    }

    let dim: u32 = Version::symbol_size(best_fi.micro_version, Type::Micro).x as u32;

    // check that we are in fact not looking at a corner of a non-micro QRCode symbol
    // we accept at most 1/3rd black pixels in the quite zone (in a QRCode symbol we expect about 1/2).
    let mut black_pixels = 0;
    for i in 0..dim {
        let px = best_pt.transform_point(Point::centered(point_i(i, dim)));
        let py = best_pt.transform_point(Point::centered(point_i(dim, i)));
        if let Some(px) = px {
            black_pixels += u32::from(cur.black_at(px));
        }
        if let Some(py) = py {
            black_pixels += u32::from(cur.black_at(py) || (image.is_in(py) && image.get_point(py)));
        }
    }
    if black_pixels > 2 * dim / 3 {
        return Err(Exceptions::NOT_FOUND);
    }

    let grid_sampler = DefaultGridSampler;
    let (sample, rps) = grid_sampler.sample_grid(
        image,
        dim,
        dim,
        &[SamplerControl {
            p1: point_i(dim, dim),
            p0: point_i(0, 0),
            transform: best_pt,
        }],
    )?;
    Ok(QRCodeDetectorResult::new(sample, rps.to_vec()))
}

pub fn sample_rmqr(image: &BitMatrix, fp: ConcentricPattern) -> Result<QRCodeDetectorResult> {
    // TODO proper
    let Some(fp_quad) = find_concentric_pattern_corners(image, fp.p, fp.size, 2) else {
        return Err(Exceptions::NOT_FOUND);
    };

    let src_quad = Quadrilateral::rectangle(7, 7, Some(0.5));

    let format_info_edge_coords: [Point; 4] =
        [point_i(8, 0), point_i(9, 0), point_i(10, 0), point_i(11, 0)];
    let format_info_coords: [Point; 18] = [
        point_i(11, 3),
        point_i(11, 2),
        point_i(11, 1),
        point_i(10, 5),
        point_i(10, 4),
        point_i(10, 3),
        point_i(10, 2),
        point_i(10, 1),
        point_i(9, 5),
        point_i(9, 4),
        point_i(9, 3),
        point_i(9, 2),
        point_i(9, 1),
        point_i(8, 5),
        point_i(8, 4),
        point_i(8, 3),
        point_i(8, 2),
        point_i(8, 1),
    ];

    let mut best_fi: FormatInformation = FormatInformation::default();
    let mut best_pt: PerspectiveTransform = PerspectiveTransform::default();
    let cur = EdgeTracer::new(image, Point::default(), Point::default());

    for i in 0..4 {
        let mod2_pix = PerspectiveTransform::quadrilateral_to_quadrilateral(
            src_quad,
            fp_quad.rotated_corners(Some(i), None),
        )?;

        let check = |i: usize, on: bool| {
            mod2_pix
                .transform_point(Point::centered(format_info_edge_coords[i]))
                .is_some_and(|p| cur.test_at(p) == Value::from(on))
        };

        // check that we see top edge timing pattern modules
        if !check(0, true) || !check(1, false) || !check(2, true) || !check(3, false) {
            continue;
        }

        let mut format_info_bits = 0;
        for coord in format_info_coords {
            append_bit(
                &mut format_info_bits,
                mod2_pix
                    .transform_point(Point::centered(coord))
                    .is_some_and(|p| cur.black_at(p)),
            );
        }

        let fi = FormatInformation::decode_rmqr(format_info_bits as u32, 0 /*format_info_bits2*/);
        if fi.hamming_distance < best_fi.hamming_distance {
            best_fi = fi;
            best_pt = mod2_pix;
        }
    }

    if !best_fi.is_valid() {
        return Err(Exceptions::NOT_FOUND);
    }

    let dim = Version::symbol_size(best_fi.micro_version, Type::RectMicro);

    // TODO: this is a WIP
    let intersect_quads = |a: &Quadrilateral, b: &Quadrilateral| -> Result<Quadrilateral> {
        let tl = a.center();
        let br = b.center();
        // rotate points such that top_left of a is furthest away from b and top_left of b is closest to a
        let offset_atarget =
            a.0.iter()
                .max_by(|a, b| {
                    Point::distance(**a, br)
                        .partial_cmp(&Point::distance(**b, br))
                        .unwrap_or(std::cmp::Ordering::Less)
                })
                .ok_or(Exceptions::FORMAT)?;
        let offset_a =
            a.0.iter()
                .position(|x| x == offset_atarget)
                .ok_or(Exceptions::FORMAT)? as i32;
        let offset_btarget =
            b.0.iter()
                .min_by(|a, b| {
                    Point::distance(**a, tl)
                        .partial_cmp(&Point::distance(**b, tl))
                        .unwrap_or(std::cmp::Ordering::Less)
                })
                .ok_or(Exceptions::FORMAT)?;
        let offset_b =
            b.0.iter()
                .position(|x| x == offset_btarget)
                .ok_or(Exceptions::FORMAT)? as i32;

        let a = a.rotated_corners(Some(offset_a), None);
        let b = b.rotated_corners(Some(offset_b), None);
        let tr = (RegressionLine::intersect(
            &RegressionLine::with_two_points(a[0], a[1]),
            &RegressionLine::with_two_points(b[1], b[2]),
        )
        .ok_or(Exceptions::FORMAT)?
            + RegressionLine::intersect(
                &RegressionLine::with_two_points(a[3], a[2]),
                &RegressionLine::with_two_points(b[0], b[3]),
            )
            .ok_or(Exceptions::FORMAT)?)
            / 2.0;

        let bl = (RegressionLine::intersect(
            &RegressionLine::with_two_points(a[0], a[3]),
            &RegressionLine::with_two_points(b[2], b[3]),
        )
        .ok_or(Exceptions::FORMAT)?
            + RegressionLine::intersect(
                &RegressionLine::with_two_points(a[1], a[2]),
                &RegressionLine::with_two_points(b[0], b[1]),
            )
            .ok_or(Exceptions::FORMAT)?)
            / 2.0;

        Ok(Quadrilateral::from([tl, tr, br, bl]))
    };

    let alignment_estimate = best_pt
        .transform_point(Into::<Point>::into(dim) - point(3.0, 3.0))
        .ok_or(Exceptions::NOT_FOUND)?;
    if let Some(found) = locate_alignment_pattern(image, fp.size / 7, alignment_estimate)
        && let Some(sp_quad) = find_concentric_pattern_corners(image, found, fp.size / 2, 1)
    {
        let mut dest = intersect_quads(&fp_quad, &sp_quad)?;
        if dim.y <= 9 {
                best_pt = PerspectiveTransform::quadrilateral_to_quadrilateral(
                    Quadrilateral::from([
                        point(6.5, 0.5),
                        point(dim.x as f32 - 1.5, dim.y as f32 - 3.5),
                        point(dim.x as f32 - 1.5, dim.y as f32 - 1.5),
                        point(6.5, 6.5),
                    ]),
                    Quadrilateral::from([
                        *fp_quad.top_right(),
                        *sp_quad.top_right(),
                        *sp_quad.bottom_right(),
                        *fp_quad.bottom_right(),
                    ]),
                )?;
            } else {
                dest[0] = fp.p;
                dest[2] = found;
                best_pt = PerspectiveTransform::quadrilateral_to_quadrilateral(
                    Quadrilateral::from([
                        point(3.5, 3.5),
                        point(dim.x as f32 - 2.5, 3.5),
                        point(dim.x as f32 - 2.5, dim.y as f32 - 2.5),
                        point(3.5, dim.y as f32 - 2.5),
                    ]),
                    dest,
                )?;
            }
    }

    let grid_sampler = DefaultGridSampler;
    let (sample, rps) = grid_sampler.sample_grid(
        image,
        dim.x as u32,
        dim.y as u32,
        &[SamplerControl {
            p1: point_i(dim.x, dim.y),
            p0: point_i(0, 0),
            transform: best_pt,
        }],
    )?;
    Ok(QRCodeDetectorResult::new(sample, rps.to_vec()))
    //  SampleGrid(image, dim.x, dim.y, best_pt)
}
