/*
 * Copyright 2022 ZXing authors
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

use std::{
    collections::HashSet,
    fmt::{self},
};

use super::{CharacterSet, Eci, string_utils};

/// Builds decoded text from byte ranges annotated with ECI character sets.
#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub struct ECIStringBuilder {
    pub has_eci: bool,
    eci_result: Option<String>,
    bytes: Vec<u8>,
    pub(crate) eci_positions: Vec<(Eci, usize, usize)>, // (Eci, start, end)
    pub symbology: SymbologyIdentifier,
    eci_list: HashSet<Eci>,
}

impl ECIStringBuilder {
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Appends one raw byte.
    pub fn append_byte(&mut self, value: u8) {
        self.eci_result = None;
        self.bytes.push(value)
    }

    pub fn append_bytes(&mut self, value: &[u8]) {
        self.eci_result = None;
        self.bytes.extend_from_slice(value)
    }

    /// Appends the UTF-8 bytes for `value`.
    pub fn append_string(&mut self, value: &str) {
        if !value.is_ascii() {
            self.append_eci(Eci::UTF8);
        }
        self.eci_result = None;
        self.bytes.extend_from_slice(value.as_bytes());
    }

    /// Marks the current byte position with a new ECI designator.
    ///
    /// Bytes appended after this call are decoded with `eci` until another ECI
    /// designator is appended.
    pub fn append_eci(&mut self, eci: Eci) {
        self.eci_result = None;

        if !self.has_eci && eci != Eci::ISO8859_1 {
            self.has_eci = true;
        }

        if self.has_eci {
            if let Some(last) = self.eci_positions.last_mut() {
                last.2 = self.bytes.len()
            }

            self.eci_positions.push((eci, self.bytes.len(), 0));

            self.eci_list.insert(eci);

            if self.eci_list.len() == 1 && (self.eci_list.contains(&Eci::Unknown)) {
                self.has_eci = false;
                self.eci_positions.clear();
            }
        }
    }

    /// Starts a new encoding range for `charset`.
    ///
    /// When `is_eci` is `true`, the range came from an explicit ECI marker in
    /// the symbol; otherwise it is an internal decoder hint.
    pub fn switch_encoding(&mut self, charset: CharacterSet, is_eci: bool) {
        if is_eci && !self.has_eci {
            self.eci_positions.clear();
        }
        if is_eci || !self.has_eci {
            if let Some(last) = self.eci_positions.last_mut() {
                last.2 = self.bytes.len()
            }

            self.eci_positions
                .push((Eci::from(charset), self.bytes.len(), 0));
        }

        self.has_eci |= is_eci;
    }

    /// Decodes the buffered bytes using their ECI ranges.
    pub fn encode_current_bytes_if_any(&self) -> String {
        let mut encoded_string = String::with_capacity(self.bytes.len());
        // First encode the first set
        let (_eci, end, _) =
            *self
                .eci_positions
                .first()
                .unwrap_or(&(Eci::ISO8859_1, self.bytes.len(), 0));

        encoded_string.push_str(
            &Self::encode_segment(&self.bytes[0..end], Eci::ISO8859_1).unwrap_or_default(),
        );

        if end == self.bytes.len() {
            return encoded_string;
        }

        // If there are more sets, encode each of them in turn
        for (eci, eci_start, eci_end) in &self.eci_positions {
            let end = if *eci_end == 0 {
                self.bytes.len()
            } else {
                *eci_end
            };
            encoded_string.push_str(
                &Self::encode_segment(&self.bytes[*eci_start..end], *eci).unwrap_or_default(),
            );
        }

        // Return the result
        encoded_string
    }

    fn encode_segment(bytes: &[u8], eci: Eci) -> Option<String> {
        let mut not_encoded_yet = true;
        let mut encoded_string = String::with_capacity(bytes.len());
        if ![Eci::Binary, Eci::Unknown].contains(&eci) {
            if eci == Eci::UTF8 {
                if !bytes.is_empty() {
                    encoded_string.push_str(&CharacterSet::UTF8.decode(bytes).ok()?);
                    not_encoded_yet = false;
                } else {
                    return None;
                }
            } else if !bytes.is_empty() {
                encoded_string.push_str(&CharacterSet::from(eci).decode(bytes).ok()?);
                not_encoded_yet = false;
            } else {
                return None;
            }
        } else if eci == Eci::Unknown
            && let Some(found_encoding) = string_utils::guess_charset(bytes)
            && let Ok(found_encoded_str) = found_encoding.decode(bytes)
        {
            encoded_string.push_str(&found_encoded_str);
            not_encoded_yet = false;
        }

        if not_encoded_yet {
            for byte in bytes {
                encoded_string.push(char::from(*byte))
            }
        }

        if encoded_string.is_empty() {
            None
        } else {
            Some(encoded_string)
        }
    }

    /// Reserves capacity for at least `additional` more bytes.
    pub fn reserve(&mut self, additional: usize) {
        self.bytes.reserve(additional);
    }

    /// Returns `true` when no bytes have been appended.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl fmt::Display for ECIStringBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(res) = &self.eci_result {
            write!(f, "{res}")
        } else {
            write!(f, "{}", self.encode_current_bytes_if_any())
        }
    }
}

impl std::ops::AddAssign<u8> for ECIStringBuilder {
    fn add_assign(&mut self, rhs: u8) {
        self.append_byte(rhs)
    }
}

impl std::ops::AddAssign<String> for ECIStringBuilder {
    fn add_assign(&mut self, rhs: String) {
        self.append_string(&rhs)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AIFlag {
    None,
    GS1,
    Aim,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SymbologyIdentifier {
    pub code: u8,
    pub modifier: u8,
    pub eci_modifier_offset: u8,
    pub ai_flag: AIFlag,
}

impl Default for SymbologyIdentifier {
    fn default() -> Self {
        Self {
            code: 0,
            modifier: 0,
            eci_modifier_offset: 0,
            ai_flag: AIFlag::None,
        }
    }
}

impl std::io::Write for ECIStringBuilder {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() == 1 {
            self.append_byte(buf[0]);
        } else {
            self.append_bytes(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
