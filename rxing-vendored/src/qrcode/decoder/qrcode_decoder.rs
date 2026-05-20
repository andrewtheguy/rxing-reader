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

use std::sync::Arc;

use anyhow::Result;

/**
 * <p>The main class which implements QR Code decoding -- as opposed to locating and extracting
 * the QR Code from an image.</p>
 *
 * @author Sean Owen
 */
use crate::{
    DecodeHints, Error,
    common::{BitMatrix, DecoderRXingResult},
};

use super::{BitMatrixParser, DataBlock, QRCodeDecoderMetaData, decoded_bit_stream_parser};

pub fn decode_bool_array(image: &[Vec<bool>]) -> Result<DecoderRXingResult> {
    decode_bool_array_with_hints(image, &DecodeHints::default())
}

/**
 * <p>Convenience method that can decode a QR Code represented as a 2D array of booleans.
 * "true" is taken to mean a black module.</p>
 *
 * @param image booleans representing white/black QR Code modules
 * @param hints decoding hints that should be used to influence decoding
 * @return text and bytes encoded within the QR Code
 * Returns an invalid-format error if the QR Code cannot be decoded
 * Returns a checksum error if error correction fails
 */
pub fn decode_bool_array_with_hints(
    image: &[Vec<bool>],
    hints: &DecodeHints,
) -> Result<DecoderRXingResult> {
    decode_bitmatrix_with_hints(&BitMatrix::parse_bools(image), hints)
}

pub fn decode_bitmatrix(bits: &BitMatrix) -> Result<DecoderRXingResult> {
    decode_bitmatrix_with_hints(bits, &DecodeHints::default())
}

/**
 * <p>Decodes a QR Code represented as a {@link BitMatrix}. A 1 or "true" is taken to mean a black module.</p>
 *
 * @param bits booleans representing white/black QR Code modules
 * @param hints decoding hints that should be used to influence decoding
 * @return text and bytes encoded within the QR Code
 * Returns an invalid-format error if the QR Code cannot be decoded
 * Returns a checksum error if error correction fails
 */
pub fn decode_bitmatrix_with_hints(
    bits: &BitMatrix,
    hints: &DecodeHints,
) -> Result<DecoderRXingResult> {
    // Construct a parser and read version, error-correction level
    let mut parser = BitMatrixParser::new(bits.clone())?;
    let mut fe = None;
    let mut ce = None;
    match decode_bitmatrix_parser_with_hints(&mut parser, hints) {
        Ok(ok) => return Ok(ok),
        Err(er) if is_invalid_format(&er) => fe = Some(er),
        Err(er) if is_checksum(&er) => ce = Some(er),
        Err(er) => return Err(er),
    }

    let mut trying = || -> Result<DecoderRXingResult> {
        // Revert the bit matrix
        parser.remask()?;

        // Will be attempting a mirrored reading of the version and format info.
        parser.set_mirror(true);

        // Preemptively read the version.
        parser.read_version()?;

        // Preemptively read the format information.
        parser.read_format_information()?;

        /*
         * Since we're here, this means we have successfully detected some kind
         * of version and format information when mirrored. This is a good sign,
         * that the QR code may be mirrored, and we should try once more with a
         * mirrored content.
         */
        // Prepare for a mirrored reading.
        parser.mirror();

        let mut result = decode_bitmatrix_parser_with_hints(&mut parser, hints)?;

        // Success! Notify the caller that the code was mirrored.
        result.set_other(Some(Arc::new(QRCodeDecoderMetaData::new(true))));

        Ok(result)
    };

    match trying() {
        Ok(res) => Ok(res),
        Err(er) if is_retryable_decode_error(&er) => {
            if let Some(fe) = fe {
                Err(fe)
            } else {
                Err(ce.unwrap_or_else(|| Error::Checksum(None).into()))
            }
        }
        Err(er) => Err(er),
    }
}

fn is_invalid_format(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<Error>()
        .is_some_and(Error::is_invalid_format)
}

fn is_checksum(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<Error>()
        .is_some_and(Error::is_checksum)
}

fn is_retryable_decode_error(error: &anyhow::Error) -> bool {
    is_invalid_format(error) || is_checksum(error)
}

fn decode_bitmatrix_parser_with_hints(
    parser: &mut BitMatrixParser,
    hints: &DecodeHints,
) -> Result<DecoderRXingResult> {
    let version = parser.read_version()?;
    let ec_level = parser
        .read_format_information()?
        .get_error_correction_level();

    // Read codewords
    let codewords = parser.read_codewords()?;
    // Separate into data blocks
    let data_blocks = DataBlock::get_data_blocks(&codewords, version, ec_level)?;

    // Count total number of data bytes
    let total_bytes = data_blocks.iter().fold(0, |acc, data_block| {
        acc + data_block.get_num_data_codewords() as usize
    });

    let mut result_bytes = vec![0u8; total_bytes];
    let mut result_offset = 0;

    // Error-correct and copy data blocks together into a stream of bytes
    for data_block in &data_blocks {
        let mut codeword_bytes = data_block.get_codewords().to_vec();
        let num_data_codewords = data_block.get_num_data_codewords() as usize;
        correct_errors(&mut codeword_bytes, num_data_codewords)?;
        for codeword_byte in codeword_bytes.iter().take(num_data_codewords) {
            result_bytes[result_offset] = *codeword_byte;
            result_offset += 1;
        }
    }

    // decode the contents of that stream of bytes
    decoded_bit_stream_parser::decode(&result_bytes, version, ec_level, hints)
}

/**
 * <p>Given data and error-correction codewords received, possibly corrupted by errors, attempts to
 * correct the errors in-place using Reed-Solomon error correction.</p>
 *
 * @param codeword_bytes data and error correction codewords
 * @param num_data_codewords number of codewords that are data bytes
 * Returns a checksum error if error correction fails
 */
pub(crate) fn correct_errors(codeword_bytes: &mut [u8], num_data_codewords: usize) -> Result<()> {
    let ecc_len = codeword_bytes.len() - num_data_codewords;
    let buf = reed_solomon::Decoder::new(ecc_len)
        .correct(codeword_bytes, None)
        .map_err(|e| Error::checksum(format!("{e:?}")))?;
    codeword_bytes[..num_data_codewords].copy_from_slice(buf.data());
    Ok(())
}
