mod bitmatrix_parser;
mod decoder;
mod detector;

use crate::{
    Binarizer, BinaryBitmap, DecodeHints,
    common::detect::ConcentricPattern,
};

use self::{
    decoder::decode,
    detector::{find_finder_patterns, generate_finder_pattern_sets, sample_qr},
};

#[derive(Default)]
pub(crate) struct QrReader;

impl QrReader {
    /// Decode every QR symbol found in `image`.
    ///
    /// `max_symbols` caps the number of results; pass `0` for unlimited.
    pub(crate) fn decode_with_hints<B: Binarizer>(
        &self,
        image: &mut BinaryBitmap<B>,
        hints: &DecodeHints,
        max_symbols: usize,
    ) -> anyhow::Result<Vec<Vec<u8>>> {
        let bin_img = image.black_matrix()?;
        let try_harder = hints.try_harder;

        let mut all_fps = find_finder_patterns(bin_img, try_harder);

        let mut used_fps: Vec<ConcentricPattern> = Vec::new();
        let mut results: Vec<Vec<u8>> = Vec::new();

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
                let decoder_result = decode(detector_result.bits());
                if let Ok(decoder_result) = decoder_result
                    && decoder_result.is_valid()
                {
                    used_fps.push(fp_set.bl);
                    used_fps.push(fp_set.tl);
                    used_fps.push(fp_set.tr);

                    results.push(decoder_result.into_bytes());

                    if max_symbols != 0 && results.len() == max_symbols {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }
}