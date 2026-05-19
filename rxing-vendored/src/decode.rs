use std::collections::HashSet;

use crate::{
    BarcodeFormat, Binarizer, BinaryBitmap, DecodeHints, Luma8LuminanceSource, RXingResult,
    common::{GlobalHistogramBinarizer, HybridBinarizer, Result as RxingResult},
    downscale_luma_buffer,
    qrcode::cpp_port::QrReader,
};

/// Pyramid downscale threshold and factor, mirroring zxing-cpp's
/// `tryDownscale` defaults.
pub const PYRAMID_DOWNSCALE_THRESHOLD: u32 = 500;
pub const PYRAMID_DOWNSCALE_FACTOR: u32 = 3;

pub fn rgba_to_luma(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let expected = (width as usize)
        .checked_mul(height as usize)
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

fn collect_bytes(results: RxingResult<Vec<RXingResult>>) -> Vec<Vec<u8>> {
    results
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.getRawBytes().to_vec())
        .collect()
}

/// Decode on `bitmap` once, then optionally flip the BitMatrix in place and
/// retry when no result was found.
pub fn decode_with_optional_invert<B: Binarizer>(
    bitmap: &mut BinaryBitmap<B>,
    hints: &DecodeHints,
    max_number_of_symbols: u32,
    try_invert: bool,
) -> Vec<Vec<u8>> {
    let results = collect_bytes(QrReader.decode_set_number_with_hints(
        bitmap,
        hints,
        max_number_of_symbols,
    ));
    if !results.is_empty() {
        return results;
    }
    if try_invert
        && let Ok(matrix) = bitmap.get_black_matrix_mut()
    {
        matrix.flip_self();
        return collect_bytes(QrReader.decode_set_number_with_hints(
            bitmap,
            hints,
            max_number_of_symbols,
        ));
    }
    Vec::new()
}

/// Try one resolution and close-pass combination using the selected binarizer.
pub fn decode_one_layer(
    source: Luma8LuminanceSource,
    hints: &DecodeHints,
    use_hybrid_binarizer: bool,
    max_number_of_symbols: u32,
    try_invert: bool,
    close: bool,
) -> Vec<Vec<u8>> {
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

/// Decode QR payload bytes from a luma image through the shared pyramid,
/// close-pass, binarizer, and optional-inversion pipeline.
pub fn decode_inner(
    luma: &[u8],
    width: u32,
    height: u32,
    try_harder: bool,
    try_invert: bool,
    use_hybrid_binarizer: bool,
    max_number_of_symbols: u32,
) -> Vec<Vec<u8>> {
    let hints = DecodeHints {
        possible_formats: Some(HashSet::from([BarcodeFormat::QR_CODE])),
        try_harder: Some(try_harder),
        ..DecodeHints::default()
    };

    if !try_harder {
        let source = Luma8LuminanceSource::new(luma.to_vec(), width, height);
        return decode_one_layer(
            source,
            &hints,
            use_hybrid_binarizer,
            max_number_of_symbols,
            try_invert,
            false,
        );
    }

    let mut cur_luma = luma.to_vec();
    let mut cur_w = width;
    let mut cur_h = height;
    loop {
        for &close in &[false, true] {
            let source = Luma8LuminanceSource::new(cur_luma.clone(), cur_w, cur_h);
            let results = decode_one_layer(
                source,
                &hints,
                use_hybrid_binarizer,
                max_number_of_symbols,
                try_invert,
                close,
            );
            if !results.is_empty() {
                return results;
            }
        }
        if cur_w.max(cur_h) <= PYRAMID_DOWNSCALE_THRESHOLD
            || cur_w.min(cur_h) < PYRAMID_DOWNSCALE_FACTOR
        {
            return Vec::new();
        }
        let (next_luma, next_w, next_h) =
            downscale_luma_buffer(&cur_luma, cur_w, cur_h, PYRAMID_DOWNSCALE_FACTOR);
        cur_luma = next_luma;
        cur_w = next_w;
        cur_h = next_h;
    }
}
