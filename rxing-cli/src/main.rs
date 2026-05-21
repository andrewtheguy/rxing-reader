use std::io::{self, Cursor, Read, Write};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use image::ImageReader;
use rxing_reader::{QrSymbol, decode_qr_codes_luma, rgba_to_luma};
use serde::Serialize;

use crate::json_view::{SymbolView, symbol_to_view};

mod json_view;

const MAX_HTTP_BODY: u64 = 64 * 1024 * 1024;

#[derive(Parser, Debug)]
#[command(
    name = "rxing-cli",
    about = "Decode QR codes from a local image file or URL.",
    version
)]
struct Cli {
    /// Path to an image file, or an http(s):// URL pointing at one.
    source: String,

    /// Output format. `text` prints one payload; `json` prints all detections.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,

    /// JSON-only. Emit each symbol's payload as `bytes_b64` (base64 of
    /// raw bytes) uniformly across every detection, instead of `text`.
    /// Rejected when `--format` is anything other than `json`.
    #[arg(long, default_value_t = false)]
    binary: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Format {
    Text,
    Json,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    if cli.binary && !matches!(cli.format, Format::Json) {
        anyhow::bail!("--binary is only valid with --format json");
    }
    let bytes = load_bytes(&cli.source)?;
    let (rgba, w, h) = decode_image_bytes(&bytes)
        .with_context(|| format!("failed to decode image from {}", cli.source))?;
    let max = match cli.format {
        Format::Text => 1,
        Format::Json => 0,
    };
    let symbols = decode_symbols(&rgba, w, h, max)?;
    render(symbols, cli.format, cli.binary)
}

fn load_bytes(source: &str) -> Result<Vec<u8>> {
    if source.starts_with("http://") || source.starts_with("https://") {
        fetch_url(source)
    } else {
        std::fs::read(source).with_context(|| format!("failed to read file {source}"))
    }
}

fn fetch_url(url: &str) -> Result<Vec<u8>> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build();
    let response = agent
        .get(url)
        .call()
        .with_context(|| format!("HTTP request failed for {url}"))?;
    let mut buf = Vec::new();
    response
        .into_reader()
        .take(MAX_HTTP_BODY + 1)
        .read_to_end(&mut buf)
        .with_context(|| format!("failed to read response body from {url}"))?;
    if buf.len() as u64 > MAX_HTTP_BODY {
        anyhow::bail!("response body from {url} exceeds {MAX_HTTP_BODY} bytes");
    }
    Ok(buf)
}

fn decode_image_bytes(bytes: &[u8]) -> Result<(Vec<u8>, usize, usize)> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("could not guess image format")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader.decode().context("image decode failed")?.into_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    Ok((rgba.into_raw(), w, h))
}

fn decode_symbols(rgba: &[u8], w: usize, h: usize, max: usize) -> Result<Vec<QrSymbol>> {
    let luma = rgba_to_luma(rgba, w, h).context("converting RGBA pixels to luma")?;
    let primary = decode_qr_codes_luma(&luma, w, h, true, true, true, max)?;
    if !primary.is_empty() {
        return Ok(primary);
    }
    let fallback = decode_qr_codes_luma(&luma, w, h, true, true, false, max)?;
    Ok(fallback)
}

fn render(symbols: Vec<QrSymbol>, format: Format, binary: bool) -> Result<ExitCode> {
    match format {
        Format::Text => match symbols.into_iter().next() {
            None => Ok(ExitCode::from(1)),
            Some(symbol) => {
                let text: String = match std::str::from_utf8(&symbol.bytes) {
                    Ok(s) => s.to_string(),
                    Err(_) => symbol.bytes.iter().map(|&b| b as char).collect(),
                };
                println!("{text}");
                Ok(ExitCode::SUCCESS)
            }
        },
        Format::Json => {
            let entries: Vec<SymbolView> = symbols
                .into_iter()
                .map(|s| symbol_to_view(s, binary))
                .collect();
            let mut stdout = io::stdout().lock();
            let mut ser =
                serde_json::Serializer::with_formatter(&mut stdout, AsciiOnlyFormatter);
            entries
                .serialize(&mut ser)
                .context("writing JSON to stdout")?;
            stdout.write_all(b"\n").context("writing trailing newline")?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// `serde_json::Formatter` that escapes any non-ASCII code point as
/// `\uXXXX` (with a surrogate pair for code points above U+FFFF), so the
/// JSON wire output is pure ASCII regardless of payload content —
/// matching Python's `json.dumps(..., ensure_ascii=True)`. The default
/// formatter routes non-ASCII through `write_string_fragment` unchanged,
/// so that's the only override needed; quote/backslash/control escapes
/// retain default behavior.
struct AsciiOnlyFormatter;

impl serde_json::ser::Formatter for AsciiOnlyFormatter {
    fn write_string_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let bytes = fragment.as_bytes();
        let mut start = 0;
        for (idx, ch) in fragment.char_indices() {
            if ch.is_ascii() {
                continue;
            }
            if start < idx {
                writer.write_all(&bytes[start..idx])?;
            }
            let code = ch as u32;
            if code <= 0xFFFF {
                write!(writer, "\\u{code:04x}")?;
            } else {
                let c = code - 0x10000;
                let high = 0xD800 + (c >> 10);
                let low = 0xDC00 + (c & 0x3FF);
                write!(writer, "\\u{high:04x}\\u{low:04x}")?;
            }
            start = idx + ch.len_utf8();
        }
        if start < bytes.len() {
            writer.write_all(&bytes[start..])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_ascii_json<T: Serialize>(value: &T) -> String {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut ser = serde_json::Serializer::with_formatter(&mut buf, AsciiOnlyFormatter);
            value.serialize(&mut ser).unwrap();
        }
        String::from_utf8(buf).unwrap()
    }

    #[derive(Serialize)]
    struct StrField<'a> {
        text: &'a str,
    }

    #[test]
    fn ascii_payload_unchanged() {
        let s = to_ascii_json(&StrField { text: "hello" });
        assert_eq!(s, r#"{"text":"hello"}"#);
        assert!(s.is_ascii());
    }

    #[test]
    fn latin1_supplement_escaped_as_u00xx() {
        let s = to_ascii_json(&StrField { text: "héllo" });
        assert_eq!(s, r#"{"text":"h\u00e9llo"}"#);
        assert!(s.is_ascii());
    }

    #[test]
    fn bmp_non_ascii_escaped() {
        let s = to_ascii_json(&StrField { text: "日本" });
        assert_eq!(s, r#"{"text":"\u65e5\u672c"}"#);
        assert!(s.is_ascii());
    }

    #[test]
    fn supplementary_plane_emits_surrogate_pair() {
        // U+1F600 GRINNING FACE → high D83D, low DE00.
        let s = to_ascii_json(&StrField { text: "\u{1F600}" });
        assert_eq!(s, r#"{"text":"\ud83d\ude00"}"#);
        assert!(s.is_ascii());
    }

    #[test]
    fn control_chars_still_use_default_escapes() {
        // Default formatter behavior for "\n" and '"' must be preserved —
        // our override only handles non-ASCII; quotes/backslash/control
        // escapes come from the trait defaults.
        let s = to_ascii_json(&StrField {
            text: "a\n\"b",
        });
        assert_eq!(s, r#"{"text":"a\n\"b"}"#);
    }
}
