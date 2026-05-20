/*
* Copyright 2020 Axel Waggershauser
*/
// SPDX-License-Identifier: Apache-2.0

use crate::{
    Exceptions,
    common::{BitMatrix, Result},
};

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

    pub fn into_pattern_view(&'_ self) -> PatternView<'_> {
        PatternView::new(self)
    }

    pub fn sum(&self) -> PatternType {
        self.0.iter().sum()
    }

    pub fn rev(&mut self) {
        self.0.reverse()
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
            *self.pattern_view.data.0.get(
                self.current_position - 1 + self.pattern_view.start + self.pattern_view.current,
            )?,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PatternView<'a> {
    data: &'a PatternRow,
    start: usize,
    count: usize,
    current: usize,
}

impl<'a> PatternView<'a> {
    // A PatternRow always starts with the width of whitespace in front of the first black bar.
    // The first element of the PatternView is the first bar.
    pub fn new(bars: &'a PatternRow) -> PatternView<'a> {
        PatternView {
            data: bars,
            start: 1,
            count: bars.0.len(),
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
            data: bars,
            start,
            count: size,
            current: base,
        }
    }

    pub fn data(&self) -> &PatternRow {
        self.data
    }
    pub fn begin(&self) -> Option<PatternType> {
        Some(*self.data.0.get(self.start)?)
    }
    pub fn end(&self) -> Option<PatternType> {
        Some(self.data.0.len() as PatternType)
    }

    pub fn sum(&self, n: Option<usize>) -> PatternType {
        if self.count == self.data.len() {
            return self.data.0.iter().sum::<PatternType>();
        }

        let n = n.unwrap_or(self.count);

        self.data
            .0
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
            .0
            .iter()
            .take(self.start + self.current)
            .copied()
            .sum::<PatternType>()
    }
    pub fn pixels_till_end(&self) -> PatternType {
        self.data
            .0
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
        !self.data.0.is_empty()
            && self.start <= self.current + self.start
            && self.current + n < (self.data.0.len())
    }
    pub fn is_valid(&self) -> bool {
        self.is_valid_with_n(self.size())
    }

    pub fn has_quiet_zone_before(&self, scale: f32, accept_if_at_first_bar: Option<bool>) -> bool {
        if accept_if_at_first_bar.unwrap_or(false) && self.is_at_first_bar() {
            return true;
        }
        let prev_idx = (self.start + self.current).checked_sub(1);
        match prev_idx.and_then(|i| self.data.0.get(i)) {
            Some(v) => {
                Into::<f32>::into(*v) >= Into::<f32>::into(self.sum(None)) * scale
            }
            None => false,
        }
    }

    pub fn has_quiet_zone_after(&self, scale: f32, accept_if_at_last_bar: Option<bool>) -> bool {
        if accept_if_at_last_bar.unwrap_or(true) && self.is_at_last_bar() {
            return true;
        }
        match self.data.0.get(self.start + self.current + self.count) {
            Some(v) => {
                Into::<f32>::into(*v) >= Into::<f32>::into(self.sum(None)) * scale
            }
            None => false,
        }
    }

    pub fn sub_view(&self, offset: usize, size: Option<usize>) -> PatternView<'a> {
        let mut size = size.unwrap_or(0);
        if size == 0 {
            size = self.count - offset;
        }

        PatternView {
            data: self.data,
            start: self.start + offset,
            count: size,
            current: self.current,
        }
    }

    pub fn shift(&mut self, n: usize) -> bool {
        self.current += n;
        !self.data.0.is_empty()
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

    fn try_get_index(&self, index: isize) -> Option<PatternType> {
        if index.abs() > self.data.0.len() as isize {
            return None;
        }
        if index >= 0 {
            let fetch_spot = ((self.start + self.current) as isize + index) as usize;
            return self.data.0.get(fetch_spot).copied();
        }
        if index.abs() > (self.start + self.current) as isize {
            return None;
        }
        let fetch_spot = ((self.start + self.current) as isize + index) as usize;
        self.data.0.get(fetch_spot).copied()
    }
}

impl std::ops::Index<isize> for PatternView<'_> {
    type Output = PatternType;

    fn index(&self, index: isize) -> &Self::Output {
        if self.count == self.data.len() && index >= 0 {
            return &self.data[index as usize];
        }
        if self.try_get_index(index).is_none() {
            panic!(
                "index out of bounds: the len is {} but the index is {}",
                self.count, index
            )
        }
        let fetch_spot = ((self.start + self.current) as isize + index) as usize;
        &self.data.0[fetch_spot]
    }
}

impl std::ops::Index<usize> for PatternView<'_> {
    type Output = PatternType;

    fn index(&self, index: usize) -> &Self::Output {
        if self.count == self.data.len() {
            return &self.data[index];
        }

        match self.data.0.get(self.start + self.current + index) {
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
        if value.data.0.len() == value.count {
            &value.data.0
        } else {
            let start_idx = value.current + value.start;
            let end_idx = (start_idx + value.count).min(value.data.0.len());
            &value.data.0[start_idx..end_idx]
        }
    }
}

/**
 * @brief The BarAndSpace struct is a simple 2 element data structure to hold information about bar(s) and space(s).
 *
 * The operator[](int) can be used in combination with a PatternView
 */
#[derive(Default, Clone)]
pub struct BarAndSpace<T: Default + std::cmp::PartialEq> {
    bar: T,
    space: T,
}
impl<T: Default + std::cmp::PartialEq> BarAndSpace<T> {
    pub fn new(bar: T, space: T) -> BarAndSpace<T> {
        Self { bar, space }
    }

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

/**
 * @brief FixedPattern describes a compile-time constant (start/stop) pattern.
 *
 * @param N  number of bars/spaces
 * @param SUM  sum over all N elements (size of pattern in modules)
 * @param IS_SPARCE  whether or not the pattern contains '0's denoting 'wide' bars/spaces
 */
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

pub type FixedSparcePattern<const N: usize, const SUM: usize> = FixedPattern<N, SUM, true>;

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

    let width = view.sum(Some(LEN));
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
        if (Into::<f32>::into(view_data[x])
            - Into::<f32>::into(pattern_data[x]) * module_size_ref)
            .abs()
            > threshold
        {
            return 0.0;
        }
    }

    module_size
}

pub fn is_right_guard<const N: usize, const SUM: usize, const IS_SPARCE: bool>(
    view: &PatternView,
    pattern: &FixedPattern<N, SUM, IS_SPARCE>,
    min_quiet_zone: f32,
    module_size_ref: f32,
) -> bool {
    let space_in_pixel = if view.is_at_last_bar() {
        None
    } else {
        view.end().map(Into::into)
    };

    const E2E: bool = false;

    is_pattern::<E2E, N, SUM, IS_SPARCE>(
        view,
        pattern,
        space_in_pixel,
        min_quiet_zone,
        module_size_ref,
    ) != 0.0
}

pub fn find_left_guard_by<const LEN: usize, Pred: Fn(&PatternView, Option<f32>) -> bool>(
    view: PatternView<'_>,
    min_size: usize,
    is_guard: Pred,
) -> Result<PatternView<'_>> {
    const PREV_IDX: isize = -1;

    if view.size() < min_size {
        return Err(Exceptions::ILLEGAL_STATE);
    }

    let mut window = view.sub_view(0, Some(LEN));
    if window.is_at_first_bar() && is_guard(&window, Some(f32::MAX)) {
        return Ok(window);
    }
    let end = Into::<usize>::into(view.end().ok_or(Exceptions::INDEX_OUT_OF_BOUNDS)?) - min_size;
    while (window.start + window.current) < end {
        let prev = window.try_get_index(PREV_IDX).map(|v| v as f32);
        if is_guard(&window, prev) {
            return Ok(window);
        }

        window.skip_pair();
    }

    Err(Exceptions::ILLEGAL_STATE)
}

pub fn find_left_guard<'a, const LEN: usize, const SUM: usize, const IS_SPARCE: bool>(
    view: PatternView<'a>,
    min_size: usize,
    pattern: &FixedPattern<LEN, SUM, IS_SPARCE>,
    min_quiet_zone: f32,
) -> Result<PatternView<'a>> {
    find_left_guard_by::<LEN, _>(view, std::cmp::max(min_size, LEN), |window, space_in_pixel| {
        // perform a fast plausability test for 1:1:3:1:1 pattern
        if window[2] < 2 as PatternType * std::cmp::max(window[0], window[4])
            || window[2] < std::cmp::max(window[1], window[3])
        {
            return false;
        }
        is_pattern::<false, LEN, SUM, IS_SPARCE>(window, pattern, space_in_pixel, min_quiet_zone, 0.0)
            != 0.0
    })
}

pub fn normalized_e2_epattern<const LEN: usize, const LEN_MINUS_2: usize, const SUM: usize>(
    view: &PatternView,
) -> [PatternType; LEN_MINUS_2] {
    let module_size: f32 = Into::<f32>::into(view.sum(Some(LEN))) / SUM as f32;

    let mut e2e = [PatternType::default(); LEN_MINUS_2];

    for i in 0..LEN_MINUS_2 {
        let v: f32 = (Into::<f32>::into(view[i]) + Into::<f32>::into(view[i + 1])) / module_size;
        e2e[i] = (v + 0.5) as PatternType;
    }

    e2e
}

pub fn normalized_pattern<const LEN: usize, const SUM: usize>(
    view: &PatternView,
) -> Result<[PatternType; LEN]> {
    let module_size: f32 = Into::<usize>::into(view.sum(Some(LEN))) as f32 / SUM as f32;
    if !module_size.is_finite() || module_size <= f32::EPSILON {
        return Err(Exceptions::NOT_FOUND);
    }
    let mut err = SUM as isize;
    let mut is = [PatternType::default(); LEN];
    let mut rs = [0.0; LEN];
    for i in 0..LEN {
        let v: f32 = Into::<f32>::into(view[i]) / module_size;
        is[i] = (v + 0.5) as PatternType;
        rs[i] = v - Into::<f32>::into(is[i]);
        err -= Into::<usize>::into(is[i]) as isize;
    }

    if err.abs() > 1 {
        return Err(Exceptions::NOT_FOUND);
    }

    if err != 0 {
        let mi = if err > 0 {
            rs.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        } else {
            rs.iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        };
        let (mi, _) = mi.ok_or(Exceptions::ILLEGAL_STATE)?;
        is[mi] += err as PatternType;
    }

    Ok(is)
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

pub fn get_pattern_row_tp(matrix: &BitMatrix, r: u32, pr: &mut PatternRow, transpose: bool) {
    let row = if transpose {
        matrix.get_col(r)
    } else {
        matrix.get_row(r)
    };

    let pixel_states: Vec<bool> = row.into();

    get_pattern_row(&pixel_states, pr)
}

pub fn get_pattern_row<T: Into<PatternType> + Copy + Default + From<T>>(
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
    use crate::common::cpp_essentials::PatternType;

    use super::{FixedPattern, get_pattern_row, is_pattern, PatternRow, PatternView};
    const N: usize = 33;

    #[test]
    fn all_white() {
        for s in 1..=N {
            let t_in: Vec<PatternType> = vec![0; s];
            let mut pr = PatternRow::default();
            get_pattern_row(&t_in, &mut pr);

            assert_eq!(pr.0.len(), 1);
            assert_eq!(pr.0[0], s as PatternType);
        }
    }

    #[test]
    fn all_black() {
        for s in 1..=N {
            let t_in: Vec<PatternType> = vec![0xff; s];
            let mut pr = PatternRow::default();
            get_pattern_row(&t_in, &mut pr);

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
            get_pattern_row(&t_in, &mut pr);

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
            get_pattern_row(&t_in, &mut pr);

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
        get_pattern_row(
            &[
                0_u16, 1, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1,
            ],
            &mut p_row,
        );

        let mut pv = PatternView::new(&p_row);

        assert_eq!(pv.data().0, p_row.0);

        assert_eq!(pv[0], 1_u16);
        assert_eq!(pv[1], 1_u16);
        assert_eq!(pv[4], 2_u16);
        assert_eq!(pv[7], 6_u16);

        assert_eq!(pv.index(), 0);
        assert!(pv.shift(1));
        assert_eq!(pv.index(), 1);
        assert!(pv.skip_pair());
        assert_eq!(pv.index(), 3);
    }
}
