//! Serde-serializable view of a [`QrSymbol`] for emission to JSON / JS.
//!
//! Shared by `rxing-cli` (writes a JSON array to stdout) and `rxing-wasm`
//! (serializes to a JS array of objects via `serde-wasm-bindgen`). Both
//! consumers enable rxing-reader's `serde` feature.
//!
//! The on-wire shape is:
//! ```json
//! {
//!   "version": 4,
//!   "error_correction_level": "M",
//!   "mask": 3,
//!   "modes": ["Byte"],
//!   "structured_append": { "index": 0, "count": 2, "parity": 17 },
//!   "ecis": ["UTF8"],
//!   "symbology": { "code": "Q", "modifier": "1", "ai_flag": "None" },
//!   "text": "hello"
//! }
//! ```
//! Exactly one of `text` (UTF-8 valid payload) or `bytes_b64` (otherwise) is
//! present, flattened into the parent object.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Serialize;

use crate::{AIFlag, QrSymbol, StructuredAppendInfo, SymbologyIdentifier};

#[derive(Serialize)]
pub struct SymbolView {
    pub version: u32,
    pub error_correction_level: String,
    pub mask: u8,
    pub modes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_append: Option<StructuredAppendView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ecis: Vec<String>,
    pub symbology: SymbologyView,
    #[serde(flatten)]
    pub payload: PayloadView,
}

#[derive(Serialize)]
pub struct StructuredAppendView {
    pub index: u8,
    pub count: u8,
    pub parity: u8,
}

#[derive(Serialize)]
pub struct SymbologyView {
    pub code: String,
    pub modifier: String,
    pub ai_flag: &'static str,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum PayloadView {
    Text { text: String },
    BytesB64 { bytes_b64: String },
}

pub fn symbol_to_view(symbol: QrSymbol) -> SymbolView {
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

pub fn sa_view(info: StructuredAppendInfo) -> StructuredAppendView {
    StructuredAppendView {
        index: info.index,
        count: info.count,
        parity: info.parity,
    }
}

pub fn symbology_view(sym: &SymbologyIdentifier) -> SymbologyView {
    SymbologyView {
        code: ascii_byte_string(sym.code),
        modifier: ascii_byte_string(sym.modifier),
        ai_flag: ai_flag_str(sym.ai_flag),
    }
}

pub fn ascii_byte_string(b: u8) -> String {
    if b == 0 {
        String::new()
    } else {
        String::from(b as char)
    }
}

pub fn ai_flag_str(f: AIFlag) -> &'static str {
    match f {
        AIFlag::None => "None",
        AIFlag::GS1 => "GS1",
        AIFlag::Aim => "Aim",
    }
}
