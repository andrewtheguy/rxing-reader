use crate::{Error, Point};
use anyhow::Result;

use super::{Direction, LINE_INTERSECTION_EPS, RegressionLineTrait};

#[inline(always)]
pub fn intersect<T: RegressionLineTrait, T2: RegressionLineTrait>(
    l1: &T,
    l2: &T2,
) -> Result<Point> {
    if !(l1.is_valid() && l2.is_valid()) {
        return Err(Error::InvalidState { message: "required internal state is missing".to_owned() }.into());
    }
    let d = l1.a() * l2.b() - l1.b() * l2.a();
    if d.abs() < LINE_INTERSECTION_EPS {
        return Err(Error::InvalidState { message: "required internal state is missing".to_owned() }.into());
    }
    let x = (l1.c() * l2.b() - l1.b() * l2.c()) / d;
    let y = (l1.a() * l2.c() - l1.c() * l2.a()) / d;
    Ok(Point { x, y })
}

#[allow(dead_code)]
#[inline(always)]
pub fn opposite(dir: Direction) -> Direction {
    if dir == Direction::Left {
        Direction::Right
    } else {
        Direction::Left
    }
}

#[inline(always)]
pub fn update_min_max<T: Ord + Copy>(min: &mut T, max: &mut T, val: T) {
    *min = std::cmp::min(*min, val);
    *max = std::cmp::max(*max, val);
}

#[inline(always)]
pub fn update_min_max_float(min: &mut f64, max: &mut f64, val: f64) {
    *min = f64::min(*min, val);
    *max = f64::max(*max, val);
}

pub fn to_string<T: Into<usize>>(val: T, len: usize) -> Result<String> {
    let mut val: usize = val.into();
    let mut result = vec!['0'; len];
    let mut idx = len;
    while idx > 0 && val != 0 {
        idx -= 1;
        result[idx] = char::from(b'0' + (val % 10) as u8);
        val /= 10;
    }
    if val != 0 {
        return Err(Error::InvalidFormat { message: "Invalid value".to_owned() }.into());
    }

    Ok(result.iter().collect())
}

pub fn to_int(a: &[u32]) -> Option<u32> {
    let total_bits = a.iter().map(|&v| u64::from(v)).sum::<u64>();
    if total_bits > 32 {
        return None;
    }

    let mut pattern: u32 = 0;
    for (i, element) in a.iter().copied().enumerate() {
        if element > 32 {
            return None;
        }
        let shifted = pattern.checked_shl(element).unwrap_or(0);
        let mask = !0xffffffffu32.checked_shl(element).unwrap_or(0);
        pattern = shifted | (mask * ((!i & 1) as u32));
    }

    Some(pattern)
}

pub fn append_bit(val: &mut i32, bit: bool) {
    *val <<= 1;

    *val |= i32::from(bit)
}

pub fn to_int_pos(bits: &[u8], pos: usize, count: usize) -> Option<u32> {
    let count = std::cmp::min(count, bits.len().saturating_sub(pos));
    let mut res = 0;
    for bit in bits.iter().skip(pos).take(count) {
        append_bit(&mut res, *bit != 0);
    }

    Some(res as u32)
}
