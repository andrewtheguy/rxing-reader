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

use unicode_segmentation::UnicodeSegmentation;

use crate::Error;
use anyhow::Result;

use super::{CharacterSet, Eci};

const ENCODERS: [CharacterSet; 14] = [
    CharacterSet::Cp437,
    CharacterSet::ISO8859_2,
    CharacterSet::ISO8859_3,
    CharacterSet::ISO8859_4,
    CharacterSet::ISO8859_5,
    CharacterSet::ISO8859_7,
    CharacterSet::ISO8859_9,
    CharacterSet::ISO8859_15,
    CharacterSet::ISO8859_16,
    CharacterSet::ShiftJis,
    CharacterSet::Cp1250,
    CharacterSet::Cp1251,
    CharacterSet::Cp1252,
    CharacterSet::Cp1256,
];

/**
 * Set of CharsetEncoders for a given input string
 *
 * Invariants:
 * - The list contains only encoders from CharacterSetECI (list is shorter then the list of encoders available on
 *   the platform for which ECI values are defined).
 * - The list contains encoders at least one encoder for every character in the input.
 * - The first encoder in the list is always the ISO-8859-1 encoder even of no character in the input can be encoded
 *   by it.
 * - If the input contains a character that is not in ISO-8859-1 then the last two entries in the list will be the
 *   UTF-8 encoder and the UTF-16BE encoder.
 *
 * @author Alex Geller
 */
#[derive(Clone)]
pub struct ECIEncoderSet {
    encoders: Vec<CharacterSet>,
    priority_encoder_index: Option<usize>,
}

impl ECIEncoderSet {
    /**
     * Constructs an encoder set
     *
     * @param string_to_encode the string that needs to be encoded
     * @param priority_charset The preferred {@link Charset} or null.
     * @param fnc1 fnc1 denotes the character in the input that represents the FNC1 character or -1 for a non-GS1 bar
     * code. When specified, it is considered an error to pass it as argument to the methods can_encode() or encode().
     */
    pub fn new(
        string_to_encode_main: &str,
        priority_charset: Option<CharacterSet>,
        fnc1: Option<&str>,
    ) -> Self {
        // List of encoders that potentially encode characters not in ISO-8859-1 in one byte.

        let mut encoders: Vec<CharacterSet>;
        let mut priority_encoder_index_value = None;

        let mut needed_encoders: Vec<CharacterSet> = Vec::new();

        let string_to_encode = string_to_encode_main.graphemes(true).collect::<Vec<&str>>();

        //we always need the ISO-8859-1 encoder. It is the default encoding
        needed_encoders.push(CharacterSet::ISO8859_1);
        let mut need_unicode_encoder = if let Some(pc) = priority_charset {
            pc == CharacterSet::UTF8 || pc == CharacterSet::UTF16BE
        } else {
            false
        };

        //Walk over the input string and see if all characters can be encoded with the list of encoders
        for c in &string_to_encode {
            let mut can_encode = false;
            for encoder in &needed_encoders {
                if fnc1.is_some_and(|fnc1| *c == fnc1) || encoder.encode(c).is_ok() {
                    can_encode = true;
                    break;
                }
            }
            if !can_encode {
                //for the character at position i we don't yet have an encoder in the list
                for encoder in ENCODERS.iter() {
                    if encoder.encode(c).is_ok() {
                        //Good, we found an encoder that can encode the character. We add him to the list and continue scanning
                        //the input
                        needed_encoders.push(*encoder);
                        can_encode = true;
                        break;
                    }
                }
            }

            if !can_encode {
                //The character is not encodeable by any of the single byte encoders so we remember that we will need a
                //Unicode encoder.
                need_unicode_encoder = true;
            }
        }

        if needed_encoders.len() == 1 && !need_unicode_encoder {
            //the entire input can be encoded by the ISO-8859-1 encoder
            encoders = vec![CharacterSet::ISO8859_1];
        } else {
            // we need more than one single byte encoder or we need a Unicode encoder.
            // In this case we append a UTF-8 and UTF-16 encoder to the list
            encoders = Vec::with_capacity(needed_encoders.len() + 2);

            encoders.extend(needed_encoders);

            encoders.push(CharacterSet::UTF8);
            encoders.push(CharacterSet::UTF16BE);
        }

        //Compute priority_encoder_index by looking up priority_charset in encoders
        if let Some(pc) = priority_charset.as_ref() {
            priority_encoder_index_value = encoders.iter().position(|enc| enc == pc);
        }
        Self {
            encoders,
            priority_encoder_index: priority_encoder_index_value,
        }
    }

    pub fn len(&self) -> usize {
        self.encoders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.encoders.is_empty()
    }

    pub fn get_charset_name(&self, index: usize) -> Result<&'static str> {
        Ok(self.get_charset(index)?.get_charset_name())
    }

    pub fn get_charset(&self, index: usize) -> Result<CharacterSet> {
        self.encoders.get(index).copied().ok_or_else(|| {
            Error::InvalidArgument {
                message: format!(
                    "encoder index {index} out of range for {} encoders",
                    self.encoders.len()
                ),
            }
            .into()
        })
    }

    pub fn get_eci(&self, encoder_index: usize) -> Result<Eci> {
        let eci = Eci::from(self.get_charset(encoder_index)?);
        if eci == Eci::Unknown {
            return Err(Error::InvalidState {
                message: format!("no ECI assignment for encoder index {encoder_index}"),
            }
            .into());
        }
        Ok(eci)
    }

    /*
     *  returns -1 if no priority charset was defined
     */
    pub const fn get_priority_encoder_index(&self) -> Option<usize> {
        self.priority_encoder_index
    }

    pub fn can_encode(&self, c: &str, encoder_index: usize) -> Result<bool> {
        let encoder = self.get_charset(encoder_index)?;
        let enc_data = encoder.encode(c);

        Ok(enc_data.is_ok())
    }

    pub fn encode_char(&self, c: &str, encoder_index: usize) -> Result<Vec<u8>> {
        let encoder = self.get_charset(encoder_index)?;
        encoder.encode(c)
    }

    pub fn encode_string(&self, s: &str, encoder_index: usize) -> Result<Vec<u8>> {
        let encoder = self.get_charset(encoder_index)?;
        encoder.encode(s)
    }
}
