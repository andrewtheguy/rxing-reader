use std::io::{self, Cursor, Read, Write};
use std::process::ExitCode;

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use clap::{Parser, ValueEnum};
use image::ImageReader;
use rxing_reader::{decode_qr_codes_luma, rgba_to_luma};
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
#[serde(untagged)]
enum Entry {
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
    let results = decode_payloads(&rgba, w, h, max)?;
    render(&results, cli.format)
}

fn load_bytes(source: &str) -> Result<Vec<u8>> {
    if source.starts_with("http://") || source.starts_with("https://") {
        fetch_url(source)
    } else {
        std::fs::read(source).with_context(|| format!("failed to read file {source}"))
    }
}

fn fetch_url(url: &str) -> Result<Vec<u8>> {
    let response = ureq::get(url)
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

fn decode_image_bytes(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("could not guess image format")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader.decode().context("image decode failed")?.into_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Ok((rgba.into_raw(), w, h))
}

fn decode_payloads(rgba: &[u8], w: u32, h: u32, max: u32) -> Result<Vec<Vec<u8>>> {
    let luma = rgba_to_luma(rgba, w, h).map_err(anyhow::Error::msg)?;
    let primary = decode_qr_codes_luma(&luma, w, h, true, true, true, max)?;
    if !primary.is_empty() {
        return Ok(primary);
    }
    let fallback = decode_qr_codes_luma(&luma, w, h, true, true, false, max)?;
    Ok(fallback)
}

fn render(results: &[Vec<u8>], format: Format) -> Result<ExitCode> {
    match format {
        Format::Text => match results.first() {
            None => Ok(ExitCode::from(1)),
            Some(bytes) => {
                match std::str::from_utf8(bytes) {
                    Ok(s) => println!("{s}"),
                    Err(_) => println!("base64:{}", BASE64.encode(bytes)),
                }
                Ok(ExitCode::SUCCESS)
            }
        },
        Format::Json => {
            let entries: Vec<Entry> = results
                .iter()
                .map(|bytes| match std::str::from_utf8(bytes) {
                    Ok(s) => Entry::Text { text: s.to_string() },
                    Err(_) => Entry::BytesB64 {
                        bytes_b64: BASE64.encode(bytes),
                    },
                })
                .collect();
            let mut stdout = io::stdout().lock();
            serde_json::to_writer(&mut stdout, &entries).context("writing JSON to stdout")?;
            stdout.write_all(b"\n").context("writing trailing newline")?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
