use std::borrow::Cow;

use anyhow::Result as RxingResult;

use crate::{
    Binarizer, BinaryBitmap, DecodeHints, Luma8LuminanceSource,
    common::{
        Eci, GlobalHistogramBinarizer, HybridBinarizer, SymbologyIdentifier,
        detect::{DecoderResult, StructuredAppendInfo},
    },
    downscale_luma_buffer,
    qrcode::{ErrorCorrectionLevel, Mode, QrReader},
};

/// Pyramid downscale threshold and factor used by try-harder QR scanning.
pub const PYRAMID_DOWNSCALE_THRESHOLD: usize = 500;
pub const PYRAMID_DOWNSCALE_FACTOR: usize = 3;

/// Per-symbol output from the QR decode pipeline: payload bytes plus the
/// metadata extracted from the QR symbol (version, EC level, mask, the data
/// modes encountered, structured-append header when present, ECIs invoked,
/// and the symbology identifier the decoder produced).
#[derive(Debug, Clone)]
pub struct QrSymbol {
    pub bytes: Vec<u8>,
    pub version: u32,
    pub error_correction_level: ErrorCorrectionLevel,
    pub mask: u8,
    pub modes: Vec<Mode>,
    pub structured_append: Option<StructuredAppendInfo>,
    pub ecis: Vec<Eci>,
    pub symbology: SymbologyIdentifier,
}

impl QrSymbol {
    /// Move the metadata + payload out of a finished `DecoderResult`. Returns
    /// `None` when the result is invalid or missing required QR metadata
    /// (e.g. the EC level was never plumbed in).
    pub(crate) fn from_decoder_result(decoder_result: DecoderResult) -> Option<Self> {
        if !decoder_result.is_valid() {
            return None;
        }
        let version = decoder_result.version();
        let error_correction_level = decoder_result.error_correction_level()?;
        let mask = decoder_result.mask();
        let modes = decoder_result.modes().to_vec();
        let structured_append = decoder_result.structured_append();
        let content = decoder_result.into_content();
        let symbology = content.symbology;
        let ecis: Vec<Eci> = content
            .eci_positions
            .iter()
            .map(|(eci, _, _)| *eci)
            .collect();
        let bytes = content.into_bytes();
        Some(QrSymbol {
            bytes,
            version,
            error_correction_level,
            mask,
            modes,
            structured_append,
            ecis,
            symbology,
        })
    }
}

pub fn rgba_to_luma(rgba: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    let expected = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "Image dimensions overflow".to_string())?;
    if rgba.len() != expected {
        return Err(format!(
            "rgba length {} != width*height*4 ({})",
            rgba.len(),
            expected
        ));
    }
    Ok(rgba
        .chunks_exact(4)
        .map(|p| {
            // ITU-R BT.601 luma weights (rounded).
            let r = p[0] as u32;
            let g = p[1] as u32;
            let b = p[2] as u32;
            ((r * 299 + g * 587 + b * 114 + 500) / 1000) as u8
        })
        .collect())
}

/// decode on `bitmap` once, then optionally flip the BitMatrix in place and
/// retry when no result was found.
pub fn decode_with_optional_invert<B: Binarizer>(
    bitmap: &mut BinaryBitmap<B>,
    hints: &DecodeHints,
    max_number_of_symbols: usize,
    try_invert: bool,
) -> Vec<QrSymbol> {
    let results = QrReader
        .decode_with_hints(bitmap, hints, max_number_of_symbols)
        .unwrap_or_default();
    if !results.is_empty() {
        return results;
    }
    if try_invert && let Ok(matrix) = bitmap.black_matrix_mut() {
        matrix.flip_self();
        return QrReader
            .decode_with_hints(bitmap, hints, max_number_of_symbols)
            .unwrap_or_default();
    }
    Vec::new()
}

/// Try one resolution and close-pass combination using the selected binarizer.
pub fn decode_one_layer(
    source: Luma8LuminanceSource<'_>,
    hints: &DecodeHints,
    use_hybrid_binarizer: bool,
    max_number_of_symbols: usize,
    try_invert: bool,
    close: bool,
) -> Vec<QrSymbol> {
    if use_hybrid_binarizer {
        let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));
        if close && bitmap.close().is_err() {
            return Vec::new();
        }
        decode_with_optional_invert(&mut bitmap, hints, max_number_of_symbols, try_invert)
    } else {
        let mut bitmap = BinaryBitmap::new(GlobalHistogramBinarizer::new(source));
        if close && bitmap.close().is_err() {
            return Vec::new();
        }
        decode_with_optional_invert(&mut bitmap, hints, max_number_of_symbols, try_invert)
    }
}

/// Decode QR symbols from a luma image through the shared pyramid,
/// close-pass, binarizer, and optional-inversion pipeline. Returns one
/// [`QrSymbol`] per detected QR code, each carrying the payload bytes
/// alongside version/EC/mask/modes metadata.
pub fn decode_qr_codes_luma(
    luma: &[u8],
    width: usize,
    height: usize,
    try_harder: bool,
    try_invert: bool,
    use_hybrid_binarizer: bool,
    max_number_of_symbols: usize,
) -> RxingResult<Vec<QrSymbol>> {
    let hints = DecodeHints { try_harder };

    if !try_harder {
        let source = Luma8LuminanceSource::new(luma, width, height)?;
        return Ok(decode_one_layer(
            source,
            &hints,
            use_hybrid_binarizer,
            max_number_of_symbols,
            try_invert,
            false,
        ));
    }

    let mut cur_luma = Cow::Borrowed(luma);
    let mut cur_w = width;
    let mut cur_h = height;
    loop {
        for &close in &[false, true] {
            let source = Luma8LuminanceSource::new(cur_luma.as_ref(), cur_w, cur_h)?;
            let results = decode_one_layer(
                source,
                &hints,
                use_hybrid_binarizer,
                max_number_of_symbols,
                try_invert,
                close,
            );
            if !results.is_empty() {
                return Ok(results);
            }
        }
        if cur_w.max(cur_h) <= PYRAMID_DOWNSCALE_THRESHOLD
            || cur_w.min(cur_h) < PYRAMID_DOWNSCALE_FACTOR
        {
            return Ok(Vec::new());
        }
        let (next_luma, next_w, next_h) =
            downscale_luma_buffer(cur_luma.as_ref(), cur_w, cur_h, PYRAMID_DOWNSCALE_FACTOR)?;
        if let Cow::Owned(next_luma) = next_luma {
            cur_luma = Cow::Owned(next_luma);
        }
        cur_w = next_w;
        cur_h = next_h;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        QrSymbol,
        common::detect::DecoderResult,
        qrcode::ErrorCorrectionLevel,
    };

    #[test]
    fn rejects_invalid_decoder_result_even_when_format_metadata_exists() {
        let decoder_result = DecoderResult::default().with_format(1, ErrorCorrectionLevel::L, 0);

        assert!(QrSymbol::from_decoder_result(decoder_result).is_none());
    }
}
