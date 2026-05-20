/*
* Copyright 2016 Nu-book Inc.
* Copyright 2016 ZXing authors
* Copyright 2022 Axel Waggershauser
*/
// SPDX-License-Identifier: Apache-2.0

use crate::{
    BarcodeFormat, DecodeHints, RXingResult,
    common::{DetectorRXingResult, cpp_essentials::ConcentricPattern},
};

use super::{
    decoder::decode,
    detector::{find_finder_patterns, generate_finder_pattern_sets, sample_mqr, sample_qr, sample_rmqr},
};

#[derive(Default)]
pub struct QrReader;

impl QrReader {
    /// decode every QR / micro QR / r_mqr symbol found in `image`.
    ///
    /// `count` caps the number of results; pass `0` for unlimited.
    pub fn decode_set_number_with_hints<B: crate::Binarizer>(
        &self,
        image: &mut crate::BinaryBitmap<B>,
        hints: &DecodeHints,
        count: u32,
    ) -> anyhow::Result<Vec<RXingResult>> {
        let bin_img = image.get_black_matrix()?;
        let max_symbols = count;
        let try_harder = hints.try_harder.unwrap_or(false);

        let mut all_fps = find_finder_patterns(bin_img, try_harder);

        let mut used_fps: Vec<ConcentricPattern> = Vec::new();
        let mut results: Vec<RXingResult> = Vec::new();

        let (check_qr, check_mqr, check_rmqr) = if let Some(formats) = &hints.possible_formats {
            (
                formats.contains(&BarcodeFormat::QrCode),
                formats.contains(&BarcodeFormat::MicroQrCode),
                formats.contains(&BarcodeFormat::RectangularMicroQrCode),
            )
        } else {
            (true, true, true)
        };

        if check_qr {
            let all_fpsets = generate_finder_pattern_sets(&mut all_fps);
            for fp_set in all_fpsets {
                if used_fps.contains(&fp_set.bl)
                    || used_fps.contains(&fp_set.tl)
                    || used_fps.contains(&fp_set.tr)
                {
                    continue;
                }

                let detector_result = sample_qr(bin_img, &fp_set);
                if let Ok(detector_result) = detector_result {
                    let decoder_result = decode(detector_result.get_bits());
                    let position = detector_result.get_points();
                    if let Ok(decoder_result) = decoder_result
                        && decoder_result.is_valid()
                    {
                        used_fps.push(fp_set.bl);
                        used_fps.push(fp_set.tl);
                        used_fps.push(fp_set.tr);

                        results.push(RXingResult::with_decoder_result_bytes_only(
                            decoder_result,
                            position,
                            BarcodeFormat::QrCode,
                        ));

                        if max_symbols != 0 && (results.len() as u32) == max_symbols {
                            break;
                        }
                    }
                }
            }
        }
        if check_mqr && !(max_symbols != 0 && (results.len() as u32) == max_symbols) {
            for fp in &all_fps {
                if used_fps.contains(fp) {
                    continue;
                }

                let detector_result = sample_mqr(bin_img, *fp);
                if let Ok(detector_result) = detector_result {
                    let decoder_result = decode(detector_result.get_bits());
                    let position = detector_result.get_points();
                    if let Ok(decoder_result) = decoder_result
                        && decoder_result.is_valid()
                    {
                        results.push(RXingResult::with_decoder_result_bytes_only(
                            decoder_result,
                            position,
                            BarcodeFormat::MicroQrCode,
                        ));

                        if max_symbols != 0 && (results.len() as u32) == max_symbols {
                            break;
                        }
                    }
                }
            }
        }
        if check_rmqr && !(max_symbols != 0 && (results.len() as u32) == max_symbols) {
            for fp in &all_fps {
                if used_fps.contains(fp) {
                    continue;
                }

                let detector_result = sample_rmqr(bin_img, *fp);
                if let Ok(detector_result) = detector_result {
                    let decoder_result = decode(detector_result.get_bits());
                    let position = detector_result.get_points();
                    if let Ok(decoder_result) = decoder_result
                        && decoder_result.is_valid()
                    {
                        results.push(RXingResult::with_decoder_result_bytes_only(
                            decoder_result,
                            position,
                            BarcodeFormat::RectangularMicroQrCode,
                        ));

                        if max_symbols != 0 && (results.len() as u32) == max_symbols {
                            break;
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}
