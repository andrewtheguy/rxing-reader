/*
 * Copyright 2007 ZXing authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::fmt;

use crate::{Error, Point, point, point_i};
use anyhow::Result;

use super::BitArray;

type BaseType = super::BitFieldBaseType;
const BASE_BITS: usize = super::BIT_FIELD_BASE_BITS;
const BASE_SHIFT: usize = super::BIT_FIELD_SHIFT_BITS;

/// Represents a 2D matrix of bits. In function arguments below, and throughout the common
/// module, x is the column position, and y is the row position. The ordering is always x, y.
/// The origin is at the top-left.
///
/// Internally the bits are represented in a 1-D array of 32-bit ints. However, each row begins
/// with a new int. This is done intentionally so that we can copy out a row into a BitArray very
/// efficiently.
///
/// The ordering of bits is row-major. Within each int, the least significant bits are used first,
/// meaning they represent lower x values. This is compatible with BitArray's implementation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BitMatrix {
    width: u32,
    height: u32,
    row_size: usize,
    bits: Vec<BaseType>,
}

impl BitMatrix {
    /// Creates an empty square `BitMatrix`.
    ///
    /// - `dimension`: height and width
    pub fn with_single_dimension(dimension: u32) -> Result<Self> {
        Self::new(dimension, dimension)
    }

    /// Creates an empty `BitMatrix`.
    ///
    /// - `width`: bit matrix width
    /// - `height`: bit matrix height
    pub fn new(width: u32, height: u32) -> Result<Self> {
        if width < 1 || height < 1 {
            return Err(Error::InvalidArgument {
                message: format!(
                    "BitMatrix::new: both dimensions must be greater than 0 (got width={width}, height={height})"
                ).into(),
            }
            .into());
        }
        Ok(Self {
            width,
            height,
            row_size: (width as usize).div_ceil(BASE_BITS),
            bits: vec![0; (width as usize).div_ceil(BASE_BITS) * height as usize],
        })
    }

    const fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            row_size: 0,
            bits: Vec::new(),
        }
    }

    #[allow(dead_code)]
    const fn with_all_data(
        &self,
        width: u32,
        height: u32,
        row_size: usize,
        bits: Vec<BaseType>,
    ) -> Self {
        Self {
            width,
            height,
            row_size,
            bits,
        }
    }

    /// Interprets a 2D array of booleans as a `BitMatrix`, where "true" means an "on" bit.
    ///
    /// - `image`: bits of the image, as a row-major 2D array. Elements are arrays representing rows
    ///
    /// Returns `BitMatrix` representation of image.
    pub fn parse_bools(image: &[Vec<bool>]) -> Self {
        let Some(first_row) = image.first() else {
            return Self::empty();
        };
        let Ok(height) = image.len().try_into() else {
            return Self::empty();
        };
        let Ok(width) = first_row.len().try_into() else {
            return Self::empty();
        };
        if width == 0 || height == 0 {
            return Self::empty();
        }

        let Ok(mut bits) = BitMatrix::new(width, height) else {
            return Self::empty();
        };
        for (i, image_i) in image.iter().enumerate().take(height as usize) {
            for (j, image_i_j) in image_i.iter().enumerate().take(width as usize) {
                if *image_i_j {
                    bits.set(j as u32, i as u32);
                }
            }
        }
        bits
    }

    pub fn parse_strings(
        string_representation: &str,
        set_string: &str,
        unset_string: &str,
    ) -> Result<Self> {
        let mut bits = vec![false; string_representation.chars().count()];
        let mut bits_pos = 0;
        let mut row_start_pos = 0;
        let mut row_length = 0; //-1;
        let mut first_run = true;
        let mut n_rows = 0;
        let mut pos = 0;
        let chars: Vec<char> = string_representation.chars().collect();
        while pos < chars.len() {
            if chars[pos] == '\n' || chars[pos] == '\r' {
                if bits_pos > row_start_pos {
                    if first_run {
                        first_run = false;
                        row_length = bits_pos - row_start_pos;
                    } else if bits_pos - row_start_pos != row_length {
                        let this_row_length = bits_pos - row_start_pos;
                        return Err(Error::InvalidArgument {
                            message: format!(
                                "parse_strings: row lengths do not match (row {n_rows} has {this_row_length} cells, expected {row_length})"
                            ).into(),
                        }
                        .into());
                    }
                    row_start_pos = bits_pos;
                    n_rows += 1;
                }
                pos += 1;
            } else if string_representation[pos..].starts_with(set_string) {
                pos += set_string.len();
                bits[bits_pos] = true;
                bits_pos += 1;
            } else if string_representation[pos..].starts_with(unset_string) {
                pos += unset_string.len();
                bits[bits_pos] = false;
                bits_pos += 1;
            } else {
                return Err(Error::InvalidArgument {
                    message: format!(
                        "illegal character encountered: {}",
                        string_representation[pos..].to_owned()
                    ).into(),
                }
                .into());
            }
        }

        // no EOL at end?
        if bits_pos > row_start_pos {
            if first_run {
                row_length = bits_pos - row_start_pos;
            } else if bits_pos - row_start_pos != row_length {
                let this_row_length = bits_pos - row_start_pos;
                return Err(Error::InvalidArgument {
                    message: format!(
                        "parse_strings: final row lengths do not match (row {n_rows} has {this_row_length} cells, expected {row_length})"
                    ).into(),
                }
                .into());
            }
            n_rows += 1;
        }

        let mut matrix = BitMatrix::new(row_length as u32, n_rows)?;
        for (i, bit) in bits.iter().enumerate().take(bits_pos) {
            if *bit {
                matrix.set((i % row_length) as u32, (i / row_length) as u32);
            }
        }
        Ok(matrix)
    }

    /// Gets the requested bit, where true means black.
    ///
    /// x The horizontal component (i.e. which column)
    /// y The vertical component (i.e. which row)
    /// returns the value of given bit in matrix, or false if the requested point is out of bounds of the image
    #[inline(always)]
    pub fn get(&self, x: u32, y: u32) -> bool {
        let offset = self.offset(y, x);
        if offset >= self.bits.len() {
            return false;
        }
        ((self.bits[offset] >> (x as usize & BASE_SHIFT)) & 1) != 0
    }

    #[inline(always)]
    pub fn at_point(&self, point: Point) -> bool {
        self.get(point.x as u32, point.y as u32)
    }

    #[inline(always)]
    pub fn at_index<T: Into<usize>>(&self, index: T) -> bool {
        self.at_point(self.calculate_point_from_index(index.into()))
    }

    #[inline(always)]
    fn calculate_point_from_index(&self, index: usize) -> Point {
        let row = index / (self.width() as usize);
        let column = index % (self.width() as usize);
        point_i(column as u32, row as u32)
    }

    #[inline(always)]
    fn offset(&self, y: u32, x: u32) -> usize {
        y as usize * self.row_size + (x as usize / BASE_BITS)
    }

    pub fn try_get(&self, x: u32, y: u32) -> Option<bool> {
        let offset = self.offset(y, x);
        if offset >= self.bits.len() {
            return None;
        }
        Some(((self.bits[offset] >> (x as usize & BASE_SHIFT)) & 1) != 0)
    }

    #[inline(always)]
    pub fn try_at_point(&self, point: Point) -> Option<bool> {
        self.try_get(point.x as u32, point.y as u32)
    }

    pub fn try_area_majority(&self, x: u32, y: u32, box_size: u32) -> Option<bool> {
        let mut matrix = Vec::with_capacity((box_size * box_size) as usize);
        let start_x = (x as i32 - box_size as i32 / 2).max(0) as u32;
        let end_x = x + box_size / 2;
        let start_y = (y as i32 - box_size as i32 / 2).max(0) as u32;
        let end_y = y + box_size / 2;

        for get_x in start_x..=end_x {
            for get_y in start_y..=end_y {
                matrix.push(self.try_get(get_x, get_y)?);
            }
        }

        let total_set = matrix.iter().filter(|bit| **bit).count();
        if (total_set as f32 / matrix.len() as f32) >= 0.5 {
            Some(true)
        } else {
            Some(false)
        }
    }

    #[inline(always)]
    pub fn check_in_bounds(&self, x: u32, y: u32) -> bool {
        self.offset(y, x) < self.bits.len()
    }

    #[inline(always)]
    pub fn check_point_in_bounds(&self, point: Point) -> bool {
        self.check_in_bounds(point.x as u32, point.y as u32)
    }

    /// Sets the given bit to true.
    ///
    /// - `x`: The horizontal component (i.e. which column)
    /// - `y`: The vertical component (i.e. which row)
    #[inline(always)]
    pub fn set(&mut self, x: u32, y: u32) {
        let offset = self.offset(y, x);
        self.bits[offset] |= 1 << (x as usize & BASE_SHIFT);
    }

    #[inline(always)]
    pub fn set_bool(&mut self, x: u32, y: u32, value: bool) {
        if value {
            self.set(x, y)
        } else {
            self.unset(x, y)
        }
    }

    #[inline(always)]
    pub fn unset(&mut self, x: u32, y: u32) {
        let offset = self.offset(y, x);
        self.bits[offset] &= !(1 << (x as usize & BASE_SHIFT));
    }

    /// Flips the given bit.
    ///
    /// - `x`: The horizontal component (i.e. which column)
    /// - `y`: The vertical component (i.e. which row)
    #[inline(always)]
    pub fn flip_coords(&mut self, x: u32, y: u32) {
        let offset = self.offset(y, x);
        self.bits[offset] ^= 1 << (x as usize & BASE_SHIFT);
    }

    /// Flips every bit in the matrix.
    pub fn flip_self(&mut self) {
        let max = self.bits.len();
        for bit_set in self.bits.iter_mut().take(max) {
            *bit_set = !*bit_set;
        }
    }

    /// Exclusive-or (XOR): Flip the bit in this `BitMatrix` if the corresponding
    /// mask bit is set.
    ///
    /// - `mask`: XOR mask
    pub fn xor(&mut self, mask: &BitMatrix) -> Result<()> {
        if self.width != mask.width || self.height != mask.height || self.row_size != mask.row_size
        {
            return Err(Error::InvalidArgument {
                message: format!(
                    "BitMatrix::xor: input matrix dimensions do not match (self={}x{} row_size={}, mask={}x{} row_size={})",
                    self.width, self.height, self.row_size,
                    mask.width, mask.height, mask.row_size,
                ).into(),
            }
            .into());
        }
        for y in 0..self.height {
            let offset = y as usize * self.row_size;
            let row_array = mask.row(y);
            let row = row_array.words();
            for (x, row_x) in row.iter().enumerate().take(self.row_size) {
                self.bits[offset + x] ^= *row_x;
            }
        }
        Ok(())
    }

    /// Clears all bits (sets to false).
    #[inline(always)]
    pub fn clear(&mut self) {
        self.bits.fill(0);
    }

    /// Sets a square region of the bit matrix to true.
    ///
    /// - `left`: The horizontal position to begin at (inclusive)
    /// - `top`: The vertical position to begin at (inclusive)
    /// - `width`: The width of the region
    /// - `height`: The height of the region
    pub fn set_region(&mut self, left: u32, top: u32, width: u32, height: u32) -> Result<()> {
        if height < 1 || width < 1 {
            return Err(Error::InvalidArgument {
                message: format!(
                    "set_region: width and height must be at least 1 (got width={width}, height={height})"
                ).into(),
            }
            .into());
        }
        let right = left + width;
        let bottom = top + height;
        if bottom > self.height || right > self.width {
            return Err(Error::InvalidArgument {
                message: format!(
                    "set_region: region (left={left}, top={top}, width={width}, height={height} -> right={right}, bottom={bottom}) does not fit inside matrix {}x{}",
                    self.width, self.height,
                ).into(),
            }
            .into());
        }
        for y in top..bottom {
            let offset = y as usize * self.row_size;
            for x in left..right {
                self.bits[offset + (x as usize / BASE_BITS)] |= 1 << (x as usize & BASE_SHIFT);
            }
        }
        Ok(())
    }

    /// A fast method to retrieve one row of data from the matrix as a BitArray.
    ///
    /// - `y`: The row to retrieve
    ///
    /// Returns the requested row as a new [`BitArray`].
    pub fn row(&self, y: u32) -> BitArray {
        let mut rw = BitArray::with_size(self.width as usize);

        let offset = y as usize * self.row_size;
        for x in 0..self.row_size {
            rw.set_bulk(x * BASE_BITS, self.bits[offset + x]);
        }
        rw
    }

    /// Returns a column of the bit matrix.
    ///
    /// The current implementation may be very slow.
    pub fn column(&self, x: u32) -> BitArray {
        let mut cw = BitArray::with_size(self.height as usize);

        for y in 0..self.height {
            if self.get(x, y) {
                cw.set(y as usize)
            }
        }

        cw
    }

    /// - `y`: row to set
    /// - `row`: [`BitArray`] to copy from
    pub fn set_row(&mut self, y: u32, row: &BitArray) {
        self.bits[y as usize * self.row_size..y as usize * self.row_size + self.row_size]
            .clone_from_slice(&row.words()[0..self.row_size])
    }

    /// Modifies this `BitMatrix` to represent the same but rotated the given degrees (0, 90, 180, 270)
    ///
    /// - `degrees`: number of degrees to rotate through counter-clockwise (0, 90, 180, 270)
    pub fn rotate(&mut self, degrees: u32) -> Result<()> {
        match degrees % 360 {
            0 => Ok(()),
            90 => {
                self.rotate90();
                Ok(())
            }
            180 => {
                self.rotate180();
                Ok(())
            }
            270 => {
                self.rotate90();
                self.rotate180();
                Ok(())
            }
            other => Err(Error::InvalidArgument {
                message: format!(
                    "rotate: degrees must be 0, 90, 180, or 270 (got {degrees}, normalized to {other})"
                ).into(),
            }
            .into()),
        }
    }

    /// Modifies this `BitMatrix` to represent the same but rotated 180 degrees
    pub fn rotate180(&mut self) {
        let max_height = self.height.div_ceil(2);
        for i in 0..max_height {
            let mut top_row = self.row(i);
            let bottom_row_index = self.height - 1 - i;
            let mut bottom_row = self.row(bottom_row_index);
            top_row.reverse();
            bottom_row.reverse();
            self.set_row(i, &bottom_row);
            self.set_row(bottom_row_index, &top_row);
        }
    }

    /// Modifies this `BitMatrix` to represent the same but rotated 90 degrees counterclockwise
    pub fn rotate90(&mut self) {
        let new_width = self.height;
        let new_height = self.width;
        let new_row_size = new_width.div_ceil(BASE_BITS as u32);
        let mut new_bits = vec![0; (new_row_size * new_height) as usize];

        for y in 0..self.height {
            for x in 0..self.width {
                let offset = self.offset(y, x);
                if ((self.bits[offset] >> (x as usize & BASE_SHIFT)) & 1) != 0 {
                    let new_offset: usize =
                        ((new_height - 1 - x) * new_row_size + (y / BASE_BITS as u32)) as usize;
                    new_bits[new_offset] |= 1 << (y as usize & BASE_SHIFT);
                }
            }
        }
        self.width = new_width;
        self.height = new_height;
        self.row_size = new_row_size as usize;
        self.bits = new_bits;
    }

    /// This is useful in detecting the enclosing rectangle of a pure QR symbol.
    ///
    /// Returns the `left, top, width, height` rectangle enclosing all set bits.
    ///
    /// Returns `None` when the matrix is all white.
    pub fn enclosing_rectangle(&self) -> Option<[u32; 4]> {
        let mut left = self.width;
        let mut top = self.height;
        let mut right: u32 = 0;
        let mut bottom = 0;

        for y in 0..self.height {
            for x32 in 0..self.row_size {
                let the_bits = self.bits[y as usize * self.row_size + x32];
                if the_bits != 0 {
                    top = top.min(y);
                    bottom = bottom.max(y);

                    let bit_lo: usize = the_bits.trailing_zeros() as usize;
                    left = left.min(((x32 * BASE_BITS) + bit_lo) as u32);

                    let bit_hi: usize = (BASE_BITS - 1) - (the_bits.leading_zeros() as usize);
                    right = right.max(((x32 * BASE_BITS) + bit_hi) as u32);
                }
            }
        }

        if right < left || bottom < top {
            return None;
        }

        Some([left, top, right - left + 1, bottom - top + 1])
    }

    /// This is useful in detecting a corner of a pure QR symbol.
    ///
    /// Returns the coordinate of the top-left-most set bit.
    ///
    /// Returns `None` when the matrix is all white.
    pub fn top_left_on_bit(&self) -> Option<Point> {
        let mut bits_offset = 0;
        while bits_offset < self.bits.len() && self.bits[bits_offset] == 0 {
            bits_offset += 1;
        }
        if bits_offset == self.bits.len() {
            return None;
        }
        let y = bits_offset / self.row_size;
        let mut x = (bits_offset % self.row_size) * BASE_BITS;

        let the_bits = self.bits[bits_offset];
        let mut bit = 0;
        while (the_bits << (BASE_SHIFT - bit)) == 0 {
            bit += 1;
        }
        x += bit;
        Some(point(x as f32, y as f32))
    }

    pub fn bottom_right_on_bit(&self) -> Option<Point> {
        let mut bits_offset = self.bits.len() as i64 - 1;
        while bits_offset >= 0 && self.bits[bits_offset as usize] == 0 {
            bits_offset -= 1;
        }
        if bits_offset < 0 {
            return None;
        }

        let y = bits_offset as usize / self.row_size;
        let mut x = (bits_offset as usize % self.row_size) * BASE_BITS;

        let the_bits = self.bits[bits_offset as usize];
        let mut bit = BASE_BITS - 1;
        while (the_bits >> bit) == 0 {
            bit -= 1;
        }
        x += bit;

        Some(point(x as f32, y as f32))
    }

    #[inline(always)]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[inline(always)]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[inline(always)]
    pub fn row_size(&self) -> usize {
        self.row_size
    }

    /// - `set_string`: representation of a set bit
    /// - `unset_string`: representation of an unset bit
    ///
    /// Returns string representation of entire matrix utilizing given strings.
    pub fn to_string(&self, set_string: &str, unset_string: &str) -> String {
        self.build_to_string(set_string, unset_string, "\n")
    }

    /// - `set_string`: representation of a set bit
    /// - `unset_string`: representation of an unset bit
    /// - `line_separator`: newline character in string representation
    ///
    /// Returns string representation of entire matrix utilizing given strings and line separator.
    fn build_to_string(
        &self,
        set_string: &str,
        unset_string: &str,
        line_separator: &str,
    ) -> String {
        let mut result = String::with_capacity(
            (self.height as usize).saturating_mul((self.width as usize).saturating_add(1)),
        );
        for y in 0..self.height {
            for x in 0..self.width {
                result.push_str(if self.get(x, y) {
                    set_string
                } else {
                    unset_string
                });
            }
            result.push_str(line_separator);
        }
        result
    }

    pub fn crop(&self, top: usize, left: usize, height: usize, width: usize) -> Result<BitMatrix> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidArgument {
                message: format!(
                    "crop: width and height must be greater than 0 (got width={width}, height={height})"
                ).into(),
            }
            .into());
        }
        if left.saturating_add(width) > self.width as usize
            || top.saturating_add(height) > self.height as usize
        {
            return Err(Error::InvalidArgument {
                message: format!(
                    "crop: region (left={left}, top={top}, width={width}, height={height}) does not fit inside matrix {}x{}",
                    self.width, self.height,
                ).into(),
            }
            .into());
        }
        let mut new_bm = BitMatrix::new(width as u32, height as u32)?;
        for y in top..top + height {
            for x in left..left + width {
                if self.get(x as u32, y as u32) {
                    let nx = (x - left) as u32;
                    let ny = (y - top) as u32;
                    new_bm.set(nx, ny)
                }
            }
        }
        Ok(new_bm)
    }

    #[inline(always)]
    pub fn is_in(&self, p: Point) -> bool {
        self.is_in_with_border(p, 0)
    }

    #[inline(always)]
    pub fn is_in_with_border(&self, p: Point, b: i32) -> bool {
        b as f32 <= p.x
            && p.x < self.width() as f32 - b as f32
            && b as f32 <= p.y
            && p.y < self.height() as f32 - b as f32
    }
}

impl fmt::Display for BitMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string("X ", "  "))
    }
}

impl TryFrom<&str> for BitMatrix {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        Self::parse_strings(value, "X", " ")
    }
}

impl From<&BitMatrix> for Vec<bool> {
    fn from(value: &BitMatrix) -> Self {
        let mut arr = vec![false; (value.width * value.height) as usize];
        for x in 0..value.width {
            for y in 0..value.height {
                let insert_pos = ((y * value.width) + x) as usize;
                arr[insert_pos] = value.get(x, y);
            }
        }
        arr
    }
}

#[cfg(test)]
mod tests {
    use super::BitMatrix;

    #[test]
    fn parse_bools_handles_empty_input() {
        let image: Vec<Vec<bool>> = Vec::new();
        let matrix = BitMatrix::parse_bools(&image);

        assert_eq!(matrix.width(), 0);
        assert_eq!(matrix.height(), 0);
    }

    #[test]
    fn parse_bools_uses_first_row_width_when_setting_bits() {
        let matrix = BitMatrix::parse_bools(&[vec![true, false], vec![false, true]]);

        assert_eq!(matrix.width(), 2);
        assert_eq!(matrix.height(), 2);
        assert!(matrix.get(0, 0));
        assert!(!matrix.get(1, 0));
        assert!(!matrix.get(0, 1));
        assert!(matrix.get(1, 1));
    }
}
