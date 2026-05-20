/*
 * Copyright 2016 Nu-book Inc.
 * Copyright 2016 ZXing authors
 */
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

use crate::Error;
use crate::common::cpp_essentials::{DecoderResult, StructuredAppendInfo};
use crate::common::{
    AIFlag, BitMatrix, BitSource, CharacterSet, ECIStringBuilder, Eci, SymbologyIdentifier,
};
use crate::qrcode::common::{ErrorCorrectionLevel, Mode, Version};
use crate::qrcode::cpp_port::bitmatrix_parser::{
    read_codewords, read_format_information, read_version,
};
use crate::qrcode::decoder::DataBlock;
use crate::qrcode::decoder::qrcode_decoder::correct_errors;

/**
* See specification GBT 18284-2000
*/
pub fn decode_hanzi_segment(
    bits: &mut BitSource,
    count: u32,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    let mut count = count;

    // Each character will require 2 bytes, decode as GB2312
    // There is no ECI value for GB2312, use GB18030 which is a superset
    result.switch_encoding(CharacterSet::GB18030, false);
    result.reserve(2 * count as usize);

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
        *result += ((assembled_two_bytes >> 8) & 0xFF) as u8;
        *result += (assembled_two_bytes & 0xFF) as u8;
        count -= 1;
    }
    Ok(())
}

pub fn decode_kanji_segment(
    bits: &mut BitSource,
    count: u32,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    let mut count = count;
    // Each character will require 2 bytes. Read the characters as 2-byte pairs
    // and decode as ShiftJis afterwards
    result.switch_encoding(CharacterSet::ShiftJis, false);
    result.reserve(2 * count as usize);

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
        *result += (assembled_two_bytes >> 8) as u8;
        *result += (assembled_two_bytes) as u8;
        count -= 1;
    }
    Ok(())
}

pub fn decode_byte_segment(
    bits: &mut BitSource,
    count: u32,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    result.switch_encoding(CharacterSet::Unknown, false);
    result.reserve(count as usize);

    for _i in 0..count {
        *result += (bits.read_bits(8)?) as u8;
    }
    Ok(())
}

pub fn to_alpha_numeric_char(value: u32) -> Result<char> {
    let value = value as usize;
    /**
     * See ISO 18004:2006, 6.4.4 Table 5
     */
    const ALPHANUMERIC_CHARS: [char; 45] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
        'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        ' ', '$', '%', '*', '+', '-', '.', '/', ':',
    ];

    if value >= (ALPHANUMERIC_CHARS.len()) {
        return Err(Error::invalid_format("to_alpha_numeric_char: invalid symbol value").into());
    }

    Ok(ALPHANUMERIC_CHARS[value])
}

pub fn decode_alphanumeric_segment(
    bits: &mut BitSource,
    count: u32,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    let mut count = count;

    // Read two characters at a time
    let mut buffer = Vec::new();

    while count > 1 {
        let next_two_chars_bits = bits.read_bits(11)?;
        buffer.push(to_alpha_numeric_char(next_two_chars_bits / 45)?);
        buffer.push(to_alpha_numeric_char(next_two_chars_bits % 45)?);
        count -= 2;
    }
    if count == 1 {
        // special case: one character left
        buffer.push(to_alpha_numeric_char(bits.read_bits(6)?)?);
    }
    // See section 6.4.8.1, 6.4.8.2
    if result.symbology.ai_flag != AIFlag::None {
        // We need to massage the result a bit if in an FNC1 mode:
        let mut i = 0;
        while i < buffer.len() {
            if buffer[i] == '%' {
                if i + 1 < buffer.len() && buffer[i + 1] == '%' {
                    buffer.remove(i + 1);
                } else {
                    // In alpha mode, % should be converted to FNC1 separator 0x1D
                    buffer[i] = char::from(0x1D);
                }
            }
            i += 1;
        }
    }

    result.switch_encoding(CharacterSet::ISO8859_1, false);
    *result += buffer.iter().collect::<String>();

    Ok(())
}

pub fn decode_numeric_segment(
    bits: &mut BitSource,
    count: u32,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    let mut count = count;

    result.switch_encoding(CharacterSet::ISO8859_1, false);
    result.reserve(count as usize);

    while count > 0 {
        let n = std::cmp::min(count, 3);
        let n_digits = bits.read_bits(1 + 3 * n as usize)?; // read 4, 7 or 10 bits into 1, 2 or 3 digits
        result.append_string(&crate::common::cpp_essentials::util::to_string(
            n_digits as usize,
            n as usize,
        )?);
        count -= n;
    }

    Ok(())
}

pub fn parse_ecivalue(bits: &mut BitSource) -> Result<Eci> {
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
    Err(Error::invalid_format("parse_ecivalue: invalid value").into())
}

/**
 * QR codes encode mode indicators and terminator codes into a constant bit length of 4.
 * micro QR codes have terminator codes that vary in bit length but are always longer than
 * the mode indicators.
 * M1 - 0 length mode code, 3 bits terminator code
 * M2 - 1 bit mode code, 5 bits terminator code
 * M3 - 2 bit mode code, 7 bits terminator code
 * M4 - 3 bit mode code, 9 bits terminator code
 * IsTerminator peaks into the bit stream to see if the current position is at the start of
 * a terminator code.  If true, then the decoding can finish. If false, then the decoding
 * can read off the next mode code.
 *
 * See ISO 18004:2015, 7.4.1 Table 2
 *
 * @param bits the stream of bits that might have a terminator code
 * @param version the QR or micro QR code version
 */
pub fn is_end_of_stream(bits: &mut BitSource, version: &Version) -> Result<bool> {
    let bits_required = Mode::get_terminator_bit_length(version); //super::qr_codec_mode::TerminatorBitsLength(version);
    let bits_available = std::cmp::min(bits.available(), bits_required as usize);
    Ok(bits_available == 0 || bits.peek_bits(bits_available)? == 0)
}

/**
* <p>QR Codes can encode text as bits in one of several modes, and can use multiple modes
* in one QR Code. This method decodes the bits back into text.</p>
*
* <p>See ISO 18004:2006, 6.4.3 - 6.4.7</p>
*/
// ZXING_EXPORT_TEST_ONLY
pub fn decode_bit_stream(
    bytes: &[u8],
    version: &Version,
    ec_level: ErrorCorrectionLevel,
) -> Result<DecoderResult<bool>> {
    let mut bits = BitSource::new(bytes);
    let mut result = ECIStringBuilder::default();
    result.symbology = SymbologyIdentifier {
        code: b'Q',
        modifier: b'1',
        eci_modifier_offset: 1,
        ai_flag: AIFlag::None,
    };
    let mut structured_append = StructuredAppendInfo::default();
    let mode_bit_length = Mode::get_codec_mode_bits_length(version);

    if version.is_model1() {
        bits.read_bits(4)?; /* Model 1 is leading with 4 0-bits -> drop them */
    }

    let res: Result<()> = (|| {
        while !is_end_of_stream(&mut bits, version)? {
            let mode: Mode = if mode_bit_length == 0 {
                Mode::Numeric // MicroQRCode version 1 is always NUMERIC and mode_bit_length is 0
            } else {
                Mode::codec_mode_for_bits(
                    bits.read_bits(mode_bit_length as usize)?,
                    Some(version.qr_type),
                )?
            };

            match mode {
                Mode::Fnc1FirstPosition => {
                    result.symbology.modifier = b'3';
                    result.symbology.ai_flag = AIFlag::GS1; // In Alphanumeric mode undouble doubled '%' and treat single '%' as <GS>
                }
                Mode::Fnc1SecondPosition => {
                    if !result.is_empty() {
                        return Err(Error::invalid_format(
                            "AIM Application Indicator (FNC1 in second position) at illegal position",
                        ).into());
                    }
                    result.symbology.modifier = b'5';
                    // ISO/IEC 18004:2015 7.4.8.3 AIM Application Indicator (FNC1 in second position), "00-99" or "A-Za-z"
                    let app_ind = bits.read_bits(8)?;
                    if app_ind < 100 {
                        // "00-09"
                        result +=
                            crate::common::cpp_essentials::util::to_string(app_ind as usize, 2)?;
                    } else if (165..=190).contains(&app_ind) || (197..=222).contains(&app_ind) {
                        // "A-Za-z"
                        result += (app_ind - 100) as u8;
                    } else {
                        return Err(
                            Error::invalid_format("Invalid AIM Application Indicator").into()
                        );
                    }
                    result.symbology.ai_flag = AIFlag::AIM;
                }
                Mode::StructuredAppend => {
                    // sequence number and parity is added later to the result metadata
                    // Read next 4 bits of index, 4 bits of symbol count, and 8 bits of parity data, then continue
                    structured_append.index = bits.read_bits(4)? as i32;
                    structured_append.count = bits.read_bits(4)? as i32 + 1;
                    structured_append.id = (bits.read_bits(8)?).to_string();
                }
                Mode::Eci => {
                    // Count doesn't apply to ECI
                    result.switch_encoding(parse_ecivalue(&mut bits)?.into(), true);
                }
                Mode::Hanzi => {
                    // First handle Hanzi mode which does not start with character count
                    // chinese mode contains a sub set indicator right after mode indicator
                    let subset = bits.read_bits(4)?;
                    if subset != 1 {
                        // GB2312_SUBSET is the only supported one right now
                        return Err(Error::invalid_format("Unsupported HANZI subset").into());
                    }
                    let count = bits.read_bits(mode.character_count_bits(version) as usize)?;
                    decode_hanzi_segment(&mut bits, count, &mut result)?;
                }
                _ => {
                    // "Normal" QR code modes:
                    // How many characters will follow, encoded in this mode?
                    let count = bits.read_bits(mode.character_count_bits(version) as usize)?;
                    match mode {
                        Mode::Numeric => decode_numeric_segment(&mut bits, count, &mut result)?,
                        Mode::Alphanumeric => {
                            decode_alphanumeric_segment(&mut bits, count, &mut result)?
                        }
                        Mode::Byte => decode_byte_segment(&mut bits, count, &mut result)?,
                        Mode::Kanji => decode_kanji_segment(&mut bits, count, &mut result)?,
                        _ => return Err(Error::invalid_format("Invalid CodecMode").into()),
                    };
                }
            }
        }
        Ok(())
    })();

    Ok(DecoderResult::with_eci_string_builder(result)
        .with_error(res.err())
        .with_ec_level(ec_level.to_string())
        .with_version_number(version.get_version_number())
        .with_structured_append(structured_append)
        .with_is_model1(version.is_model1()))
}

pub fn decode(bits: &BitMatrix) -> Result<DecoderResult<bool>> {
    if !Version::has_valid_size(bits) {
        return Err(Error::invalid_format("Invalid symbol size").into());
    }
    let Ok(format_info) = read_format_information(bits) else {
        return Err(Error::invalid_format("Invalid format information").into());
    };

    let Ok(version) = read_version(bits, format_info.qr_type()) else {
        return Err(Error::invalid_format("Invalid version").into());
    };

    // Read codewords
    let codewords = read_codewords(bits, version, &format_info)?;
    if codewords.is_empty() {
        return Err(Error::invalid_format("Failed to read codewords").into());
    }

    // Separate into data blocks
    let data_blocks: Vec<DataBlock> =
        DataBlock::get_data_blocks(&codewords, version, format_info.error_correction_level)?;
    if data_blocks.is_empty() {
        return Err(Error::invalid_format("Failed to get data blocks").into());
    }

    // Count total number of data bytes
    let op =
        |total_bytes, data_block: &DataBlock| total_bytes + data_block.get_num_data_codewords();
    let total_bytes = data_blocks.iter().fold(0, op);
    let mut result_bytes = vec![0u8; total_bytes as usize];
    let mut result_iterator = 0;

    // Error-correct and copy data blocks together into a stream of bytes
    for data_block in data_blocks.iter() {
        let mut codeword_bytes = data_block.get_codewords().to_vec();
        let num_data_codewords = data_block.get_num_data_codewords() as usize;

        correct_errors(&mut codeword_bytes, num_data_codewords)?;

        result_bytes[result_iterator..(result_iterator + num_data_codewords)]
            .copy_from_slice(&codeword_bytes[..num_data_codewords]);
        result_iterator += num_data_codewords;
    }

    // decode the contents of that stream of bytes
    Ok(
        decode_bit_stream(&result_bytes, version, format_info.error_correction_level)?
            .with_is_mirrored(format_info.is_mirrored),
    )
}
