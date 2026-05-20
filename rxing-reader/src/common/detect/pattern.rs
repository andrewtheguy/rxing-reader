/*
* Copyright 2020 Axel Waggershauser
*/
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

use crate::{Error, common::BitMatrix};

pub type PatternType = u16;
pub type Pattern<const N: usize> = [PatternType; N];

fn bar_and_space_sum<
    const LEN: usize,
    T: Into<RT> + Copy,
    RT: Default + std::cmp::PartialEq + std::ops::AddAssign,
>(
    view: &[T],
) -> BarAndSpace<RT> {
    let mut res = BarAndSpace::default();
    for i in 0..LEN {
        res[i] += view[i].into();
    }
    res
}

#[derive(Default, Debug)]
pub struct PatternRow(Vec<PatternType>);

impl PatternRow {
    pub fn new(v: Vec<PatternType>) -> Self {
        Self(v)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn sum(&self) -> PatternType {
        self.0.iter().sum()
    }

    pub fn rev(&mut self) {
        self.0.reverse()
    }

    pub fn as_slice(&self) -> &[PatternType] {
        &self.0
    }
}

impl IntoIterator for PatternRow {
    type Item = PatternType;

    type IntoIter = std::vec::IntoIter<PatternType>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl std::ops::Index<usize> for PatternRow {
    type Output = PatternType;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for PatternRow {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl From<Vec<PatternType>> for PatternRow {
    fn from(value: Vec<PatternType>) -> Self {
        Self(value)
    }
}

pub struct PatternViewIterator<'a> {
    pattern_view: &'a PatternView<'a>,
    current_position: usize,
}

impl Iterator for PatternViewIterator<'_> {
    type Item = PatternType;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_position + 1 > self.pattern_view.count {
            return None;
        }

        self.current_position += 1;

        Some(
            *self.pattern_view.data.get(
                self.current_position - 1 + self.pattern_view.start + self.pattern_view.current,
            )?,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PatternView<'a> {
    data: &'a [PatternType],
    start: usize,
    count: usize,
    current: usize,
}

impl<'a> PatternView<'a> {
    // A PatternRow always starts with the width of whitespace in front of the
    // first black bar (see `fill_pattern_row`). `start = 1` skips that
    // leading-whitespace element, so `view[0]` is the first bar; `count`
    // is the number of view elements (bars + spaces, excluding the
    // leading-whitespace prefix).
    pub fn new(bars: &'a PatternRow) -> PatternView<'a> {
        PatternView {
            data: &bars.0,
            start: 1,
            count: bars.0.len().saturating_sub(1),
            current: 0,
        }
    }

    /// Construct a view directly over a bar-only buffer (no leading-whitespace
    /// element at index 0). `view[0]` is `bars[0]`; use this for buffers that
    /// were built as raw bar-width arrays rather than fed through
    /// `fill_pattern_row`.
    pub fn from_bars(bars: &'a [PatternType]) -> PatternView<'a> {
        PatternView {
            data: bars,
            start: 0,
            count: bars.len(),
            current: 0,
        }
    }

    pub fn with_config(
        bars: &'a PatternRow,
        start: usize,
        size: usize,
        base: usize,
        _end: usize,
    ) -> PatternView<'a> {
        PatternView {
            data: &bars.0,
            start,
            count: size,
            current: base,
        }
    }

    pub fn data(&self) -> &[PatternType] {
        self.data
    }
    pub fn begin(&self) -> Option<PatternType> {
        Some(*self.data.get(self.start)?)
    }
    pub fn end(&self) -> Option<PatternType> {
        Some(self.data.len() as PatternType)
    }

    pub fn sum(&self) -> PatternType {
        self.sum_first(self.count)
    }

    pub fn sum_first(&self, n: usize) -> PatternType {
        self.data
            .iter()
            .skip(self.start + self.current)
            .take(n)
            .copied()
            .sum::<PatternType>()
    }

    pub fn iter(&'a self) -> PatternViewIterator<'a> {
        PatternViewIterator {
            pattern_view: self,
            current_position: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.count
    }

    // index is the number of bars and spaces from the first bar to the current position
    pub fn index(&self) -> usize {
        self.current
    }
    pub fn pixels_in_front(&self) -> PatternType {
        self.data
            .iter()
            .take(self.start + self.current)
            .copied()
            .sum::<PatternType>()
    }
    pub fn pixels_till_end(&self) -> PatternType {
        self.data
            .iter()
            .take(self.start + self.current + self.count)
            .copied()
            .sum::<PatternType>()
            .saturating_sub(1)
    }
    pub fn is_at_first_bar(&self) -> bool {
        self.current == 0
    }
    pub fn is_at_last_bar(&self) -> bool {
        self.current == self.start + self.count - 1
    }
    pub fn is_valid_with_n(&self, n: usize) -> bool {
        !self.data.is_empty()
            && self.start <= self.current + self.start
            && self.current + n < (self.data.len())
    }
    pub fn is_valid(&self) -> bool {
        self.is_valid_with_n(self.size())
    }

    pub fn has_quiet_zone_before(&self, scale: f32, accept_if_at_first_bar: bool) -> bool {
        if accept_if_at_first_bar && self.is_at_first_bar() {
            return true;
        }
        let prev_idx = (self.start + self.current).checked_sub(1);
        match prev_idx.and_then(|i| self.data.get(i)) {
            Some(v) => Into::<f32>::into(*v) >= Into::<f32>::into(self.sum()) * scale,
            None => false,
        }
    }

    pub fn has_quiet_zone_after(&self, scale: f32, accept_if_at_last_bar: bool) -> bool {
        if accept_if_at_last_bar && self.is_at_last_bar() {
            return true;
        }
        match self.data.get(self.start + self.current + self.count) {
            Some(v) => Into::<f32>::into(*v) >= Into::<f32>::into(self.sum()) * scale,
            None => false,
        }
    }

    pub fn sub_view(&self, offset: usize, size: usize) -> PatternView<'a> {
        PatternView {
            data: self.data,
            start: self.start + offset,
            count: size,
            current: self.current,
        }
    }

    pub fn shift(&mut self, n: usize) -> bool {
        self.current += n;
        !self.data.is_empty()
    }

    pub fn skip_pair(&mut self) -> bool {
        self.shift(2)
    }

    pub fn skip_symbol(&mut self) -> bool {
        self.shift(self.count)
    }

    pub fn skip_single(&mut self) -> bool {
        self.shift(1)
    }

    pub fn extend(&mut self) {
        self.count = std::cmp::max(
            0,
            self.data.len() as isize - (self.current + self.start) as isize,
        ) as usize
    }

    /// Translate a signed view-relative index into an absolute `self.data`
    /// position, returning `None` when the index would land outside the
    /// underlying buffer. Returns the position rather than the value so
    /// `Index<isize>` / `Index<i32>` can resolve to `&self.data[spot]`
    /// without recomputing the offset.
    fn try_index(&self, index: isize) -> Option<usize> {
        if index.abs() > self.data.len() as isize {
            return None;
        }
        let base = (self.start + self.current) as isize;
        if index < 0 && index.abs() > base {
            return None;
        }
        let spot = (base + index) as usize;
        (spot < self.data.len()).then_some(spot)
    }
}

impl std::ops::Index<isize> for PatternView<'_> {
    type Output = PatternType;

    fn index(&self, index: isize) -> &Self::Output {
        let spot = self.try_index(index).unwrap_or_else(|| {
            panic!(
                "index out of bounds: the len is {} but the index is {}",
                self.count, index
            )
        });
        &self.data[spot]
    }
}

impl std::ops::Index<usize> for PatternView<'_> {
    type Output = PatternType;

    fn index(&self, index: usize) -> &Self::Output {
        match self.data.get(self.start + self.current + index) {
            Some(value) => value,
            None => panic!(
                "index out of bounds: the len is {} but the index is {}",
                self.count, index
            ),
        }
    }
}

impl std::ops::Index<i32> for PatternView<'_> {
    type Output = PatternType;

    fn index(&self, index: i32) -> &Self::Output {
        std::ops::Index::<isize>::index(self, index as isize)
    }
}

impl<'a> From<&PatternView<'a>> for Vec<PatternType> {
    fn from(value: &PatternView<'a>) -> Self {
        let mut v = vec![PatternType::default(); value.count];
        for i in 0..value.count {
            v[i] = value[i];
        }
        v
    }
}

impl<'a, const LEN: usize> From<&PatternView<'a>> for [PatternType; LEN] {
    fn from(value: &PatternView<'a>) -> Self {
        let mut result = [PatternType::default(); LEN];
        let copy_len = std::cmp::min(value.count, LEN);
        for i in 0..copy_len {
            result[i] = value[i];
        }
        result
    }
}

impl<'a> From<&PatternView<'a>> for &'a [PatternType] {
    fn from(value: &PatternView<'a>) -> Self {
        let start_idx = value.current + value.start;
        let end_idx = (start_idx + value.count).min(value.data.len());
        &value.data[start_idx..end_idx]
    }
}

/// The indexed operator can be used in combination with a `PatternView`.
#[derive(Default, Clone)]
pub struct BarAndSpace<T: Default + std::cmp::PartialEq> {
    bar: T,
    space: T,
}
impl<T: Default + std::cmp::PartialEq> BarAndSpace<T> {
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        self.bar != T::default() && self.space != T::default()
    }
}

impl<T: Default + std::cmp::PartialEq> std::ops::Index<usize> for BarAndSpace<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index & 1 == 0 {
            &self.bar
        } else {
            &self.space
        }
    }
}

impl<T: Default + std::cmp::PartialEq> std::ops::IndexMut<usize> for BarAndSpace<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index & 1 == 0 {
            &mut self.bar
        } else {
            &mut self.space
        }
    }
}

/// - `N`: number of bars/spaces
/// - `SUM`: sum over all N elements (size of pattern in modules)
/// - `IS_SPARCE`: whether or not the pattern contains '0's denoting 'wide' bars/spaces
pub struct FixedPattern<const N: usize, const SUM: usize, const IS_SPARCE: bool = false> {
    data: [PatternType; N],
}

impl<const N: usize, const SUM: usize, const IS_SPARCE: bool> From<FixedPattern<N, SUM, IS_SPARCE>>
    for Pattern<N>
{
    fn from(val: FixedPattern<N, SUM, IS_SPARCE>) -> Self {
        val.data
    }
}

impl<const N: usize, const SUM: usize, const IS_SPARCE: bool> FixedPattern<N, SUM, IS_SPARCE> {
    pub const fn new(data: [PatternType; N]) -> Self {
        FixedPattern { data }
    }

    pub fn with_reference(data: &[PatternType; N]) -> Self {
        FixedPattern { data: *data }
    }

    pub fn as_slice(&self) -> &[PatternType] {
        &self.data
    }

    pub fn size(&self) -> usize {
        N
    }

    fn sums(&self) -> BarAndSpace<PatternType> {
        bar_and_space_sum::<N, PatternType, PatternType>(&self.data)
    }
}

impl<const N: usize, const SUM: usize, const IS_SPARCE: bool> std::ops::Index<usize>
    for FixedPattern<N, SUM, IS_SPARCE>
{
    type Output = PatternType;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

pub fn is_pattern<const E2E: bool, const LEN: usize, const SUM: usize, const SPARSE: bool>(
    view: &PatternView,
    pattern: &FixedPattern<LEN, SUM, SPARSE>,
    space_in_pixel: Option<f32>,
    min_quiet_zone: f32,
    module_size_ref: f32,
) -> f32 {
    let mut module_size_ref = module_size_ref;
    let view_data: &[PatternType] = view.into();
    let pattern_data = pattern.as_slice();
    if view_data.len() < LEN {
        return 0.0;
    }

    if E2E {
        let widths = bar_and_space_sum::<LEN, PatternType, f64>(view_data);
        let sums = pattern.sums();
        let mod_size: BarAndSpace<f64> = BarAndSpace {
            bar: widths[0] / sums[0] as f64,
            space: widths[1] / sums[1] as f64,
        };

        let [min_module_size, max_module_size] = [
            f64::min(mod_size[0], mod_size[1]),
            f64::max(mod_size[0], mod_size[1]),
        ];
        if max_module_size > 4.0 * min_module_size {
            // make sure module sizes of bars and spaces are not too far away from each other
            return 0.0;
        }

        if min_quiet_zone != 0.0
            && (space_in_pixel.unwrap_or_default()) < min_quiet_zone * mod_size.space as f32
        {
            return 0.0;
        }

        let thr: BarAndSpace<f64> = BarAndSpace {
            bar: mod_size[0] * 0.75 + 0.5,
            space: mod_size[1] / (2.0 + f64::from(LEN < 6)) + 0.5,
        };

        for x in 0..LEN {
            if (view_data[x] as f64 - pattern_data[x] as f64 * mod_size[x]).abs() > thr[x] {
                return 0.0;
            }
        }

        let module_size: f64 = (mod_size[0] + mod_size[1]) / 2.0;
        return module_size as f32;
    }

    let width = view.sum_first(LEN);
    if SUM == 0 {
        return 0.0;
    }
    if SUM > LEN && Into::<usize>::into(width) < SUM {
        return 0.0;
    }

    let module_size: f32 = (Into::<f32>::into(width)) / (SUM as f32);

    if min_quiet_zone != 0.0
        && (space_in_pixel.unwrap_or(f32::MAX)) < min_quiet_zone * module_size - 1.0
    {
        return 0.0;
    }

    if module_size_ref == 0.0 {
        module_size_ref = module_size;
    }

    let threshold = module_size_ref * (0.5 + (E2E as u8) as f32 * 0.25) + 0.5;

    // the offset of 0.5 is to make the code less sensitive to quantization errors for small (near 1) module sizes.
    // TODO: review once we have upsampling in the binarizer in place.

    for x in 0..LEN {
        if (Into::<f32>::into(view_data[x]) - Into::<f32>::into(pattern_data[x]) * module_size_ref)
            .abs()
            > threshold
        {
            return 0.0;
        }
    }

    module_size
}

pub fn find_left_guard_by<const LEN: usize, Pred: Fn(&PatternView, Option<f32>) -> bool>(
    view: PatternView<'_>,
    min_size: usize,
    is_guard: Pred,
) -> Result<PatternView<'_>> {
    const PREV_IDX: isize = -1;

    if view.size() < min_size {
        return Err(Error::InvalidState {
            message: "required internal state is missing".into(),
        }
        .into());
    }

    let mut window = view.sub_view(0, LEN);
    if window.is_at_first_bar() && is_guard(&window, Some(f32::MAX)) {
        return Ok(window);
    }
    let end = Into::<usize>::into(view.end().ok_or_else(|| Error::InvalidState {
        message: "pattern view has no end index".into(),
    })?) - min_size;
    while (window.start + window.current) < end {
        let prev = window
            .try_index(PREV_IDX)
            .map(|spot| window.data[spot] as f32);
        if is_guard(&window, prev) {
            return Ok(window);
        }

        window.skip_pair();
    }

    Err(Error::InvalidState {
        message: "required internal state is missing".into(),
    }
    .into())
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Color {
    White = 0,
    Black = 1,
}

impl<T: Into<PatternType>> From<T> for Color {
    fn from(value: T) -> Self {
        match value.into() {
            0 => Color::White,
            _ => Color::Black,
        }
    }
}

pub fn read_pattern_row(matrix: &BitMatrix, r: u32, pr: &mut PatternRow, transpose: bool) {
    let row = if transpose {
        matrix.column(r)
    } else {
        matrix.row(r)
    };

    let pixel_states: Vec<bool> = row.into();

    fill_pattern_row(&pixel_states, pr)
}

pub fn fill_pattern_row<T: Into<PatternType> + Copy + Default + From<T>>(
    b_row: &[T],
    p_row: &mut PatternRow,
) {
    p_row.0.clear();

    if Color::from(b_row.first().copied().unwrap_or_default()) == Color::Black {
        p_row.0.push(0);
    }

    let mut current_color = Color::from(b_row.first().copied().unwrap_or_default());
    let mut count = 0;

    for bit in b_row.iter() {
        let this_color = Color::from(*bit);

        if current_color != this_color {
            p_row.0.push(count);
            count = 0;

            current_color = this_color;
        }

        count += 1;
    }

    if count != 0 {
        p_row.0.push(count);
    }

    if current_color == Color::Black {
        p_row.0.push(0);
    }
}

#[cfg(test)]
mod tests {
    use crate::common::detect::PatternType;

    use super::{FixedPattern, PatternRow, PatternView, fill_pattern_row, is_pattern};
    const N: usize = 33;

    #[test]
    fn all_white() {
        for s in 1..=N {
            let t_in: Vec<PatternType> = vec![0; s];
            let mut pr = PatternRow::default();
            fill_pattern_row(&t_in, &mut pr);

            assert_eq!(pr.0.len(), 1);
            assert_eq!(pr.0[0], s as PatternType);
        }
    }

    #[test]
    fn all_black() {
        for s in 1..=N {
            let t_in: Vec<PatternType> = vec![0xff; s];
            let mut pr = PatternRow::default();
            fill_pattern_row(&t_in, &mut pr);

            assert_eq!(pr.0.len(), 3);
            assert_eq!(pr.0[0], 0);
            assert_eq!(pr.0[1], s as PatternType);
            assert_eq!(pr.0[2], 0);
        }
    }

    #[test]
    fn black_white() {
        for s in 1..=N {
            let mut t_in: Vec<PatternType> = vec![0; N];
            t_in[..s].copy_from_slice(&vec![1; s]);
            let mut pr = PatternRow::default();
            fill_pattern_row(&t_in, &mut pr);

            assert_eq!(pr.0.len(), 3);
            assert_eq!(pr.0[0], 0);
            assert_eq!(pr.0[1], s as PatternType);
            assert_eq!(pr.0[2], (N - s) as PatternType);
        }
    }

    #[test]
    fn white_black() {
        for s in 0..N {
            let mut t_in: Vec<PatternType> = vec![0xff; N];
            t_in[..s].copy_from_slice(&vec![0; s]);
            let mut pr = PatternRow::default();
            fill_pattern_row(&t_in, &mut pr);

            assert_eq!(pr.0.len(), 3);
            assert_eq!(pr.0[0], s as PatternType);
            assert_eq!(pr.0[1], (N - s) as PatternType);
            assert_eq!(pr.0[2], 0);
        }
    }

    #[test]
    fn is_pattern_returns_zero_for_short_view() {
        let p_row = PatternRow::new(vec![1, 2]);
        let view = PatternView::new(&p_row);
        let pattern = FixedPattern::<3, 3, false>::new([1, 1, 1]);

        assert_eq!(
            is_pattern::<false, 3, 3, false>(&view, &pattern, None, 0.0, 0.0),
            0.0
        );
    }

    #[test]
    fn basic_pattern_view() {
        let mut p_row = PatternRow::default();
        fill_pattern_row(
            &[
                0_u16, 1, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1,
            ],
            &mut p_row,
        );
        // p_row layout (11 elements): [leading_whitespace, bar, space, bar,
        // space, bar, space, bar, space, bar, trailing_zero]
        //                            = [1,                  1,   1,     1,
        //                               2,     3,   2,     6,   4,     1, 0]
        // PatternView::new sets start = 1, so view[i] = data[1 + i].

        let mut pv = PatternView::new(&p_row);

        assert_eq!(pv.data(), p_row.0.as_slice());

        // view[0] is the first bar (1) — the leading-whitespace element at
        // data[0] is no longer reachable via positive indexing; it sits one
        // slot "before" view[0] and is read via has_quiet_zone_before /
        // try_index(-1).
        assert_eq!(pv[0], 1_u16); // data[1]: first bar
        assert_eq!(pv[1], 1_u16); // data[2]: first space
        assert_eq!(pv[4], 3_u16); // data[5]: third bar
        assert_eq!(pv[7], 4_u16); // data[8]: fourth space

        assert_eq!(pv.index(), 0);
        assert!(pv.shift(1));
        assert_eq!(pv.index(), 1);
        assert!(pv.skip_pair());
        assert_eq!(pv.index(), 3);
    }
}
