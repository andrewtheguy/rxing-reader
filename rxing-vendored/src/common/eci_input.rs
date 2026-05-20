/*
 * Copyright 2021 ZXing authors
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

use std::fmt::Display;

use anyhow::Result;

use super::Eci;

/// Interface to navigate a sequence of ECIs and bytes.
pub trait ECIInput: Display {
    /// Returns the length of this input. The length is the number of bytes or
    /// ECIs in the sequence.
    fn length(&self) -> usize;

    /// Returns the character at the specified index or the FNC1 character.
    ///
    /// Returns an invalid-argument error if `index >= length()` or if the value
    /// at `index` is an ECI.
    fn char_at(&self, index: usize) -> Result<char>;

    /// Returns the subsequence in the half-open range `start..end`.
    ///
    /// Returns an invalid-argument error if `end > length()`, `start > end`, or
    /// any value in the range is an ECI.
    fn sub_sequence(&self, start: usize, end: usize) -> Result<Vec<char>>;

    /// Returns `true` if the value at `index` is an ECI.
    ///
    /// Returns an invalid-argument error if `index >= length()`.
    fn is_eci(&self, index: usize) -> Result<bool>;

    /// Returns the ECI value at the specified index.
    ///
    /// Returns an invalid-argument error if `index >= length()` or if the value
    /// at `index` is not an ECI.
    fn get_ecivalue(&self, index: usize) -> Result<Eci>;

    fn have_ncharacters(&self, index: usize, n: usize) -> Result<bool>;
}