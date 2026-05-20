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

use std::{fmt, sync::Arc};

use unicode_segmentation::UnicodeSegmentation;

use crate::Error;
use anyhow::Result;

use super::{CharacterSet, ECIEncoderSet, ECIInput, Eci};

//* approximated (latch + 2 codewords)
pub const COST_PER_ECI: usize = 3;
const FNC1: u16 = 1000;

/**
 * Class that converts a character string into a sequence of ECIs and bytes
 *
 * The implementation uses the Dijkstra algorithm to produce minimal encodings
 *
 * @author Alex Geller
 */
pub struct MinimalECIInput {
    bytes: Vec<u16>,
    fnc1: u16,
}

impl ECIInput for MinimalECIInput {
    /**
     * Returns the length of this input.  The length is the number
     * of {@code byte}s, FNC1 characters or ECIs in the sequence.
     *
     * @return  the number of {@code char}s in this sequence
     */
    fn length(&self) -> usize {
        self.bytes.len()
    }

    /**
     * Returns the {@code byte} value at the specified index.  An index ranges from zero
     * to {@code length() - 1}.  The first {@code byte} value of the sequence is at
     * index zero, the next at index one, and so on, as for array
     * indexing.
     *
     * @param   index the index of the {@code byte} value to be returned
     *
     * @return  the specified {@code byte} value as character or the FNC1 character
     *
     * Returns an out-of-bounds error
     *          if the {@code index} argument is negative or not less than
     *          {@code length()}
     * Returns an invalid-argument error
     *          if the value at the {@code index} argument is an ECI (@see #is_eci)
     */
    fn char_at(&self, index: usize) -> Result<char> {
        if index >= self.length() {
            return Err(Error::out_of_bounds(index.to_string()).into());
        }
        if self.is_fnc1(index)? {
            Ok(self.fnc1 as u8 as char)
        } else if self.is_eci(index)? {
            Err(Error::invalid_argument(format!(
                "value at {index} is not a character but an ECI"
            )).into())
        } else {
            Ok(self.bytes[index] as u8 as char)
        }
    }

    /**
     * Returns a {@code CharSequence} that is a subsequence of this sequence.
     * The subsequence starts with the {@code char} value at the specified index and
     * ends with the {@code char} value at index {@code end - 1}.  The length
     * (in {@code char}s) of the
     * returned sequence is {@code end - start}, so if {@code start == end}
     * then an empty sequence is returned.
     *
     * @param   start   the start index, inclusive
     * @param   end     the end index, exclusive
     *
     * @return  the specified subsequence
     *
     * Returns an out-of-bounds error
     *          if {@code start} or {@code end} are negative,
     *          if {@code end} is greater than {@code length()},
     *          or if {@code start} is greater than {@code end}
     * Returns an invalid-argument error
     *          if a value in the range {@code start}-{@code end} is an ECI (@see #is_eci)
     */
    fn sub_sequence(&self, start: usize, end: usize) -> Result<Vec<char>> {
        if start > end || end > self.length() {
            return Err(Error::OutOfBounds.into());
        }
        let mut result = Vec::with_capacity(end - start);
        for i in start..end {
            if self.is_eci(i)? {
                return Err(Error::invalid_argument(format!(
                    "value at {i} is not a character but an ECI"
                )).into());
            }
            result.push(self.char_at(i)?);
        }
        Ok(result)
    }

    /**
     * Determines if a value is an ECI
     *
     * @param   index the index of the value
     *
     * @return  true if the value at position {@code index} is an ECI
     *
     * Returns an out-of-bounds error
     *          if the {@code index} argument is negative or not less than
     *          {@code length()}
     */
    fn is_eci(&self, index: usize) -> Result<bool> {
        if index >= self.length() {
            return Err(Error::OutOfBounds.into());
        }
        Ok(self.bytes[index] > 255 && self.bytes[index] != FNC1)
    }

    /**
     * Returns the {@code int} ECI value at the specified index.  An index ranges from zero
     * to {@code length() - 1}.  The first {@code byte} value of the sequence is at
     * index zero, the next at index one, and so on, as for array
     * indexing.
     *
     * @param   index the index of the {@code int} value to be returned
     *
     * @return  the specified {@code int} ECI value.
     *          The ECI specified the encoding of all bytes with a higher index until the
     *          next ECI or until the end of the input if no other ECI follows.
     *
     * Returns an out-of-bounds error
     *          if the {@code index} argument is negative or not less than
     *          {@code length()}
     * Returns an invalid-argument error
     *          if the value at the {@code index} argument is not an ECI (@see #is_eci)
     */
    fn get_ecivalue(&self, index: usize) -> Result<Eci> {
        if index >= self.length() {
            return Err(Error::OutOfBounds.into());
        }
        if !self.is_eci(index)? {
            return Err(Error::invalid_argument(format!(
                "value at {index} is not an ECI but a character"
            )).into());
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
    fn fnc1_value(fnc1: Option<&str>) -> Result<Option<u16>> {
        let Some(fnc1) = fnc1 else {
            return Ok(None);
        };
        let mut chars = fnc1.chars();
        let Some(ch) = chars.next() else {
            return Err(Error::invalid_argument(
                "fnc1 marker cannot be empty",
            ).into());
        };
        if chars.next().is_some() {
            return Err(Error::invalid_argument(
                "fnc1 marker must be a single character",
            ).into());
        }
        if (ch as u32) > u16::MAX as u32 {
            return Err(Error::invalid_argument(
                "fnc1 marker must fit in u16",
            ).into());
        }
        Ok(Some(ch as u16))
    }

    fn first_char(value: &str) -> Result<char> {
        value
            .chars()
            .next()
            .ok_or_else(|| Error::invalid_argument("empty character segment").into())
    }

    /**
     * Constructs a minimal input
     *
     * @param string_to_encode the character string to encode
     * @param priority_charset The preferred {@link Charset}. When the value of the argument is null, the algorithm
     *   chooses charsets that leads to a minimal representation. Otherwise the algorithm will use the priority
     *   charset to encode any character in the input that can be encoded by it if the charset is among the
     *   supported charsets.
     * @param fnc1 denotes the character in the input that represents the FNC1 character or -1 if this is not GS1
     *   input.
     */
    pub fn new(
        string_to_encode_input: &str,
        priority_charset: Option<CharacterSet>,
        fnc1: Option<&str>,
    ) -> Result<Self> {
        let fnc1_value = Self::fnc1_value(fnc1)?;
        let string_to_encode = string_to_encode_input.graphemes(true).collect::<Vec<&str>>();
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

    /**
     * Determines if a value is the FNC1 character
     *
     * @param   index the index of the value
     *
     * @return  true if the value at position {@code index} is the FNC1 character
     *
     * Returns an out-of-bounds error
     *          if the {@code index} argument is negative or not less than
     *          {@code length()}
     */
    pub fn is_fnc1(&self, index: usize) -> Result<bool> {
        if index >= self.length() {
            return Err(Error::OutOfBounds.into());
        }
        Ok(self.bytes[index] == FNC1)
    }

    fn add_edge(
        edges: &mut [Vec<Option<Arc<InputEdge>>>],
        to: usize,
        edge: Arc<InputEdge>,
    ) -> Result<()> {
        let slot = edges
            .get_mut(to)
            .and_then(|row| row.get_mut(edge.encoder_index))
            .ok_or_else(|| Error::out_of_bounds(to.to_string()))?;
        let should_replace = match slot {
            Some(existing) => existing.cached_total_size > edge.cached_total_size,
            None => true,
        };
        if should_replace {
            *slot = Some(edge);
        }
        Ok(())
    }

    fn add_edges(
        string_to_encode: &[&str],
        encoder_set: &ECIEncoderSet,
        edges: &mut [Vec<Option<Arc<InputEdge>>>],
        from: usize,
        previous: Option<Arc<InputEdge>>,
        fnc1: Option<&str>,
    ) -> Result<()> {
        let ch = string_to_encode
            .get(from)
            .copied()
            .ok_or_else(|| Error::out_of_bounds(from.to_string()))?;

        let mut start = 0;
        let mut end = encoder_set.len();
        if let Some(pei) = encoder_set.get_priority_encoder_index()
            && (fnc1.is_some_and(|fnc1| ch == fnc1) || encoder_set.can_encode(ch, pei)?)
        {
            start = pei;
            end = start + 1;
        }

        for i in start..end {
            if fnc1.is_some_and(|fnc1| ch == fnc1) || encoder_set.can_encode(ch, i)?
            {
                let edge = InputEdge::new(ch, encoder_set, i, previous.clone(), fnc1)?;
                Self::add_edge(
                    edges,
                    from + 1,
                    Arc::new(edge),
                )?;
            }
        }
        Ok(())
    }

    /// Minimially encode a string with the given characterset.
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
        Self::add_edges(string_to_encode, encoder_set, &mut edges, 0, None, fnc1)?;

        for i in 1..=input_length {
            for j in 0..encoder_set.len() {
                if edges[i][j].is_some() && i < input_length {
                    let edg = edges[i][j].clone();
                    Self::add_edges(string_to_encode, encoder_set, &mut edges, i, edg, fnc1)?;
                }
            }
            //optimize memory by removing edges that have been passed.
            edges[i - 1][..encoder_set.len()].fill(None);
        }
        let mut minimal_j: i32 = -1;
        let mut minimal_size: i32 = i32::MAX;
        for (j, slot) in edges[input_length].iter().enumerate().take(encoder_set.len()) {
            if let Some(edge) = slot
                && (edge.cached_total_size as i32) < minimal_size
            {
                minimal_size = edge.cached_total_size as i32;
                minimal_j = j as i32;
            }
        }
        if minimal_j < 0 {
            return Err(Error::invalid_state(format!(
                "internal error: failed to encode \"{}\"",
                string_to_encode.join("")
            )).into());
        }
        let mut ints_al: Vec<u16> = Vec::new();
        let mut current = edges[input_length][minimal_j as usize].clone();
        while let Some(c) = current {
            if c.is_fnc1() {
                ints_al.push(1000);
            } else {
                let encoded = encoder_set.encode_char(&c.c, c.encoder_index)?;
                encoded.iter().rev().for_each(|&byte| {
                    ints_al.push(byte as u16);
                });
            }
            let previous_encoder_index = if let Some(prev) = &c.previous {
                prev.encoder_index
            } else {
                0
            };

            if previous_encoder_index != c.encoder_index {
                ints_al.push(256_u16 + encoder_set.get_eci(c.encoder_index)? as u16);
            }
            current = c.previous.clone();
        }

        ints_al.reverse();
        Ok(ints_al)
    }
}

struct InputEdge {
    c: String,
    encoder_index: usize, //the encoding of this edge
    previous: Option<Arc<InputEdge>>,
    cached_total_size: usize,
}
impl InputEdge {
    const FNC1_UNICODE: &'static str = "\u{1000}";

    pub fn new(
        c: &str,
        encoder_set: &ECIEncoderSet,
        encoder_index: usize,
        previous: Option<Arc<InputEdge>>,
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
                .map_err(|e| {
                    Error::invalid_argument(format!(
                        "failed to encode \"{c}\" with encoder index {encoder_index}: {e}"
                    ))
                })?
                .len()
        };

        if let Some(prev) = previous {
            let previous_encoder_index = prev.encoder_index;
            if previous_encoder_index != encoder_index {
                size += COST_PER_ECI;
            }
            size += prev.cached_total_size;

            Ok(Self {
                c: String::from(c),
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
                c: String::from(c),
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
