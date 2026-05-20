use crate::Point;

/// Orders an array of three points in `A, B, C` order such that AB is less than AC
/// and BC is less than AC, and the angle between BC and BA is less than 180 degrees.
///
/// - `patterns`: array of three `Point` to order
pub fn order_best_patterns<T: Into<Point> + Copy>(patterns: &mut [T; 3]) {
    // Find distances between pattern centers
    let zero_one_distance = Point::distance(patterns[0].into(), patterns[1].into());
    let one_two_distance = Point::distance(patterns[1].into(), patterns[2].into());
    let zero_two_distance = Point::distance(patterns[0].into(), patterns[2].into());

    // Assume one closest to other two is B; A and C will just be guesses at first
    let (mut point_a, point_b, mut point_c) =
        if one_two_distance >= zero_one_distance && one_two_distance >= zero_two_distance {
            (patterns[1], patterns[0], patterns[2])
        } else if zero_two_distance >= one_two_distance && zero_two_distance >= zero_one_distance {
            (patterns[0], patterns[1], patterns[2])
        } else {
            (patterns[0], patterns[2], patterns[1])
        };

    // Use cross product to figure out whether A and C are correct or flipped.
    // This asks whether BC x BA has a positive z component, which is the arrangement
    // we want for A, B, C. If it's negative, then we've got it flipped around and
    // should swap A and C.
    if Point::cross_product_z(point_a.into(), point_b.into(), point_c.into()) < 0.0 {
        std::mem::swap(&mut point_a, &mut point_c);
    }

    *patterns = [point_a, point_b, point_c];
}
