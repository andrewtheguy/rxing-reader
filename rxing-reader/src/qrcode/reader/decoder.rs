/*
 * Copyright 2016 Nu-book Inc.
 * Copyright 2016 ZXing authors
 */
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result, bail, ensure};

use crate::Error;
use crate::common::{
    AIFlag, BitMatrix, BitSource, CharacterSet, ECIStringBuilder, Eci, SymbologyIdentifier,
    detect::{DecoderResult, StructuredAppendInfo, to_fixed_len_string as padded_digits},
};
use crate::qrcode::reed_solomon::correct_qr_errors;
use crate::qrcode::{ErrorCorrectionLevel, Mode, Version, VersionRef};

use super::bitmatrix_parser::{read_codewords, read_format_information, read_version};

const BYTE_BITS: usize = 8;
const DOUBLE_BYTE_SEGMENT_BITS: usize = 13;
const HANZI_SUBSET_BITS: usize = 4;
const GB2312_SUBSET: u32 = 1;
const HANZI_BYTE_MULTIPLIER: u32 = 0x060;
const HANZI_LOW_RANGE_LIMIT: u32 = 0x00A00;
const HANZI_LOW_RANGE_OFFSET: u32 = 0x0A1A1;
const HANZI_HIGH_RANGE_OFFSET: u32 = 0x0A6A1;
const KANJI_BYTE_MULTIPLIER: u32 = 0x0C0;
const KANJI_LOW_RANGE_LIMIT: u32 = 0x01F00;
const KANJI_LOW_RANGE_OFFSET: u32 = 0x08140;
const KANJI_HIGH_RANGE_OFFSET: u32 = 0x0C140;
const ALPHANUMERIC_PAIR_BITS: usize = 11;
const ALPHANUMERIC_SINGLE_BITS: usize = 6;
const ALPHANUMERIC_RADIX: usize = 45;
const ALPHANUMERIC_CHARS: [char; ALPHANUMERIC_RADIX] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
    'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ' ', '$', '%', '*', '+', '-', '.', '/', ':',
];
const NUMERIC_DIGIT_GROUP_BITS: [usize; 4] = [0, 4, 7, 10];
const FNC1_GROUP_SEPARATOR: u8 = 0x1D;
const STRUCTURED_APPEND_INDEX_BITS: usize = 4;
const STRUCTURED_APPEND_COUNT_BITS: usize = 4;
const STRUCTURED_APPEND_PARITY_BITS: usize = 8;
const AIM_NUMERIC_MAX: u32 = 99;
const AIM_ALPHA_OFFSET: u32 = 100;
const ECI_ONE_BYTE_PREFIX_MASK: u32 = 0x80;
const ECI_ONE_BYTE_PAYLOAD_MASK: u32 = 0x7F;
const ECI_TWO_BYTE_PREFIX_MASK: u32 = 0xC0;
const ECI_TWO_BYTE_PREFIX: u32 = 0x80;
const ECI_TWO_BYTE_PAYLOAD_MASK: u32 = 0x3F;
const ECI_THREE_BYTE_PREFIX_MASK: u32 = 0xE0;
const ECI_THREE_BYTE_PREFIX: u32 = 0xC0;
const ECI_THREE_BYTE_PAYLOAD_MASK: u32 = 0x1F;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HanziSubset {
    Gb2312,
}

impl TryFrom<u32> for HanziSubset {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            GB2312_SUBSET => Ok(Self::Gb2312),
            _ => bail!(Error::invalid_format(format!(
                    "Unsupported HANZI subset {value} (only GB2312_SUBSET = {GB2312_SUBSET} is supported)"
                )
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AimApplicationIndicator {
    Numeric(usize),
    Alphabetic(u8),
}

impl TryFrom<u32> for AimApplicationIndicator {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0..=AIM_NUMERIC_MAX => Ok(Self::Numeric(
                usize::try_from(value).with_context(|| {
                    Error::invalid_format(format!(
                        "AIM numeric indicator {value} does not fit in usize"
                    ))
                })?,
            )),
            165..=190 | 197..=222 => Ok(Self::Alphabetic(
                u8::try_from(value - AIM_ALPHA_OFFSET).with_context(|| {
                    Error::invalid_format(format!(
                        "AIM alphabetic indicator {value} does not fit in u8"
                    ))
                })?,
            )),
            _ => bail!(Error::invalid_format(format!(
                    "Invalid AIM Application Indicator value {value} (expected 0..=99, 165..=190, or 197..=222)"
                )
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EciValueLength {
    OneByte,
    TwoBytes,
    ThreeBytes,
}

impl EciValueLength {
    fn from_first_byte(first_byte: u32) -> Result<Self> {
        if (first_byte & ECI_ONE_BYTE_PREFIX_MASK) == 0 {
            return Ok(Self::OneByte);
        }
        if (first_byte & ECI_TWO_BYTE_PREFIX_MASK) == ECI_TWO_BYTE_PREFIX {
            return Ok(Self::TwoBytes);
        }
        if (first_byte & ECI_THREE_BYTE_PREFIX_MASK) == ECI_THREE_BYTE_PREFIX {
            return Ok(Self::ThreeBytes);
        }
        bail!(Error::invalid_format(format!(
                "parse_eci_value: invalid leading byte 0x{first_byte:02X} (top bits do not match any ECI length encoding)"
            )
        ));
    }
}

fn read_bits_as_usize(bits: &mut BitSource, num_bits: usize, context: &str) -> Result<usize> {
    let value = bits
        .read_bits(num_bits)
        .with_context(|| Error::invalid_format(format!("reading {context}")))?;
    usize::try_from(value).with_context(|| {
        Error::invalid_format(format!("{context} does not fit in usize (num_bits={num_bits})"))
    })
}

fn read_count(bits: &mut BitSource, num_bits: usize) -> Result<usize> {
    read_bits_as_usize(bits, num_bits, "QR segment count")
}

fn append_be_u16(result: &mut ECIStringBuilder, value: u32) -> Result<()> {
    let value = u16::try_from(value).with_context(|| {
        Error::invalid_format(format!("double-byte character value 0x{value:X} out of range"))
    })?;
    let bytes = value.to_be_bytes();
    *result += bytes[0];
    *result += bytes[1];
    Ok(())
}

/// See specification GBT 18284-2000
pub fn decode_hanzi_segment(
    bits: &mut BitSource,
    count: usize,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    let mut count = count;

    // Each character will require 2 bytes, decode as GB2312
    // There is no ECI value for GB2312, use GB18030 which is a superset
    result.switch_encoding(CharacterSet::GB18030, false);
    result.reserve(2 * count);

    while count > 0 {
        // Each 13 bits encodes a 2-byte character
        let two_bytes = bits
            .read_bits(DOUBLE_BYTE_SEGMENT_BITS)
            .context("reading HANZI double-byte segment")?;
        let mut assembled_two_bytes =
            ((two_bytes / HANZI_BYTE_MULTIPLIER) << 8) | (two_bytes % HANZI_BYTE_MULTIPLIER);
        if assembled_two_bytes < HANZI_LOW_RANGE_LIMIT {
            // In the 0xA1A1 to 0xAAFE range
            assembled_two_bytes += HANZI_LOW_RANGE_OFFSET;
        } else {
            // In the 0xB0A1 to 0xFAFE range
            assembled_two_bytes += HANZI_HIGH_RANGE_OFFSET;
        }
        append_be_u16(result, assembled_two_bytes)?;
        count -= 1;
    }
    Ok(())
}

pub fn decode_kanji_segment(
    bits: &mut BitSource,
    count: usize,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    let mut count = count;
    // Each character will require 2 bytes. Read the characters as 2-byte pairs
    // and decode as ShiftJis afterwards
    result.switch_encoding(CharacterSet::ShiftJis, false);
    result.reserve(2 * count);

    while count > 0 {
        // Each 13 bits encodes a 2-byte character
        let two_bytes = bits
            .read_bits(DOUBLE_BYTE_SEGMENT_BITS)
            .context("reading KANJI double-byte segment")?;
        let mut assembled_two_bytes =
            ((two_bytes / KANJI_BYTE_MULTIPLIER) << 8) | (two_bytes % KANJI_BYTE_MULTIPLIER);
        if assembled_two_bytes < KANJI_LOW_RANGE_LIMIT {
            // In the 0x8140 to 0x9FFC range
            assembled_two_bytes += KANJI_LOW_RANGE_OFFSET;
        } else {
            // In the 0xE040 to 0xEBBF range
            assembled_two_bytes += KANJI_HIGH_RANGE_OFFSET;
        }
        append_be_u16(result, assembled_two_bytes)?;
        count -= 1;
    }
    Ok(())
}

pub fn decode_byte_segment(
    bits: &mut BitSource,
    count: usize,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    result.switch_encoding(CharacterSet::Unknown, false);
    result.reserve(count);

    for _ in 0..count {
        *result += u8::try_from(
            bits.read_bits(BYTE_BITS)
                .context("reading QR byte segment byte")?,
        )
        .with_context(|| Error::invalid_format("byte segment produced a value outside u8"))?;
    }
    Ok(())
}

pub fn to_alphanumeric_char(value: usize) -> Result<char> {
    ALPHANUMERIC_CHARS
        .get(value)
        .copied()
        .with_context(|| {
            Error::invalid_format(format!(
                "to_alphanumeric_char: value {value} out of range (expected 0..{})",
                ALPHANUMERIC_CHARS.len()
            ))
        })
}

pub fn decode_alphanumeric_segment(
    bits: &mut BitSource,
    count: usize,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    let mut count = count;

    // Read two characters at a time
    let mut buffer = Vec::new();

    while count > 1 {
        let next_two_chars_bits =
            read_bits_as_usize(bits, ALPHANUMERIC_PAIR_BITS, "alphanumeric pair value")?;
        buffer.push(to_alphanumeric_char(next_two_chars_bits / ALPHANUMERIC_RADIX)?);
        buffer.push(to_alphanumeric_char(next_two_chars_bits % ALPHANUMERIC_RADIX)?);
        count -= 2;
    }
    if count == 1 {
        // special case: one character left
        let value = read_bits_as_usize(
            bits,
            ALPHANUMERIC_SINGLE_BITS,
            "alphanumeric single value",
        )?;
        buffer.push(to_alphanumeric_char(value)?);
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
                    buffer[i] = char::from(FNC1_GROUP_SEPARATOR);
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
    count: usize,
    result: &mut ECIStringBuilder,
) -> Result<()> {
    let mut count = count;

    result.switch_encoding(CharacterSet::ISO8859_1, false);
    result.reserve(count);

    while count > 0 {
        let n = std::cmp::min(count, 3);
        let n_digits =
            read_bits_as_usize(bits, NUMERIC_DIGIT_GROUP_BITS[n], "numeric segment value")?;
        result.append_string(&padded_digits(n_digits, n)?);
        count -= n;
    }

    Ok(())
}

pub fn parse_eci_value(bits: &mut BitSource) -> Result<Eci> {
    let first_byte = bits.read_bits(BYTE_BITS).context("reading ECI first byte")?;
    match EciValueLength::from_first_byte(first_byte)? {
        EciValueLength::OneByte => Eci::try_from(first_byte & ECI_ONE_BYTE_PAYLOAD_MASK)
            .context("decoding one-byte ECI assignment value"),
        EciValueLength::TwoBytes => {
            let second_byte = bits.read_bits(BYTE_BITS).context("reading ECI second byte")?;
            Eci::try_from(((first_byte & ECI_TWO_BYTE_PAYLOAD_MASK) << BYTE_BITS) | second_byte)
                .context("decoding two-byte ECI assignment value")
        }
        EciValueLength::ThreeBytes => {
            let second_third_bytes = bits
                .read_bits(2 * BYTE_BITS)
                .context("reading ECI second and third bytes")?;
            Eci::try_from(
                ((first_byte & ECI_THREE_BYTE_PAYLOAD_MASK) << (2 * BYTE_BITS))
                    | second_third_bytes,
            )
            .context("decoding three-byte ECI assignment value")
        }
    }
}

/// QR codes encode mode indicators and terminator codes into a constant bit length of 4.
/// IsTerminator peeks into the bit stream to see if the current position is at the start of
/// a terminator code.  If true, then the decoding can finish. If false, then the decoding
/// can read off the next mode code.
///
/// See ISO 18004:2015, 7.4.1 Table 2
///
/// - `bits`: the stream of bits that might have a terminator code
/// - `version`: the QR code version
pub fn is_end_of_stream(bits: &mut BitSource, version: &Version) -> Result<bool> {
    let bits_required = Mode::terminator_bit_length(version);
    let bits_available = std::cmp::min(bits.available(), bits_required);
    Ok(bits_available == 0
        || bits
            .peek_bits(bits_available)
            .context("peeking QR mode terminator bits")?
            == 0)
}

/// QR Codes can encode text as bits in one of several modes, and can use multiple modes
/// in one QR Code. This method decodes the bits back into text.
///
/// See ISO 18004:2006, 6.4.3 - 6.4.7
// ZXING_EXPORT_TEST_ONLY
pub fn decode_bit_stream(bytes: &[u8], version: &Version) -> Result<DecoderResult> {
    let mut bits = BitSource::new(bytes);
    let mut result = ECIStringBuilder::default();
    result.symbology = SymbologyIdentifier {
        code: b'Q',
        modifier: b'1',
        eci_modifier_offset: 1,
        ai_flag: AIFlag::None,
    };
    let mode_bit_length = Mode::codec_mode_bits_length(version);

    let mut modes_seen: Vec<Mode> = Vec::new();
    let mut structured_append: Option<StructuredAppendInfo> = None;

    let res: Result<()> = (|| {
        while !is_end_of_stream(&mut bits, version)? {
            let mode_bits = bits
                .read_bits(mode_bit_length)
                .context("reading QR mode indicator")?;
            let mode = Mode::codec_mode_for_bits(mode_bits)?;
            if !modes_seen.contains(&mode) {
                modes_seen.push(mode);
            }

            match mode {
                Mode::Fnc1FirstPosition => {
                    result.symbology.modifier = b'3';
                    result.symbology.ai_flag = AIFlag::GS1; // In Alphanumeric mode undouble doubled '%' and treat single '%' as <GS>
                }
                Mode::Fnc1SecondPosition => {
                    ensure!(
                        result.is_empty(),
                        Error::invalid_format(
                            "AIM Application Indicator (FNC1 in second position) at illegal position"
                        )
                    );
                    result.symbology.modifier = b'5';
                    // ISO/IEC 18004:2015 7.4.8.3 AIM Application Indicator (FNC1 in second position), "00-99" or "A-Za-z"
                    let indicator = bits
                        .read_bits(BYTE_BITS)
                        .context("reading AIM application indicator")?;
                    match AimApplicationIndicator::try_from(indicator)? {
                        AimApplicationIndicator::Numeric(value) => {
                            result += padded_digits(value, 2)?;
                        }
                        AimApplicationIndicator::Alphabetic(value) => {
                            result += value;
                        }
                    }
                    result.symbology.ai_flag = AIFlag::Aim;
                }
                Mode::StructuredAppend => {
                    let index = bits
                        .read_bits(STRUCTURED_APPEND_INDEX_BITS)
                        .context("reading structured-append index")?;
                    let count = bits
                        .read_bits(STRUCTURED_APPEND_COUNT_BITS)
                        .context("reading structured-append count")?;
                    let parity = bits
                        .read_bits(STRUCTURED_APPEND_PARITY_BITS)
                        .context("reading structured-append parity")?;
                    structured_append = Some(StructuredAppendInfo {
                        index: index as u8,
                        count: count as u8,
                        parity: parity as u8,
                    });
                }
                Mode::Eci => {
                    // Count doesn't apply to ECI
                    result.switch_encoding(parse_eci_value(&mut bits)?.into(), true);
                }
                Mode::Hanzi => {
                    // First handle Hanzi mode which does not start with character count
                    // chinese mode contains a sub set indicator right after mode indicator
                    let subset = bits
                        .read_bits(HANZI_SUBSET_BITS)
                        .context("reading HANZI subset")?;
                    let _subset = HanziSubset::try_from(subset)?;
                    let count = read_count(&mut bits, mode.character_count_bits(version))?;
                    decode_hanzi_segment(&mut bits, count, &mut result)?;
                }
                _ => {
                    // "Normal" QR code modes:
                    // How many characters will follow, encoded in this mode?
                    let count = read_count(&mut bits, mode.character_count_bits(version))?;
                    match mode {
                        Mode::Numeric => decode_numeric_segment(&mut bits, count, &mut result)?,
                        Mode::Alphanumeric => {
                            decode_alphanumeric_segment(&mut bits, count, &mut result)?
                        }
                        Mode::Byte => decode_byte_segment(&mut bits, count, &mut result)?,
                        Mode::Kanji => decode_kanji_segment(&mut bits, count, &mut result)?,
                        other => {
                            bail!(Error::invalid_format(format!(
                                    "Invalid CodecMode {other:?} encountered in data segment (count={count})"
                                )));
                        }
                    };
                }
            }
        }
        Ok(())
    })();

    let mut decoder_result =
        DecoderResult::with_eci_string_builder(result).with_error(res.err());
    decoder_result.set_modes(modes_seen);
    if let Some(info) = structured_append {
        decoder_result.set_structured_append(info);
    }
    Ok(decoder_result)
}

pub fn decode(bits: &BitMatrix) -> Result<DecoderResult> {
    ensure!(
        Version::has_valid_size(bits),
        Error::invalid_format(format!("Invalid QR symbol size: {}x{}", bits.width(), bits.height()))
    );
    let format_info = read_format_information(bits).with_context(|| {
        Error::invalid_format(format!(
            "Invalid format information in {}x{} symbol",
            bits.width(),
            bits.height()
        ))
    })?;

    let version = read_version(bits).with_context(|| {
        Error::invalid_format(format!(
            "Invalid version in {}x{} QR symbol",
            bits.width(),
            bits.height()
        ))
    })?;

    // Read codewords
    let codewords = read_codewords(bits, version, &format_info)
        .context("reading QR codewords from sampled matrix")?;
    ensure!(
        !codewords.is_empty(),
        Error::invalid_format(format!(
                "Failed to read codewords for version {} ({}x{} symbol)",
                version.number(),
                bits.width(),
                bits.height()
            ))
    );

    // Separate into data blocks
    let data_blocks: Vec<DataBlock> =
        DataBlock::data_blocks(&codewords, version, format_info.error_correction_level)
            .context("splitting QR codewords into data blocks")?;
    ensure!(
        !data_blocks.is_empty(),
        Error::invalid_format(format!(
                "Failed to get data blocks for version {} (codewords={}, ec_level={:?})",
                version.number(),
                codewords.len(),
                format_info.error_correction_level
            ))
    );

    // Count total number of data bytes
    let op =
        |total_bytes, data_block: &DataBlock| total_bytes + data_block.num_data_codewords();
    let total_bytes = data_blocks.iter().fold(0, op);
    let mut result_bytes = vec![0u8; total_bytes];
    let mut result_iterator = 0;

    // Error-correct and copy data blocks together into a stream of bytes
    for data_block in data_blocks {
        let num_data_codewords = data_block.num_data_codewords();
        let mut codeword_bytes = data_block.codewords;

        correct_errors(&mut codeword_bytes, num_data_codewords)
            .context("correcting QR data block errors")?;

        result_bytes[result_iterator..(result_iterator + num_data_codewords)]
            .copy_from_slice(&codeword_bytes[..num_data_codewords]);
        result_iterator += num_data_codewords;
    }

    // decode the contents of that stream of bytes
    let decoder_result = decode_bit_stream(&result_bytes, version)
        .context("decoding corrected QR bit stream")?;
    Ok(decoder_result.with_format(
        version.number(),
        format_info.error_correction_level,
        u8::from(format_info.data_mask),
    ))
}

struct DataBlock {
    num_data_codewords: usize,
    codewords: Vec<u8>,
}

impl DataBlock {
    fn new(num_data_codewords: usize, codewords: Vec<u8>) -> Self {
        Self {
            num_data_codewords,
            codewords,
        }
    }

    fn data_blocks(
        raw_codewords: &[u8],
        version: VersionRef,
        ec_level: ErrorCorrectionLevel,
    ) -> Result<Vec<Self>> {
        ensure!(
            raw_codewords.len() == version.total_codewords(),
            Error::invalid_argument(format!(
                    "raw codewords length {} does not match expected total codewords {}",
                    raw_codewords.len(),
                    version.total_codewords()
                ))
        );

        let ec_blocks = version.ec_blocks_for_level(ec_level)
            .context("looking up QR error-correction blocks")?;

        let mut result = Vec::new();
        let mut num_result_blocks = 0;
        for ec_block in ec_blocks.blocks() {
            for _ in 0..ec_block.count() {
                let num_data_codewords = ec_block.data_codewords();
                let num_block_codewords =
                    ec_blocks.ec_codewords_per_block() + num_data_codewords;
                result.push(DataBlock::new(
                    num_data_codewords,
                    vec![0u8; num_block_codewords],
                ));
                num_result_blocks += 1;
            }
        }

        ensure!(
            !result.is_empty(),
            Error::invalid_argument("result block list is empty; possible data corruption or misconfiguration")
        );

        let shorter_blocks_total_codewords = result[0].codewords.len();
        let mut longer_blocks_start_at = result.len() - 1;
        while longer_blocks_start_at > 0 {
            let num_codewords = result[longer_blocks_start_at].codewords.len();

            if num_codewords == shorter_blocks_total_codewords {
                break;
            }
            longer_blocks_start_at -= 1;
        }
        if result[longer_blocks_start_at].codewords.len() == shorter_blocks_total_codewords {
            longer_blocks_start_at += 1;
        }

        let shorter_blocks_num_data_codewords =
            shorter_blocks_total_codewords - ec_blocks.ec_codewords_per_block();
        let mut raw_codewords_offset = 0;
        for i in 0..shorter_blocks_num_data_codewords {
            for result_j in result.iter_mut().take(num_result_blocks) {
                result_j.codewords[i] = raw_codewords[raw_codewords_offset];
                raw_codewords_offset += 1;
            }
        }
        for res in result
            .iter_mut()
            .take(num_result_blocks)
            .skip(longer_blocks_start_at)
        {
            res.codewords[shorter_blocks_num_data_codewords] = raw_codewords[raw_codewords_offset];
            raw_codewords_offset += 1;
        }
        let max = result[0].codewords.len();
        for i in shorter_blocks_num_data_codewords..max {
            for (j, res) in result.iter_mut().enumerate().take(num_result_blocks) {
                let i_offset = if j < longer_blocks_start_at { i } else { i + 1 };
                res.codewords[i_offset] = raw_codewords[raw_codewords_offset];
                raw_codewords_offset += 1;
            }
        }
        Ok(result)
    }

    fn num_data_codewords(&self) -> usize {
        self.num_data_codewords
    }

}

fn correct_errors(codeword_bytes: &mut [u8], num_data_codewords: usize) -> Result<()> {
    correct_qr_errors(codeword_bytes, num_data_codewords)
        .with_context(|| Error::checksum("QR Reed-Solomon error correction failed"))?;
    Ok(())
}
