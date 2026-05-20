use std::io::{self, Cursor, Read, Write};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use clap::{Parser, ValueEnum};
use image::ImageReader;
use rxing_reader::{
    AIFlag, QrSymbol, StructuredAppendInfo, SymbologyIdentifier, decode_qr_codes_luma,
    rgba_to_luma,
};
use serde::Serialize;

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
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Format {
    Text,
    Json,
}

#[derive(Serialize)]
struct SymbolJson {
    version: u32,
    error_correction_level: String,
    mask: u8,
    modes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    structured_append: Option<StructuredAppendJson>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ecis: Vec<String>,
    symbology: SymbologyJson,
    #[serde(flatten)]
    payload: PayloadJson,
}

#[derive(Serialize)]
struct StructuredAppendJson {
    index: u8,
    count: u8,
    parity: u8,
}

#[derive(Serialize)]
struct SymbologyJson {
    code: String,
    modifier: String,
    ai_flag: &'static str,
}

#[derive(Serialize)]
#[serde(untagged)]
enum PayloadJson {
    Text { text: String },
    BytesB64 { bytes_b64: String },
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
    let bytes = load_bytes(&cli.source)?;
    let (rgba, w, h) = decode_image_bytes(&bytes)
        .with_context(|| format!("failed to decode image from {}", cli.source))?;
    let max = match cli.format {
        Format::Text => 1,
        Format::Json => 0,
    };
    let symbols = decode_symbols(&rgba, w, h, max)?;
    render(&symbols, cli.format)
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
    let luma = rgba_to_luma(rgba, w, h).map_err(anyhow::Error::msg)?;
    let primary = decode_qr_codes_luma(&luma, w, h, true, true, true, max)?;
    if !primary.is_empty() {
        return Ok(primary);
    }
    let fallback = decode_qr_codes_luma(&luma, w, h, true, true, false, max)?;
    Ok(fallback)
}

fn symbol_to_json(symbol: &QrSymbol) -> SymbolJson {
    SymbolJson {
        version: symbol.version,
        error_correction_level: symbol.error_correction_level.to_string(),
        mask: symbol.mask,
        modes: symbol.modes.iter().map(|m| m.to_string()).collect(),
        structured_append: symbol.structured_append.map(structured_append_json),
        ecis: symbol.ecis.iter().map(|e| e.to_string()).collect(),
        symbology: symbology_json(&symbol.symbology),
        payload: payload_json(&symbol.bytes),
    }
}

fn structured_append_json(info: StructuredAppendInfo) -> StructuredAppendJson {
    StructuredAppendJson {
        index: info.index,
        count: info.count,
        parity: info.parity,
    }
}

fn symbology_json(sym: &SymbologyIdentifier) -> SymbologyJson {
    SymbologyJson {
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

fn payload_json(bytes: &[u8]) -> PayloadJson {
    match std::str::from_utf8(bytes) {
        Ok(s) => PayloadJson::Text { text: s.to_string() },
        Err(_) => PayloadJson::BytesB64 {
            bytes_b64: BASE64.encode(bytes),
        },
    }
}

fn render(symbols: &[QrSymbol], format: Format) -> Result<ExitCode> {
    match format {
        Format::Text => match symbols.first() {
            None => Ok(ExitCode::from(1)),
            Some(symbol) => {
                match std::str::from_utf8(&symbol.bytes) {
                    Ok(s) => println!("{s}"),
                    Err(_) => println!("base64:{}", BASE64.encode(&symbol.bytes)),
                }
                Ok(ExitCode::SUCCESS)
            }
        },
        Format::Json => {
            let entries: Vec<SymbolJson> = symbols.iter().map(symbol_to_json).collect();
            let mut stdout = io::stdout().lock();
            serde_json::to_writer(&mut stdout, &entries).context("writing JSON to stdout")?;
            stdout.write_all(b"\n").context("writing trailing newline")?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
