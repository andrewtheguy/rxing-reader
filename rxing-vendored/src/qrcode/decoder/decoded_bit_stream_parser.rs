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

use once_cell::sync::Lazy;

use crate::{
    DecodeHints, Exceptions,
    common::{
        BitSource, CharacterSet, DecoderRXingResult, ECIStringBuilder, Eci, Result, string_utils,
    },
};

use crate::qrcode::common::{ErrorCorrectionLevel, Mode, VersionRef};

/*
 * <p>QR Codes can encode text as bits in one of several modes, and can use multiple modes
 * in one QR Code. This class decodes the bits back into text.</p>
 *
 * <p>See ISO 18004:2006, 6.4.3 - 6.4.7</p>
 *
 * @author Sean Owen
 */

/**
 * See ISO 18004:2006, 6.4.4 Table 5
 */
const ALPHANUMERIC_CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
const GB2312_SUBSET: u32 = 1;

static CACHED_ALPHANUMERIC_CHARS: Lazy<Vec<char>> =
    Lazy::new(|| ALPHANUMERIC_CHARS.chars().collect());

pub fn decode(
    bytes: &[u8],
    version: VersionRef,
    ec_level: ErrorCorrectionLevel,
    hints: &DecodeHints,
) -> Result<DecoderRXingResult> {
    let mut bits = BitSource::new(bytes);
    let mut result = ECIStringBuilder::with_capacity(50); //String::with_capacity(50);
    let mut byte_segments: std::vec::Vec<std::vec::Vec<u8>> = vec![];
    let mut symbol_sequence = -1;
    let mut parity_data = -1;

    let mut current_character_set_eci = None;
    let mut fc1_in_effect = false;
    let mut has_fnc1first = false;
    let mut has_fnc1second = false;
    let mut mode;
    loop {
        // While still another segment to read...
        if bits.available() < 4 {
            // OK, assume we're done. Really, a TERMINATOR mode should have been recorded here
            mode = Mode::Terminator;
        } else {
            mode = Mode::for_bits(bits.read_bits(4)? as u8)?; // mode is encoded by 4 bits
        }
        match mode {
            Mode::Terminator => {}
            Mode::Fnc1FirstPosition => {
                has_fnc1first = true; // symbology detection
                // We do little with FNC1 except alter the parsed result a bit according to the spec
                fc1_in_effect = true;
            }
            Mode::Fnc1SecondPosition => {
                has_fnc1second = true; // symbology detection
                // We do little with FNC1 except alter the parsed result a bit according to the spec
                fc1_in_effect = true;
            }
            Mode::StructuredAppend => {
                if bits.available() < 16 {
                    return Err(Exceptions::format_with(format!(
                        "Mode::Structured append expected bits.available() < 16, found bits of {}",
                        bits.available()
                    )));
                }
                // sequence number and parity is added later to the result metadata
                // Read next 8 bits (symbol sequence #) and 8 bits (parity data), then continue
                symbol_sequence = bits.read_bits(8)? as i32;
                parity_data = bits.read_bits(8)? as i32;
            }
            Mode::Eci => {
                // Count doesn't apply to ECI
                let value = parse_ecivalue(&mut bits)?;
                current_character_set_eci = CharacterSet::from(value).into(); //CharacterSet::get_character_set_by_eci(value).ok();
                if current_character_set_eci.is_none() {
                    return Err(Exceptions::format_with(format!(
                        "Value of {value} not valid"
                    )));
                }
            }
            Mode::Hanzi => {
                // First handle Hanzi mode which does not start with character count
                // Chinese mode contains a sub set indicator right after mode indicator
                let subset = bits.read_bits(4)?;
                let count_hanzi =
                    bits.read_bits(mode.get_character_count_bits(version) as usize)? as usize;
                if subset == GB2312_SUBSET {
                    decode_hanzi_segment(&mut bits, &mut result, count_hanzi)?;
                }
            }
            _ => {
                // "Normal" QR code modes:
                // How many characters will follow, encoded in this mode?
                let count = bits.read_bits(mode.get_character_count_bits(version) as usize)? as usize;
                match mode {
                    Mode::Numeric => decode_numeric_segment(&mut bits, &mut result, count)?,
                    Mode::Alphanumeric => {
                        decode_alphanumeric_segment(&mut bits, &mut result, count, fc1_in_effect)?
                    }
                    Mode::Byte => decode_byte_segment(
                        &mut bits,
                        &mut result,
                        count,
                        current_character_set_eci,
                        &mut byte_segments,
                        hints,
                    )?,
                    Mode::Kanji => decode_kanji_segment(&mut bits, &mut result, count)?,
                    _ => return Err(Exceptions::FORMAT),
                }
            }
        }

        if mode == Mode::Terminator {
            break;
        }
    }

    let symbology_modifier = get_symbology_identifier(
        current_character_set_eci.is_some(),
        has_fnc1first,
        has_fnc1second,
    );

    Ok(DecoderRXingResult::with_all(
        bytes.to_owned(),
        result.build_result().to_string(),
        byte_segments.to_vec(),
        format!("{}", u8::from(ec_level)),
        symbol_sequence,
        parity_data,
        symbology_modifier,
        String::default(),
        false,
    ))
}

fn get_symbology_identifier(has_charset: bool, has_fnc1first: bool, has_fnc1second: bool) -> u32 {
    if has_charset {
        if has_fnc1first {
            4
        } else if has_fnc1second {
            6
        } else {
            2
        }
    } else if has_fnc1first {
        3
    } else if has_fnc1second {
        5
    } else {
        1
    }
}

/**
 * See specification GBT 18284-2000
 */
fn decode_hanzi_segment(
    bits: &mut BitSource,
    result: &mut ECIStringBuilder,
    count: usize,
) -> Result<()> {
    // Don't crash trying to read more bits than we have available.
    if count * 13 > bits.available() {
        return Err(Exceptions::FORMAT);
    }

    // Each character will require 2 bytes. Read the characters as 2-byte pairs
    // and decode as GB2312 afterwards
    let mut buffer = vec![0u8; 2 * count];
    let mut offset = 0;
    let mut count = count;
    while count > 0 {
        // Each 13 bits encodes a 2-byte character
        let two_bytes = bits.read_bits(13)?;
        let mut assembled_two_bytes = ((two_bytes / 0x060) << 8) | (two_bytes % 0x060);
        if assembled_two_bytes < 0x00A00 {
            // In the 0xA1A1 to 0xAAFE range
            assembled_two_bytes += 0x0A1A1;
        } else {
            // In the 0xB0A1 to 0xFAFE range
            assembled_two_bytes += 0x0A6A1;
        }

        buffer[offset] = (assembled_two_bytes >> 8) as u8;
        buffer[offset + 1] = assembled_two_bytes as u8;
        offset += 2;
        count -= 1;
    }

    result.append_eci(Eci::GB18030);
    result.append_bytes(&buffer);

    Ok(())
}

fn decode_kanji_segment(
    bits: &mut BitSource,
    result: &mut ECIStringBuilder,
    count: usize,
) -> Result<()> {
    // Don't crash trying to read more bits than we have available.
    if count * 13 > bits.available() {
        return Err(Exceptions::FORMAT);
    }

    // Each character will require 2 bytes. Read the characters as 2-byte pairs
    // and decode as ShiftJis afterwards
    let mut buffer = vec![0u8; 2 * count];
    let mut offset = 0;
    let mut count = count;
    while count > 0 {
        // Each 13 bits encodes a 2-byte character
        let two_bytes = bits.read_bits(13)?;
        let mut assembled_two_bytes = ((two_bytes / 0x0C0) << 8) | (two_bytes % 0x0C0);
        if assembled_two_bytes < 0x01F00 {
            // In the 0x8140 to 0x9FFC range
            assembled_two_bytes += 0x08140;
        } else {
            // In the 0xE040 to 0xEBBF range
            assembled_two_bytes += 0x0C140;
        }
        buffer[offset] = (assembled_two_bytes >> 8) as u8;
        buffer[offset + 1] = assembled_two_bytes as u8;
        offset += 2;
        count -= 1;
    }

    let encoder = CharacterSet::ShiftJis;

    result.append_eci(Eci::from(encoder));
    result.append_bytes(&buffer);

    Ok(())
}

fn decode_byte_segment(
    bits: &mut BitSource,
    result: &mut ECIStringBuilder,
    count: usize,
    current_character_set_eci: Option<CharacterSet>,
    byte_segments: &mut Vec<Vec<u8>>,
    hints: &DecodeHints,
) -> Result<()> {
    // Don't crash trying to read more bits than we have available.
    if 8 * count > bits.available() {
        return Err(Exceptions::FORMAT);
    }

    let mut read_bytes = vec![0u8; count];

    for byte in read_bytes.iter_mut().take(count) {
        *byte = bits.read_bits(8)? as u8;
    }
    let encoding = if current_character_set_eci.is_none() {
        // The spec isn't clear on this mode; see
        // section 6.4.5: t does not say which encoding to assuming
        // upon decoding. I have seen ISO-8859-1 used as well as
        // ShiftJis -- without anything like an ECI designator to
        // give a hint.
        string_utils::guess_charset(&read_bytes, hints).ok_or(Exceptions::ILLEGAL_STATE)?
    } else {
        current_character_set_eci.ok_or(Exceptions::ILLEGAL_STATE)?
    };

    result.append_eci(Eci::from(encoding));
    result.append_bytes(&read_bytes);

    byte_segments.push(read_bytes);

    Ok(())
}

fn to_alpha_numeric_char(value: u32) -> Result<char> {
    if value as usize >= ALPHANUMERIC_CHARS.len() {
        return Err(Exceptions::FORMAT);
    }

    Ok(CACHED_ALPHANUMERIC_CHARS[value as usize])
}

fn decode_alphanumeric_segment(
    bits: &mut BitSource,
    result: &mut ECIStringBuilder,
    count: usize,
    fc1_in_effect: bool,
) -> Result<()> {
    let mut r_hld = Vec::with_capacity(count);
    // Read two characters at a time
    let start = 0;
    let mut count = count;
    while count > 1 {
        if bits.available() < 11 {
            return Err(Exceptions::FORMAT);
        }
        let next_two_chars_bits = bits.read_bits(11)?;
        r_hld.push(to_alpha_numeric_char(next_two_chars_bits / 45)?);
        r_hld.push(to_alpha_numeric_char(next_two_chars_bits % 45)?);
        count -= 2;
    }
    if count == 1 {
        // special case: one character left
        if bits.available() < 6 {
            return Err(Exceptions::FORMAT);
        }
        r_hld.push(to_alpha_numeric_char(bits.read_bits(6)?)?);
    }
    // See section 6.4.8.1, 6.4.8.2
    if fc1_in_effect {
        // We need to massage the result a bit if in an FNC1 mode.
        // Walk forward with explicit index management because remove(i+1)
        // shrinks the buffer underneath us.
        let mut i = start;
        while i < r_hld.len() {
            if r_hld[i] == '%' {
                if i + 1 < r_hld.len() && r_hld[i + 1] == '%' {
                    // %% is rendered as %
                    r_hld.remove(i + 1);
                } else {
                    // In alpha mode, % should be converted to FNC1 separator 0x1D
                    r_hld[i] = 0x1D as char;
                }
            }
            i += 1;
        }
    }

    result.append_eci(Eci::ISO8859_1);
    result.append_string(&r_hld.iter().collect::<String>());

    Ok(())
}

fn decode_numeric_segment(
    bits: &mut BitSource,
    result: &mut ECIStringBuilder,
    count: usize,
) -> Result<()> {
    result.append_eci(Eci::ISO8859_1);
    let mut count = count;
    // Read three digits at a time
    while count >= 3 {
        // Each 10 bits encodes three digits
        if bits.available() < 10 {
            return Err(Exceptions::FORMAT);
        }
        let three_digits_bits = bits.read_bits(10)?;
        if three_digits_bits >= 1000 {
            return Err(Exceptions::FORMAT);
        }
        result.append_char(to_alpha_numeric_char(three_digits_bits / 100)?);
        result.append_char(to_alpha_numeric_char((three_digits_bits / 10) % 10)?);
        result.append_char(to_alpha_numeric_char(three_digits_bits % 10)?);
        count -= 3;
    }
    if count == 2 {
        // Two digits left over to read, encoded in 7 bits
        if bits.available() < 7 {
            return Err(Exceptions::FORMAT);
        }
        let two_digits_bits = bits.read_bits(7)?;
        if two_digits_bits >= 100 {
            return Err(Exceptions::FORMAT);
        }
        result.append_char(to_alpha_numeric_char(two_digits_bits / 10)?);
        result.append_char(to_alpha_numeric_char(two_digits_bits % 10)?);
    } else if count == 1 {
        // One digit left over to read
        if bits.available() < 4 {
            return Err(Exceptions::FORMAT);
        }
        let digit_bits = bits.read_bits(4)?;
        if digit_bits >= 10 {
            return Err(Exceptions::FORMAT);
        }
        result.append_char(to_alpha_numeric_char(digit_bits)?);
    }

    Ok(())
}

fn parse_ecivalue(bits: &mut BitSource) -> Result<Eci> {
    let first_byte = bits.read_bits(8)?;
    if (first_byte & 0x80) == 0 {
        // just one byte
        return Eci::try_from(first_byte & 0x7F);
    }
    if (first_byte & 0xC0) == 0x80 {
        // two bytes
        let second_byte = bits.read_bits(8)?;
        return Eci::try_from(((first_byte & 0x3F) << 8) | second_byte);
    }
    if (first_byte & 0xE0) == 0xC0 {
        // three bytes
        let second_third_bytes = bits.read_bits(16)?;
        return Eci::try_from(((first_byte & 0x1F) << 16) | second_third_bytes);
    }

    Err(Exceptions::FORMAT)
}
