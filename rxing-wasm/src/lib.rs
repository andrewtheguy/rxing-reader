use rxing::decode::{decode_inner, rgba_to_luma};
use wasm_bindgen::prelude::*;

/// Run the decode pipeline once with `use_hybrid_binarizer`, then — when
/// `binarizer_fallback` is set and the first pass produced nothing — once
/// more with the opposite binarizer. The two rxing binarizers fail on
/// disjoint inputs (Hybrid loses on stylized clean-bg QRs with colored
/// finders; Global loses on uneven illumination), so retrying the other
/// rescues both failure modes at the cost of an extra full pipeline pass.
/// Default for the wasm export is `binarizer_fallback = false`, matching
/// upstream zxing-wasm which picks a single binarizer per call.
#[allow(clippy::too_many_arguments)] // mirrors the wasm export signature 1:1
fn read_inner(
    luma: Vec<u8>,
    width: u32,
    height: u32,
    try_harder: bool,
    try_invert: bool,
    use_hybrid_binarizer: bool,
    binarizer_fallback: bool,
    max_number_of_symbols: u32,
) -> Vec<Vec<u8>> {
    let primary = decode_inner(
        &luma,
        width,
        height,
        try_harder,
        try_invert,
        use_hybrid_binarizer,
        max_number_of_symbols,
    );
    if !primary.is_empty() || !binarizer_fallback {
        return primary;
    }
    decode_inner(
        &luma,
        width,
        height,
        try_harder,
        try_invert,
        !use_hybrid_binarizer,
        max_number_of_symbols,
    )
}

/// Read every QR code in raw RGBA pixels, returning each payload's raw bytes.
///
/// - `rgba`: row-major RGBA pixels, length must equal `width * height * 4`
/// - `try_harder`: spend more time looking for a barcode. Walks a downscale
///   pyramid (factor 3, threshold 500 px) and a 3×3 morphological-close
///   pass per layer, in addition to densifying the finder-pattern scan via
///   rxing's `TryHarder` hint. Equivalent to zxing-wasm's
///   `try_harder + tryDownscale + tryDenoise`.
/// - `try_invert`: retry with the BitMatrix flipped if the first pass yields
///   no results (covers white-on-dark / inverted-reflectance codes).
/// - `use_hybrid_binarizer`: when `true`, use rxing's adaptive
///   `HybridBinarizer`; when `false`, the faster but less robust
///   `GlobalHistogramBinarizer`.
/// - `binarizer_fallback`: when `true` and the primary binarizer produces no
///   results, retry the full pipeline once more with the opposite binarizer.
///   The two binarizers fail on disjoint inputs (Hybrid on stylized clean-bg
///   QRs with colored finders, Global on uneven illumination), so fallback
///   rescues both at the cost of an extra full pipeline pass. Set `false` for
///   battery-critical live scanning, `true` for one-shot image-upload paths.
/// - `max_number_of_symbols`: cap the number of symbols returned per pass.
///   Pass `0` to remove the cap. Pass `1` when only one detection is needed —
///   lets the multi-decode loop short-circuit on the first valid result and
///   skips micro QR / r_mqr fallbacks once a QR is found.
///
/// Retry order when no results: original × invert →
/// (try_harder: original-closed × invert → downscale 1× → 1× closed →
/// downscale 2× → 2× closed → …). The first pass that produces results
/// wins; remaining passes are skipped.
///
/// Rotation is handled natively by rxing's finder-pattern canonical
/// reordering (`detector.rs:139-263`), so no explicit rotation-retry flag
/// is exposed — empirical testing showed the (now-removed) `try_rotate`
/// loop produced zero additional decodes on every fixture in the test
/// suite. Cameras held sideways / upside-down decode identically to
/// upright captures.
///
/// Returns a JS `Array` of `Uint8Array`, one per detected symbol (empty when
/// none are found). Returns `Err` only for invalid input (e.g. mismatched
/// buffer length). Callers that need a string must decode the bytes
/// themselves (e.g. `new TextDecoder().decode(bytes)`).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)] // wasm-bindgen call shape; packing into a struct hurts the JS side
pub fn read_qr_codes_rgba(
    rgba: &[u8],
    width: u32,
    height: u32,
    try_harder: bool,
    try_invert: bool,
    use_hybrid_binarizer: bool,
    binarizer_fallback: bool,
    max_number_of_symbols: u32,
) -> Result<js_sys::Array, JsValue> {
    let luma = rgba_to_luma(rgba, width, height).map_err(|m| JsValue::from_str(&m))?;
    let payloads = read_inner(
        luma,
        width,
        height,
        try_harder,
        try_invert,
        use_hybrid_binarizer,
        binarizer_fallback,
        max_number_of_symbols,
    );

    let out = js_sys::Array::new_with_length(payloads.len() as u32);
    for (i, bytes) in payloads.into_iter().enumerate() {
        out.set(i as u32, js_sys::Uint8Array::from(bytes.as_slice()).into());
    }
    Ok(out)
}
