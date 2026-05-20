/*
 * Copyright 2008 ZXing authors
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

use crate::Error;
use anyhow::Result;

/// Encapsulates a Character Set ECI, according to "Extended Channel Interpretations" 5.3.1.1
/// of ISO 18004.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CharacterSet {
    Cp437,
    ISO8859_1,
    ISO8859_2,
    ISO8859_3,
    ISO8859_4,
    ISO8859_5,
    ISO8859_6,
    ISO8859_7,
    ISO8859_8,
    ISO8859_9,
    ISO8859_10,
    ISO8859_11,
    ISO8859_13,
    ISO8859_14,
    ISO8859_15,
    ISO8859_16,
    ShiftJis,
    Cp1250,
    Cp1251,
    Cp1252,
    Cp1256,
    UTF16BE,
    UTF8,
    Ascii,
    Big5,
    GB2312,
    GB18030,
    EucKr,
    UTF16LE,
    UTF32BE,
    UTF32LE,
    Binary,
    Unknown,
}

impl CharacterSet {
    fn encoding(&self) -> Option<&'static encoding_rs::Encoding> {
        match self {
            CharacterSet::ISO8859_2 => Some(encoding_rs::ISO_8859_2),
            CharacterSet::ISO8859_3 => Some(encoding_rs::ISO_8859_3),
            CharacterSet::ISO8859_4 => Some(encoding_rs::ISO_8859_4),
            CharacterSet::ISO8859_5 => Some(encoding_rs::ISO_8859_5),
            CharacterSet::ISO8859_6 => Some(encoding_rs::ISO_8859_6),
            CharacterSet::ISO8859_7 => Some(encoding_rs::ISO_8859_7),
            CharacterSet::ISO8859_8 => Some(encoding_rs::ISO_8859_8),
            CharacterSet::ISO8859_9 => Some(encoding_rs::WINDOWS_1254),
            CharacterSet::ISO8859_10 => Some(encoding_rs::ISO_8859_10),
            CharacterSet::ISO8859_11 => Some(encoding_rs::WINDOWS_874),
            CharacterSet::ISO8859_13 => Some(encoding_rs::ISO_8859_13),
            CharacterSet::ISO8859_14 => Some(encoding_rs::ISO_8859_14),
            CharacterSet::ISO8859_15 => Some(encoding_rs::ISO_8859_15),
            CharacterSet::ISO8859_16 => Some(encoding_rs::ISO_8859_16),
            CharacterSet::ShiftJis => Some(encoding_rs::SHIFT_JIS),
            CharacterSet::Cp1250 => Some(encoding_rs::WINDOWS_1250),
            CharacterSet::Cp1251 => Some(encoding_rs::WINDOWS_1251),
            CharacterSet::Cp1252 => Some(encoding_rs::WINDOWS_1252),
            CharacterSet::Cp1256 => Some(encoding_rs::WINDOWS_1256),
            CharacterSet::UTF8 => Some(encoding_rs::UTF_8),
            CharacterSet::Ascii => encoding_rs::Encoding::for_label(b"ascii"),
            CharacterSet::Big5 => Some(encoding_rs::BIG5),
            CharacterSet::GB18030 => Some(encoding_rs::GB18030),
            CharacterSet::GB2312 => Some(encoding_rs::GBK),
            CharacterSet::EucKr => Some(encoding_rs::EUC_KR),
            CharacterSet::UTF16BE => Some(encoding_rs::UTF_16BE),
            CharacterSet::UTF16LE => Some(encoding_rs::UTF_16LE),
            _ => None,
        }
    }

    pub fn decode(&self, input: &[u8]) -> Result<String> {
        match self {
            CharacterSet::Cp437 => {
                use codepage_437::BorrowFromCp437;
                use codepage_437::CP437_CONTROL;

                Ok(String::borrow_from_cp437(input, &CP437_CONTROL))
            }
            CharacterSet::UTF32BE => {
                if !input.len().is_multiple_of(4) {
                    return Err(Error::InvalidFormat {
                        message: "Invalid UTF-32BE: trailing bytes".into(),
                    }
                    .into());
                }
                input
                    .chunks_exact(4)
                    .map(|c| {
                        let val = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
                        char::from_u32(val).ok_or_else(|| {
                            Error::InvalidFormat {
                                message: "Invalid UTF-32BE".into(),
                            }
                            .into()
                        })
                    })
                    .collect()
            }
            CharacterSet::UTF32LE => {
                if !input.len().is_multiple_of(4) {
                    return Err(Error::InvalidFormat {
                        message: "Invalid UTF-32LE: trailing bytes".into(),
                    }
                    .into());
                }
                input
                    .chunks_exact(4)
                    .map(|c| {
                        let val = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                        char::from_u32(val).ok_or_else(|| {
                            Error::InvalidFormat {
                                message: "Invalid UTF-32LE".into(),
                            }
                            .into()
                        })
                    })
                    .collect()
            }
            CharacterSet::Binary | CharacterSet::ISO8859_1 => {
                Ok(input.iter().map(|&b| char::from(b)).collect())
            }
            CharacterSet::Ascii => {
                let mut s = String::with_capacity(input.len());
                for &b in input {
                    if b > 0x7F {
                        return Err(Error::InvalidFormat {
                            message: "Invalid ASCII".into(),
                        }
                        .into());
                    }
                    s.push(char::from(b));
                }
                Ok(s)
            }
            _ => {
                if let Some(enc) = self.encoding() {
                    let (res, _, had_errors) = enc.decode(input);
                    if had_errors {
                        return Err(Error::InvalidFormat {
                            message: "Could not decode character".into(),
                        }
                        .into());
                    }
                    Ok(res.into_owned())
                } else {
                    Err(Error::InvalidFormat {
                        message: "Unsupported encoding".into(),
                    }
                    .into())
                }
            }
        }
    }
}
