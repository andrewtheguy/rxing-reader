use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rxing_reader::{
    AIFlag, QrSymbol, StructuredAppendInfo, SymbologyIdentifier, decode_qr_codes_luma,
    rgba_to_luma,
};
use serde::Serialize;
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
fn read_luma(
    luma: Vec<u8>,
    width: usize,
    height: usize,
    try_harder: bool,
    try_invert: bool,
    use_hybrid_binarizer: bool,
    binarizer_fallback: bool,
    max_number_of_symbols: usize,
) -> Result<Vec<QrSymbol>, JsValue> {
    let primary = decode_qr_codes_luma(
        &luma,
        width,
        height,
        try_harder,
        try_invert,
        use_hybrid_binarizer,
        max_number_of_symbols,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    if !primary.is_empty() || !binarizer_fallback {
        return Ok(primary);
    }
    decode_qr_codes_luma(
        &luma,
        width,
        height,
        try_harder,
        try_invert,
        !use_hybrid_binarizer,
        max_number_of_symbols,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))
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
/// themselves (e.g. `new TextDecoder().decode(bytes)`). For per-symbol
/// metadata (version, EC level, mask, modes, etc.) use
/// [`read_qr_codes_rgba_detailed`].
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
    let width = width as usize;
    let height = height as usize;
    let max_number_of_symbols = max_number_of_symbols as usize;
    let luma = rgba_to_luma(rgba, width, height).map_err(|m| JsValue::from_str(&m))?;
    let symbols = read_luma(
        luma,
        width,
        height,
        try_harder,
        try_invert,
        use_hybrid_binarizer,
        binarizer_fallback,
        max_number_of_symbols,
    )?;

    let out = js_sys::Array::new_with_length(symbols.len() as u32);
    for (i, symbol) in symbols.into_iter().enumerate() {
        out.set(i as u32, js_sys::Uint8Array::from(symbol.bytes.as_slice()).into());
    }
    Ok(out)
}

#[derive(Serialize)]
struct SymbolView {
    version: u32,
    error_correction_level: String,
    mask: u8,
    modes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    structured_append: Option<StructuredAppendView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ecis: Vec<String>,
    symbology: SymbologyView,
    #[serde(flatten)]
    payload: PayloadView,
}

#[derive(Serialize)]
struct StructuredAppendView {
    index: u8,
    count: u8,
    parity: u8,
}

#[derive(Serialize)]
struct SymbologyView {
    code: String,
    modifier: String,
    ai_flag: &'static str,
}

#[derive(Serialize)]
#[serde(untagged)]
enum PayloadView {
    Text { text: String },
    BytesB64 { bytes_b64: String },
}

fn symbol_to_view(symbol: QrSymbol) -> SymbolView {
    let payload = match std::str::from_utf8(&symbol.bytes) {
        Ok(s) => PayloadView::Text { text: s.to_string() },
        Err(_) => PayloadView::BytesB64 {
            bytes_b64: BASE64.encode(&symbol.bytes),
        },
    };
    SymbolView {
        version: symbol.version,
        error_correction_level: symbol.error_correction_level.to_string(),
        mask: symbol.mask,
        modes: symbol.modes.iter().map(|m| m.to_string()).collect(),
        structured_append: symbol.structured_append.map(sa_view),
        ecis: symbol.ecis.iter().map(|e| e.to_string()).collect(),
        symbology: symbology_view(&symbol.symbology),
        payload,
    }
}

fn sa_view(info: StructuredAppendInfo) -> StructuredAppendView {
    StructuredAppendView {
        index: info.index,
        count: info.count,
        parity: info.parity,
    }
}

fn symbology_view(sym: &SymbologyIdentifier) -> SymbologyView {
    SymbologyView {
        code: ascii_byte_string(sym.code),
        modifier: ascii_byte_string(sym.modifier),
        ai_flag: ai_flag_str(sym.ai_flag),
    }
}

fn ascii_byte_string(b: u8) -> String {
    if b == 0 {
        String::new()
    } else {
        String::from(b as char)
    }
}

fn ai_flag_str(f: AIFlag) -> &'static str {
    match f {
        AIFlag::None => "None",
        AIFlag::GS1 => "GS1",
        AIFlag::Aim => "Aim",
    }
}

/// Read every QR code in raw RGBA pixels, returning a JS `Array` of objects
/// — one per detected symbol — carrying both the payload and the QR
/// metadata extracted during decoding.
///
/// Each entry has the shape:
/// ```js
/// {
///   version: 4,                       // 1..=40
///   error_correction_level: "M",       // "L"|"M"|"Q"|"H"
///   mask: 3,                          // 0..=7
///   modes: ["Byte"],                  // unique data modes in encounter order
///   structured_append: { index, count, parity } | undefined,
///   ecis: ["UTF8", ...],
///   symbology: { code: "Q", modifier: "1", ai_flag: "None" },
///   // exactly one of:
///   text: "hello"                     // when the payload decodes as UTF-8
///   // or:
///   bytes_b64: "..."                  // when it does not
/// }
/// ```
/// Argument semantics match [`read_qr_codes_rgba`].
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn read_qr_codes_rgba_detailed(
    rgba: &[u8],
    width: u32,
    height: u32,
    try_harder: bool,
    try_invert: bool,
    use_hybrid_binarizer: bool,
    binarizer_fallback: bool,
    max_number_of_symbols: u32,
) -> Result<JsValue, JsValue> {
    let width = width as usize;
    let height = height as usize;
    let max_number_of_symbols = max_number_of_symbols as usize;
    let luma = rgba_to_luma(rgba, width, height).map_err(|m| JsValue::from_str(&m))?;
    let symbols = read_luma(
        luma,
        width,
        height,
        try_harder,
        try_invert,
        use_hybrid_binarizer,
        binarizer_fallback,
        max_number_of_symbols,
    )?;

    let views: Vec<SymbolView> = symbols.into_iter().map(symbol_to_view).collect();
    serde_wasm_bindgen::to_value(&views).map_err(|e| JsValue::from_str(&e.to_string()))
}
