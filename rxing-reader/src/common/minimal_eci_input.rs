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

use std::fmt;

use unicode_segmentation::UnicodeSegmentation;

use crate::Error;
use anyhow::Result;

use super::{CharacterSet, ECIEncoderSet, ECIInput, Eci};

//* approximated (latch + 2 codewords)
pub const COST_PER_ECI: usize = 3;
const FNC1: u16 = 1000;

/// Converts a character string into a sequence of ECIs and bytes.
///
/// The implementation uses the Dijkstra algorithm to produce minimal encodings.
pub struct MinimalECIInput {
    bytes: Vec<u16>,
    fnc1: u16,
}

impl ECIInput for MinimalECIInput {
    fn length(&self) -> usize {
        self.bytes.len()
    }

    fn char_at(&self, index: usize) -> Result<char> {
        if index >= self.length() {
            return Err(self.invalid_index(index).into());
        }
        if self.is_fnc1(index)? {
            Ok(self.fnc1 as u8 as char)
        } else if self.is_eci(index)? {
            Err(Error::InvalidArgument {
                message: format!("value at {index} is not a character but an ECI"),
            }
            .into())
        } else {
            Ok(self.bytes[index] as u8 as char)
        }
    }

    fn sub_sequence(&self, start: usize, end: usize) -> Result<Vec<char>> {
        if start > end || end > self.length() {
            return Err(self.invalid_range(start, end).into());
        }
        let mut result = Vec::with_capacity(end - start);
        for i in start..end {
            if self.is_eci(i)? {
                return Err(Error::InvalidArgument {
                    message: format!("value at {i} is not a character but an ECI"),
                }
                .into());
            }
            result.push(self.char_at(i)?);
        }
        Ok(result)
    }

    fn is_eci(&self, index: usize) -> Result<bool> {
        if index >= self.length() {
            return Err(self.invalid_index(index).into());
        }
        Ok(self.bytes[index] > 255 && self.bytes[index] != FNC1)
    }

    fn get_ecivalue(&self, index: usize) -> Result<Eci> {
        if index >= self.length() {
            return Err(self.invalid_index(index).into());
        }
        if !self.is_eci(index)? {
            return Err(Error::InvalidArgument {
                message: format!("value at {index} is not an ECI but a character"),
            }
            .into());
        }
        Eci::try_from(self.bytes[index] as u32 - 256)
    }

    fn have_ncharacters(&self, index: usize, n: usize) -> Result<bool> {
        if index + n > self.bytes.len() {
            return Ok(false);
        }
        for i in 0..n {
            if self.is_eci(index + i)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
impl MinimalECIInput {
    fn invalid_index(&self, index: usize) -> Error {
        Error::InvalidArgument {
            message: format!(
                "index {index} out of range for input length {}",
                self.length()
            ),
        }
    }

    fn invalid_range(&self, start: usize, end: usize) -> Error {
        Error::InvalidArgument {
            message: format!(
                "range {start}..{end} is invalid for input length {}",
                self.length()
            ),
        }
    }

    fn fnc1_value(fnc1: Option<&str>) -> Result<Option<u16>> {
        let Some(fnc1) = fnc1 else {
            return Ok(None);
        };
        let mut chars = fnc1.chars();
        let Some(ch) = chars.next() else {
            return Err(Error::InvalidArgument {
                message: "fnc1 marker cannot be empty".to_owned(),
            }
            .into());
        };
        if chars.next().is_some() {
            return Err(Error::InvalidArgument {
                message: "fnc1 marker must be a single character".to_owned(),
            }
            .into());
        }
        if (ch as u32) > u16::MAX as u32 {
            return Err(Error::InvalidArgument {
                message: "fnc1 marker must fit in u16".to_owned(),
            }
            .into());
        }
        Ok(Some(ch as u16))
    }

    fn first_char(value: &str) -> Result<char> {
        value.chars().next().ok_or_else(|| {
            Error::InvalidArgument {
                message: "empty character segment".to_owned(),
            }
            .into()
        })
    }

    /// Constructs a minimal input.
    pub fn new(
        string_to_encode_input: &str,
        priority_charset: Option<CharacterSet>,
        fnc1: Option<&str>,
    ) -> Result<Self> {
        let fnc1_value = Self::fnc1_value(fnc1)?;
        let string_to_encode = string_to_encode_input
            .graphemes(true)
            .collect::<Vec<&str>>();
        let encoder_set = ECIEncoderSet::new(string_to_encode_input, priority_charset, fnc1);
        let bytes = if encoder_set.len() == 1 {
            //optimization for the case when all can be encoded without ECI in ISO-8859-1)
            string_to_encode
                .iter()
                .map(|c| {
                    if fnc1.is_some_and(|fnc1| *c == fnc1) {
                        Ok(FNC1)
                    } else {
                        Ok(Self::first_char(c)? as u16)
                    }
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            Self::encode_minimally(&string_to_encode, &encoder_set, fnc1)?
        };

        Ok(Self {
            bytes,
            fnc1: fnc1_value.unwrap_or(FNC1),
        })
    }

    pub fn get_fnc1_character(&self) -> u16 {
        self.fnc1
    }

    /// Returns `true` if the value at `index` is the FNC1 character.
    ///
    /// Returns an invalid-argument error if `index >= length()`.
    pub fn is_fnc1(&self, index: usize) -> Result<bool> {
        if index >= self.length() {
            return Err(self.invalid_index(index).into());
        }
        Ok(self.bytes[index] == FNC1)
    }

    fn add_edge<'a>(
        edges: &mut [Vec<Option<usize>>],
        edge_arena: &mut Vec<InputEdge<'a>>,
        to: usize,
        edge: InputEdge<'a>,
    ) -> Result<()> {
        let encoder_index = edge.encoder_index;
        let slot = edges
            .get_mut(to)
            .and_then(|row| row.get_mut(encoder_index))
            .ok_or_else(|| Error::InvalidState {
                message: format!("edge graph is missing slot ({to}, {encoder_index})"),
            })?;
        let should_replace = match slot {
            Some(existing) => edge_arena[*existing].cached_total_size > edge.cached_total_size,
            None => true,
        };
        if should_replace {
            edge_arena.push(edge);
            *slot = Some(edge_arena.len() - 1);
        }
        Ok(())
    }

    fn add_edges<'a>(
        string_to_encode: &[&'a str],
        encoder_set: &ECIEncoderSet,
        edges: &mut [Vec<Option<usize>>],
        edge_arena: &mut Vec<InputEdge<'a>>,
        from: usize,
        previous: Option<usize>,
        fnc1: Option<&str>,
    ) -> Result<()> {
        let ch = string_to_encode
            .get(from)
            .copied()
            .ok_or_else(|| Error::InvalidState {
                message: format!(
                    "character position {from} is outside input of length {}",
                    string_to_encode.len()
                ),
            })?;

        let mut start = 0;
        let mut end = encoder_set.len();
        if let Some(pei) = encoder_set.get_priority_encoder_index()
            && (fnc1.is_some_and(|fnc1| ch == fnc1) || encoder_set.can_encode(ch, pei)?)
        {
            start = pei;
            end = start + 1;
        }

        for i in start..end {
            if fnc1.is_some_and(|fnc1| ch == fnc1) || encoder_set.can_encode(ch, i)? {
                let edge = InputEdge::new(ch, encoder_set, i, previous, edge_arena, fnc1)?;
                Self::add_edge(edges, edge_arena, from + 1, edge)?;
            }
        }
        Ok(())
    }

    /// Finds the lowest-cost sequence of bytes and ECI designators for the input graphemes.
    ///
    /// Returns an error if the string cannot be encoded.
    pub fn encode_minimally(
        string_to_encode: &[&str],
        encoder_set: &ECIEncoderSet,
        fnc1: Option<&str>,
    ) -> Result<Vec<u16>> {
        let input_length = string_to_encode.len();

        // Array that represents vertices. There is a vertex for every character and encoding.
        let mut edges = vec![vec![None; encoder_set.len()]; input_length + 1];
        let mut edge_arena = Vec::new();
        Self::add_edges(
            string_to_encode,
            encoder_set,
            &mut edges,
            &mut edge_arena,
            0,
            None,
            fnc1,
        )?;

        for i in 1..=input_length {
            for j in 0..encoder_set.len() {
                if i < input_length
                    && let Some(edge_index) = edges[i][j]
                {
                    Self::add_edges(
                        string_to_encode,
                        encoder_set,
                        &mut edges,
                        &mut edge_arena,
                        i,
                        Some(edge_index),
                        fnc1,
                    )?;
                }
            }
            //optimize memory by removing edges that have been passed.
            edges[i - 1][..encoder_set.len()].fill(None);
        }
        let mut minimal_j: i32 = -1;
        let mut minimal_size: i32 = i32::MAX;
        for (j, slot) in edges[input_length]
            .iter()
            .enumerate()
            .take(encoder_set.len())
        {
            if let Some(edge_index) = slot
                && (edge_arena[*edge_index].cached_total_size as i32) < minimal_size
            {
                minimal_size = edge_arena[*edge_index].cached_total_size as i32;
                minimal_j = j as i32;
            }
        }
        if minimal_j < 0 {
            return Err(Error::InvalidState {
                message: format!(
                    "internal error: failed to encode \"{}\"",
                    string_to_encode.join("")
                ),
            }
            .into());
        }
        let mut ints_al: Vec<u16> = Vec::new();
        let mut current = edges[input_length][minimal_j as usize];
        while let Some(edge_index) = current {
            let c = &edge_arena[edge_index];
            if c.is_fnc1() {
                ints_al.push(1000);
            } else {
                let encoded = encoder_set.encode_char(c.c, c.encoder_index)?;
                encoded.iter().rev().for_each(|&byte| {
                    ints_al.push(byte as u16);
                });
            }
            let previous_encoder_index = if let Some(prev) = c.previous {
                edge_arena[prev].encoder_index
            } else {
                0
            };

            if previous_encoder_index != c.encoder_index {
                ints_al.push(256_u16 + encoder_set.get_eci(c.encoder_index)? as u16);
            }
            current = c.previous;
        }

        ints_al.reverse();
        Ok(ints_al)
    }
}

struct InputEdge<'a> {
    c: &'a str,
    encoder_index: usize, //the encoding of this edge
    previous: Option<usize>,
    cached_total_size: usize,
}
impl<'a> InputEdge<'a> {
    const FNC1_UNICODE: &'static str = "\u{1000}";

    pub fn new(
        c: &'a str,
        encoder_set: &ECIEncoderSet,
        encoder_index: usize,
        previous: Option<usize>,
        edge_arena: &[InputEdge<'a>],
        fnc1: Option<&str>,
    ) -> Result<Self> {
        let c = if fnc1.is_some_and(|fnc1| c == fnc1) {
            Self::FNC1_UNICODE
        } else {
            c
        };
        let mut size = if c == Self::FNC1_UNICODE {
            1
        } else {
            encoder_set
                .encode_char(c, encoder_index)
                .map_err(|e| Error::InvalidArgument {
                    message: format!(
                        "failed to encode \"{c}\" with encoder index {encoder_index}: {e}"
                    ),
                })?
                .len()
        };

        if let Some(prev) = previous {
            let previous_encoder_index = edge_arena[prev].encoder_index;
            if previous_encoder_index != encoder_index {
                size += COST_PER_ECI;
            }
            size += edge_arena[prev].cached_total_size;

            Ok(Self {
                c,
                encoder_index,
                previous: Some(prev),
                cached_total_size: size,
            })
        } else {
            let previous_encoder_index = 0;
            if previous_encoder_index != encoder_index {
                size += COST_PER_ECI;
            }

            Ok(Self {
                c,
                encoder_index,
                previous: None,
                cached_total_size: size,
            })
        }
    }

    pub fn is_fnc1(&self) -> bool {
        self.c == Self::FNC1_UNICODE
    }
}

impl fmt::Display for MinimalECIInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = String::new();
        for i in 0..self.length() {
            if i > 0 {
                result.push_str(", ");
            }
            if self.is_eci(i).map_err(|_| fmt::Error)? {
                result.push_str("ECI(");
                result.push_str(&self.get_ecivalue(i).map_err(|_| fmt::Error)?.to_string());
                result.push(')');
            } else {
                let ch = self.char_at(i).map_err(|_| fmt::Error)?;
                if (ch as u8) < 128 {
                    result.push('\'');
                    result.push(ch);
                    result.push('\'');
                } else {
                    result.push(ch);
                }
            }
        }
        write!(f, "{result}")
    }
}
