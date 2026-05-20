use crate::Error;
use anyhow::Result;

use super::Direction;

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
        return Err(Error::InvalidFormat {
            message: "Invalid value".to_owned(),
        }
        .into());
    }

    Ok(result.iter().collect())
}

pub fn append_bit(val: &mut i32, bit: bool) {
    *val <<= 1;

    *val |= i32::from(bit)
}

