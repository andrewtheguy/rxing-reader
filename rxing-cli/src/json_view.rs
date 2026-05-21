//! Serde-serializable view of a [`QrSymbol`] for emission to JSON.
//!
//! Used by `rxing-cli` to write a JSON array to stdout. The wasm crate
//! does not depend on this module — it returns native JS objects with
//! `bytes: Uint8Array` directly, so text/base64 encoding is a
//! CLI-only concern.
//!
//! Payload representation is controlled per call via the `binary` toggle
//! passed to [`symbol_to_view`]; the choice is uniform across every
//! symbol in the call, so consumers know up front which field to read.
//!
//! - `binary = false` → flattened `text: String`. Valid UTF-8 payloads
//!   decode directly; for invalid UTF-8, each byte is mapped to its
//!   Latin-1 code point (byte = `char`), keeping the round-trip
//!   lossless. The JSON output escapes non-ASCII as `\uXXXX`, so the
//!   wire bytes are pure ASCII regardless of payload content.
//! - `binary = true` → flattened `bytes_b64: String` carrying base64
//!   (STANDARD alphabet) of the raw payload bytes.
//!
//! The on-wire shape (with `binary = false`) is:
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

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rxing_reader::{AIFlag, QrSymbol, StructuredAppendInfo, SymbologyIdentifier};
use serde::Serialize;

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

pub fn symbol_to_view(symbol: QrSymbol, binary: bool) -> SymbolView {
    let payload = if binary {
        PayloadView::BytesB64 {
            bytes_b64: BASE64.encode(&symbol.bytes),
        }
    } else {
        let text = match std::str::from_utf8(&symbol.bytes) {
            Ok(s) => s.to_string(),
            Err(_) => symbol.bytes.iter().map(|&b| b as char).collect(),
        };
        PayloadView::Text { text }
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

#[cfg(test)]
mod tests {
    use super::*;
    use rxing_reader::ErrorCorrectionLevel;

    fn symbol_with_bytes(bytes: Vec<u8>) -> QrSymbol {
        QrSymbol {
            bytes,
            version: 1,
            error_correction_level: ErrorCorrectionLevel::M,
            mask: 0,
            modes: Vec::new(),
            structured_append: None,
            ecis: Vec::new(),
            symbology: SymbologyIdentifier::default(),
        }
    }

    #[test]
    fn text_mode_ascii_payload() {
        let view = symbol_to_view(symbol_with_bytes(b"hello".to_vec()), false);
        match view.payload {
            PayloadView::Text { text } => assert_eq!(text, "hello"),
            PayloadView::BytesB64 { .. } => panic!("expected text variant"),
        }
    }

    #[test]
    fn text_mode_valid_utf8_payload() {
        let view = symbol_to_view(symbol_with_bytes("héllo".as_bytes().to_vec()), false);
        match view.payload {
            PayloadView::Text { text } => assert_eq!(text, "héllo"),
            PayloadView::BytesB64 { .. } => panic!("expected text variant"),
        }
    }

    #[test]
    fn text_mode_invalid_utf8_maps_byte_to_latin1_char() {
        let raw = vec![0xFFu8, 0xFE, 0x00, b'A'];
        let view = symbol_to_view(symbol_with_bytes(raw.clone()), false);
        let text = match view.payload {
            PayloadView::Text { text } => text,
            PayloadView::BytesB64 { .. } => panic!("expected text variant"),
        };
        let chars: Vec<u32> = text.chars().map(|c| c as u32).collect();
        let expected: Vec<u32> = raw.iter().map(|&b| b as u32).collect();
        assert_eq!(chars, expected);
    }

    #[test]
    fn binary_mode_emits_base64_for_ascii_payload() {
        let view = symbol_to_view(symbol_with_bytes(b"hello".to_vec()), true);
        match view.payload {
            PayloadView::BytesB64 { bytes_b64 } => assert_eq!(bytes_b64, "aGVsbG8="),
            PayloadView::Text { .. } => panic!("expected bytes_b64 variant"),
        }
    }

    #[test]
    fn binary_mode_emits_base64_for_invalid_utf8() {
        let raw = vec![0xFFu8, 0xFE, 0x00, b'A'];
        let view = symbol_to_view(symbol_with_bytes(raw.clone()), true);
        match view.payload {
            PayloadView::BytesB64 { bytes_b64 } => {
                assert_eq!(bytes_b64, BASE64.encode(&raw));
            }
            PayloadView::Text { .. } => panic!("expected bytes_b64 variant"),
        }
    }
}
