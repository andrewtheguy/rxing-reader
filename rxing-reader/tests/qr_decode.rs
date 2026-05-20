use std::collections::HashSet;
use std::path::PathBuf;

use image::ImageReader;
use rxing_reader::{
    AIFlag, Eci, ErrorCorrectionLevel, Mode, QrSymbol, decode_qr_codes_luma, rgba_to_luma,
};

fn load_image_as_rgba(relative_path: &str) -> (Vec<u8>, usize, usize) {
    let mut full = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    full.push(relative_path);
    let img = ImageReader::open(&full)
        .expect("open image")
        .with_guessed_format()
        .expect("guess image format")
        .decode()
        .expect("decode image")
        .into_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    (img.into_raw(), w, h)
}

// The full Cartesian product of (try_harder, try_invert, use_hybrid_binarizer).
// All three flags compose freely on the only remaining decode path
// (`decode_qr_codes_luma` + the manual BitMatrix flip for
// inversion); each affects an orthogonal stage of the pipeline.
const ALL_COMBOS: [(bool, bool, bool); 8] = [
    (false, false, false),
    (false, false, true),
    (false, true, false),
    (false, true, true),
    (true, false, false),
    (true, false, true),
    (true, true, false),
    (true, true, true),
];

const FAST_QR_GENERATED_MARGIN_MODULES: usize = 4;
const FAST_QR_GENERATED_SCALE: usize = 4;

#[derive(Clone, Copy)]
struct FastQrGeneratedFixture {
    file_name: &'static str,
    payload: &'static [u8],
    version: u32,
    error_correction_level: ErrorCorrectionLevel,
    mask: u8,
    modes: &'static [Mode],
    ecis: &'static [Eci],
}

impl FastQrGeneratedFixture {
    fn path(&self) -> String {
        format!("tests/fixtures/fast_qr/{}", self.file_name)
    }

    fn expected_side_pixels(&self) -> usize {
        let module_count = self.version as usize * 4 + 17;
        (module_count + FAST_QR_GENERATED_MARGIN_MODULES * 2) * FAST_QR_GENERATED_SCALE
    }
}

const FAST_QR_GENERATED_FIXTURES: &[FastQrGeneratedFixture] = &[
    FastQrGeneratedFixture {
        file_name: "numeric_l_v01_mask0.png",
        payload: b"01234567",
        version: 1,
        error_correction_level: ErrorCorrectionLevel::L,
        mask: 0,
        modes: &[Mode::Numeric],
        ecis: &[Eci::ISO8859_1],
    },
    FastQrGeneratedFixture {
        file_name: "alphanumeric_m_v02_mask1.png",
        payload: b"FAST QR-42",
        version: 2,
        error_correction_level: ErrorCorrectionLevel::M,
        mask: 1,
        modes: &[Mode::Alphanumeric],
        ecis: &[Eci::ISO8859_1],
    },
    FastQrGeneratedFixture {
        file_name: "byte_q_v03_mask2.png",
        payload: b"byte/q/v03/\x00\xff",
        version: 3,
        error_correction_level: ErrorCorrectionLevel::Q,
        mask: 2,
        modes: &[Mode::Byte],
        ecis: &[Eci::Unknown],
    },
    FastQrGeneratedFixture {
        file_name: "byte_h_v04_mask3.png",
        payload: b"byte h v04 mask3",
        version: 4,
        error_correction_level: ErrorCorrectionLevel::H,
        mask: 3,
        modes: &[Mode::Byte],
        ecis: &[Eci::Unknown],
    },
    FastQrGeneratedFixture {
        file_name: "numeric_m_v05_mask4.png",
        payload: b"3141592653589793238462643383279",
        version: 5,
        error_correction_level: ErrorCorrectionLevel::M,
        mask: 4,
        modes: &[Mode::Numeric],
        ecis: &[Eci::ISO8859_1],
    },
    FastQrGeneratedFixture {
        file_name: "alphanumeric_q_v06_mask5.png",
        payload: b"MASK 5 ALPHANUMERIC QR",
        version: 6,
        error_correction_level: ErrorCorrectionLevel::Q,
        mask: 5,
        modes: &[Mode::Alphanumeric],
        ecis: &[Eci::ISO8859_1],
    },
    FastQrGeneratedFixture {
        file_name: "byte_l_v07_mask6.png",
        payload: b"byte/l/v07/mask6\x10\x11",
        version: 7,
        error_correction_level: ErrorCorrectionLevel::L,
        mask: 6,
        modes: &[Mode::Byte],
        ecis: &[Eci::Unknown],
    },
    FastQrGeneratedFixture {
        file_name: "byte_h_v08_mask7.png",
        payload: b"byte h v08 mask7",
        version: 8,
        error_correction_level: ErrorCorrectionLevel::H,
        mask: 7,
        modes: &[Mode::Byte],
        ecis: &[Eci::Unknown],
    },
    FastQrGeneratedFixture {
        file_name: "numeric_l_v10_mask0.png",
        payload: b"01234567890123456789012345678901234567890123456789",
        version: 10,
        error_correction_level: ErrorCorrectionLevel::L,
        mask: 0,
        modes: &[Mode::Numeric],
        ecis: &[Eci::ISO8859_1],
    },
    FastQrGeneratedFixture {
        file_name: "alphanumeric_m_v27_mask1.png",
        payload: b"VERSION 27 ALPHA MODE",
        version: 27,
        error_correction_level: ErrorCorrectionLevel::M,
        mask: 1,
        modes: &[Mode::Alphanumeric],
        ecis: &[Eci::ISO8859_1],
    },
    FastQrGeneratedFixture {
        file_name: "byte_q_v40_mask2.png",
        payload: b"version 40 byte fixture from fast_qr",
        version: 40,
        error_correction_level: ErrorCorrectionLevel::Q,
        mask: 2,
        modes: &[Mode::Byte],
        ecis: &[Eci::Unknown],
    },
];

fn decode_combo(
    rgba: &[u8],
    w: usize,
    h: usize,
    (try_harder, try_invert, use_hybrid_binarizer): (bool, bool, bool),
) -> Option<Vec<u8>> {
    decode_combo_symbol(rgba, w, h, (try_harder, try_invert, use_hybrid_binarizer))
        .map(|s| s.bytes)
}

fn decode_combo_symbol(
    rgba: &[u8],
    w: usize,
    h: usize,
    (try_harder, try_invert, use_hybrid_binarizer): (bool, bool, bool),
) -> Option<QrSymbol> {
    let luma = rgba_to_luma(rgba, w, h).expect("luma");
    decode_qr_codes_luma(&luma, w, h, try_harder, try_invert, use_hybrid_binarizer, 1)
        .expect("decode")
        .into_iter()
        .next()
}

fn assert_fast_qr_fixture_symbol(
    symbol: &QrSymbol,
    fixture: &FastQrGeneratedFixture,
    combo: (bool, bool, bool),
) {
    assert_eq!(
        symbol.bytes.as_slice(),
        fixture.payload,
        "payload mismatch for {} combo={:?}",
        fixture.file_name,
        combo
    );
    assert_eq!(
        symbol.version, fixture.version,
        "version mismatch for {} combo={:?}",
        fixture.file_name, combo
    );
    assert_eq!(
        symbol.error_correction_level, fixture.error_correction_level,
        "EC level mismatch for {} combo={:?}",
        fixture.file_name, combo
    );
    assert_eq!(
        symbol.mask, fixture.mask,
        "mask mismatch for {} combo={:?}",
        fixture.file_name, combo
    );
    assert_eq!(
        symbol.modes.as_slice(),
        fixture.modes,
        "mode metadata mismatch for {} combo={:?}",
        fixture.file_name,
        combo
    );
    assert_eq!(
        symbol.ecis.as_slice(),
        fixture.ecis,
        "ECI metadata mismatch for {} combo={:?}",
        fixture.file_name,
        combo
    );
    assert_eq!(
        symbol.structured_append, None,
        "unexpected structured append metadata for {} combo={:?}",
        fixture.file_name, combo
    );
    assert_eq!(symbol.symbology.code, b'Q');
    assert_eq!(symbol.symbology.modifier, b'1');
    assert_eq!(symbol.symbology.eci_modifier_offset, 1);
    assert_eq!(symbol.symbology.ai_flag, AIFlag::None);
}

#[test]
fn decodes_fast_qr_generated_fixtures_and_pins_metadata() {
    for fixture in FAST_QR_GENERATED_FIXTURES {
        let path = fixture.path();
        let (rgba, width, height) = load_image_as_rgba(&path);
        let expected_side_pixels = fixture.expected_side_pixels();
        assert_eq!(
            (width, height),
            (expected_side_pixels, expected_side_pixels),
            "{} dimensions should match its forced QR version",
            fixture.file_name
        );

        for combo in ALL_COMBOS {
            let symbol = decode_combo_symbol(&rgba, width, height, combo).unwrap_or_else(|| {
                panic!(
                    "{} failed to decode for combo={:?}",
                    fixture.file_name, combo
                )
            });
            assert_fast_qr_fixture_symbol(&symbol, fixture, combo);
        }
    }
}

#[test]
fn decodes_qr_sample_png_in_every_combination() {
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_sample.png");
    for combo in ALL_COMBOS {
        let symbol = decode_combo_symbol(&rgba, w, h, combo)
            .unwrap_or_else(|| panic!("qr_sample.png failed to decode for combo={:?}", combo));
        assert_eq!(
            symbol.bytes.as_slice(),
            b"jfghjghjghfkghjkghj",
            "unexpected bytes for combo={:?}",
            combo
        );
        // Metadata pinned from a one-off observation against the fixture so
        // future decoder regressions (mis-reading version / EC / mask /
        // mode) trip the test.
        assert_eq!(symbol.version, 2, "version for combo={:?}", combo);
        assert_eq!(
            symbol.modes,
            vec![Mode::Byte],
            "expected Byte mode for combo={:?}",
            combo
        );
    }
}

#[test]
fn decodes_qr_code_complex_png_in_every_combination() {
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_code_complex.png");
    for combo in ALL_COMBOS {
        let bytes = decode_combo(&rgba, w, h, combo).unwrap_or_else(|| {
            panic!("qr_code_complex.png failed to decode for combo={:?}", combo)
        });
        assert_eq!(
            bytes.as_slice(),
            b"https://qr-code-styling.com",
            "unexpected bytes for combo={:?}",
            combo
        );
    }
}

#[test]
fn decodes_real_fountain_byte_mode_fixture_in_every_combination() {
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/fountain_binary_real.png");
    for combo in ALL_COMBOS {
        let symbol = decode_combo_symbol(&rgba, w, h, combo).unwrap_or_else(|| {
            panic!(
                "fountain_binary_real.png failed to decode for combo={:?}",
                combo
            )
        });
        // Binary fountain payload starts with magic bytes 0xff 0xfd; the bytes-only
        // path returns the raw QR BYTE-mode payload without any UTF-8/Latin-1 mangling.
        assert_eq!(
            &symbol.bytes[..2],
            [0xff, 0xfd],
            "wrong magic prefix for combo={:?}",
            combo
        );
        assert!(
            symbol.modes.contains(&Mode::Byte),
            "expected Byte mode in {:?} for combo={:?}",
            symbol.modes,
            combo
        );
    }
}

#[test]
fn qr_sample_inverted_png_requires_try_invert() {
    // Pixel-inverted (255 - rgb) copy of `qr_sample.png`, generated by the
    // ignored `generate_synthetic_fixtures` test below. The synthetic
    // inversion exercises the AlsoInverted hint in isolation: try_harder is
    // unused (image is small and clean), and HybridBinarizer vs
    // GlobalHistogramBinarizer both succeed on the flipped matrix. Decodes
    // iff try_invert = true.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_sample_inverted.png");
    for combo in ALL_COMBOS {
        let (_, try_invert, _) = combo;
        let result = decode_combo(&rgba, w, h, combo);
        if try_invert {
            let bytes = result.unwrap_or_else(|| {
                panic!(
                    "qr_sample_inverted.png expected to decode for combo={:?}",
                    combo
                )
            });
            assert_eq!(
                bytes.as_slice(),
                b"jfghjghjghfkghjkghj",
                "unexpected bytes for combo={:?}",
                combo
            );
        } else {
            assert!(
                result.is_none(),
                "qr_sample_inverted.png expected to NOT decode without try_invert for combo={:?}",
                combo
            );
        }
    }
}

#[test]
fn qr_sample_small_in_canvas_png_requires_try_harder() {
    // The baseline `qr_sample.png` downscaled to 80x80 and pasted into a
    // 1600x1600 white canvas (see `generate_synthetic_fixtures` below).
    // `find_finder_patterns` picks skip = (3*1600)/(4*97) ≈ 12 by default;
    // the shrunken finder modules are ~3 px tall, so the coarse scan walks
    // past them and only the dense try_harder=true scan (skip=3) catches
    // one. try_invert and the binarizer choice are both irrelevant.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_sample_small_in_canvas.png");
    for combo in ALL_COMBOS {
        let (try_harder, _, _) = combo;
        let result = decode_combo(&rgba, w, h, combo);
        if try_harder {
            let bytes = result.unwrap_or_else(|| {
                panic!(
                    "qr_sample_small_in_canvas.png expected to decode for combo={:?}",
                    combo
                )
            });
            assert_eq!(
                bytes.as_slice(),
                b"jfghjghjghfkghjkghj",
                "unexpected bytes for combo={:?}",
                combo
            );
        } else {
            assert!(
                result.is_none(),
                "qr_sample_small_in_canvas.png expected to NOT decode without try_harder for combo={:?}",
                combo
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Binarizer fallback. The wasm wrapper exposes a `binarizer_fallback` option
// that retries the full pipeline once with the opposite binarizer when the
// primary produces no results. Mirrors the `read_luma` policy in
// `rxing-wasm/src/lib.rs`. Validated here against the two binarizer-isolation
// fixtures (each decodes on only one binarizer) — proves fallback rescues
// both failure modes regardless of which binarizer the caller picks first.
// ---------------------------------------------------------------------------

/// Mirror of `read_luma`'s `binarizer_fallback` policy: run the single-symbol
/// pipeline with `primary_use_hybrid`; on miss, run again with the other.
fn decode_with_binarizer_fallback(
    rgba: &[u8],
    w: usize,
    h: usize,
    try_harder: bool,
    try_invert: bool,
    primary_use_hybrid: bool,
) -> Option<Vec<u8>> {
    if let Some(bytes) = decode_combo(rgba, w, h, (try_harder, try_invert, primary_use_hybrid)) {
        return Some(bytes);
    }
    decode_combo(rgba, w, h, (try_harder, try_invert, !primary_use_hybrid))
}

#[test]
fn binarizer_fallback_rescues_qr_complex_2_with_hybrid_primary() {
    // qr-complex-2.png decodes only on GlobalHistogramBinarizer (see
    // `qr_complex_2_png_requires_global_histogram_binarizer`). Caller picks
    // Hybrid as primary; binarizer fallback should retry on Global and decode.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr-complex-2.png");
    let bytes = decode_with_binarizer_fallback(&rgba, w, h, false, false, true)
        .expect("binarizer fallback should rescue via Global");
    assert_eq!(bytes.as_slice(), b"http://scnv.io/MsSv?qr=1");
}

#[test]
fn binarizer_fallback_rescues_vignetted_with_global_primary() {
    // Mirror: qr_sample_vignetted.png decodes only on HybridBinarizer (see
    // `qr_sample_vignetted_png_requires_hybrid_binarizer`). Caller picks
    // Global as primary; binarizer fallback should retry on Hybrid and decode.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_sample_vignetted.png");
    let bytes = decode_with_binarizer_fallback(&rgba, w, h, false, false, false)
        .expect("binarizer fallback should rescue via Hybrid");
    assert_eq!(bytes.as_slice(), QR_SAMPLE_TEXT);
}

#[test]
fn rgba_length_mismatch_is_rejected() {
    let err = rgba_to_luma(&[0u8; 15], 2, 2).expect_err("expected length-mismatch error");
    assert!(err.contains("rgba length"), "unexpected error: {}", err);
}

// ---------------------------------------------------------------------------
// Real-world fixtures that exercise the `try_harder` pyramid + morphological
// close pre-pass. Neither decodes at the original resolution; both surface
// only after the buffer is downscaled and/or `BinaryBitmap::close()` is
// applied. These pin the capability we lost when `FilteredImageReader` was
// removed and re-added via `decode_qr_codes_luma`'s try_harder branch + the
// `BinaryBitmap::close()` / `downscale_luma_buffer` utilities.
// ---------------------------------------------------------------------------

#[test]
fn qr_zoo_jpg_requires_try_harder_and_try_invert() {
    // 2258×1344 phone photo of a white-on-dark-green QR. Needs the pyramid
    // (try_harder) to surface the finders and the in-place flip
    // (try_invert) to read the inverted reflectance.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_zoo.jpg");
    let expected = b"https://zoo.sandiegozoo.org/2024-sdmag-pandas";
    for combo in ALL_COMBOS {
        let (try_harder, try_invert, _) = combo;
        let result = decode_combo(&rgba, w, h, combo);
        if try_harder && try_invert {
            let bytes = result
                .unwrap_or_else(|| panic!("qr_zoo.jpg expected to decode for combo={:?}", combo));
            assert_eq!(
                bytes.as_slice(),
                expected.as_slice(),
                "unexpected bytes for combo={:?}",
                combo
            );
        } else {
            assert!(
                result.is_none(),
                "qr_zoo.jpg expected to NOT decode for combo={:?}, got Some({} bytes)",
                combo,
                result.as_ref().map(|b| b.len()).unwrap_or(0)
            );
        }
    }
}

/// Loop over `ALL_COMBOS` asserting that `rgba` decodes to `expected` iff
/// `try_harder = true`. `label` is interpolated into panic messages so test
/// failures point at the specific fixture/transform under test.
fn assert_requires_try_harder(rgba: &[u8], w: usize, h: usize, expected: &[u8], label: &str) {
    for combo in ALL_COMBOS {
        let (try_harder, _, _) = combo;
        let result = decode_combo(rgba, w, h, combo);
        if try_harder {
            let bytes =
                result.unwrap_or_else(|| panic!("{label} expected to decode for combo={combo:?}"));
            assert_eq!(
                bytes.as_slice(),
                expected,
                "{label}: unexpected bytes for combo={combo:?}",
            );
        } else {
            assert!(
                result.is_none(),
                "{label} expected to NOT decode without try_harder for combo={combo:?}",
            );
        }
    }
}

#[test]
fn qr_complex_2_png_requires_global_histogram_binarizer() {
    // 431×431 stylized QR (45°-rotated "diamond" framing, orange finder
    // patterns, orange center logo overlaying the data modules, blue speckled
    // dark modules on white). Decodes iff `use_hybrid_binarizer = false`;
    // `try_harder` and `try_invert` are both irrelevant. The HybridBinarizer's
    // 8×8 local-threshold blocks mis-classify the colored finders and center
    // logo (foreground is medium-bright orange, not black), while the
    // GlobalHistogramBinarizer's single image-wide threshold separates the
    // overall dark/light populations cleanly. Pins the GlobalHistogramBinarizer
    // branch of `decode_one_layer`; the mirror test
    // `qr_sample_vignetted_png_requires_hybrid_binarizer` pins the Hybrid
    // branch, so deleting either binarizer breaks one of the two.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr-complex-2.png");
    let expected = b"http://scnv.io/MsSv?qr=1";
    for combo in ALL_COMBOS {
        let (_, _, use_hybrid_binarizer) = combo;
        let result = decode_combo(&rgba, w, h, combo);
        if !use_hybrid_binarizer {
            let bytes = result.unwrap_or_else(|| {
                panic!("qr-complex-2.png expected to decode for combo={:?}", combo)
            });
            assert_eq!(
                bytes.as_slice(),
                expected.as_slice(),
                "unexpected bytes for combo={:?}",
                combo
            );
        } else {
            assert!(
                result.is_none(),
                "qr-complex-2.png expected to NOT decode with HybridBinarizer for combo={:?}, got Some({} bytes)",
                combo,
                result.as_ref().map(|b| b.len()).unwrap_or(0)
            );
        }
    }
}

#[test]
fn qr_sample_vignetted_png_requires_hybrid_binarizer() {
    // qr_sample.png multiplied by a radial gradient from white (centre) to
    // gray50 (corners) — see `fixtures/regen_synthetic.sh`. Strong dark
    // vignette that drops corner luminance to ~50% of centre while preserving
    // local black/white contrast inside each small neighbourhood. The mirror
    // case of `qr_complex_2_png_requires_global_histogram_binarizer`: decodes
    // iff `use_hybrid_binarizer = true`, regardless of `try_harder` /
    // `try_invert`. GlobalHistogramBinarizer's single image-wide threshold
    // can't separate dim-corner whites from bright-centre blacks (overlapping
    // luminance populations); HybridBinarizer's per-8×8-block thresholds
    // adapt to local luminance and decode cleanly. Together with the
    // `qr-complex-2.png` test, this pins both binarizer branches in
    // `decode_one_layer` as load-bearing — neither can be deleted without
    // losing a real-world failure mode.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_sample_vignetted.png");
    for combo in ALL_COMBOS {
        let (_, _, use_hybrid_binarizer) = combo;
        let result = decode_combo(&rgba, w, h, combo);
        if use_hybrid_binarizer {
            let bytes = result.unwrap_or_else(|| {
                panic!(
                    "qr_sample_vignetted.png expected to decode for combo={:?}",
                    combo
                )
            });
            assert_eq!(
                bytes.as_slice(),
                QR_SAMPLE_TEXT,
                "unexpected bytes for combo={:?}",
                combo
            );
        } else {
            assert!(
                result.is_none(),
                "qr_sample_vignetted.png expected to NOT decode with GlobalHistogramBinarizer for combo={:?}, got Some({} bytes)",
                combo,
                result.as_ref().map(|b| b.len()).unwrap_or(0)
            );
        }
    }
}

#[test]
fn qr_code_complex_rotated_jpg_requires_try_harder() {
    // 183×210 phone photo of a rotated QR encoding a longer URL payload (named
    // `qr_code_complex_rotated.jpg` to flag the long-URL payload, distinct
    // from the short `jfghjghj...` payload in qr_sample.png). The original
    // resolution is below the pyramid threshold so no downscale layer fires;
    // the close-pass (try_harder = true → bitmap.close()) is what surfaces
    // the finders.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_code_complex_rotated.jpg");
    assert_requires_try_harder(
        &rgba,
        w,
        h,
        b"https://nc.cesdk12.org/ncsd/PXP2_Login_Parent.aspx?regenerateSessionId=True",
        "qr_code_complex_rotated.jpg",
    );
}

// ---------------------------------------------------------------------------
// Rotation tests. The fixture-based option-isolation tests above don't cover
// rotation (none of the fixtures are pre-rotated). These exercise the
// rotation invariance built into rxing's QR detector by rotating the
// baseline fixture in-memory before decoding.
// ---------------------------------------------------------------------------

const QR_SAMPLE_TEXT: &[u8] = b"jfghjghjghfkghjkghj";

fn invert_rgba(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| [255 - p[0], 255 - p[1], 255 - p[2], p[3]])
        .collect()
}

/// Rotate an RGBA buffer 90° clockwise. Source (x, y) maps to dst (h - 1 - y, x).
fn rotate_rgba_90_cw(rgba: &[u8], w: usize, h: usize) -> (Vec<u8>, usize, usize) {
    let mut out = vec![0u8; rgba.len()];
    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * 4;
            let dx = h - 1 - y;
            let dy = x;
            let dst = (dy * h + dx) * 4;
            out[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    (out, h, w)
}

#[test]
fn inverted_qr_sample_in_memory_requires_try_invert() {
    // In-memory parallel of `qr_sample_inverted_png_requires_try_invert`:
    // pixel-inverts the baseline RGBA at test time instead of loading the
    // pre-inverted PNG from disk. Catches regressions to the
    // `decode_with_optional_invert` flip path that wouldn't surface if the
    // fixture file is out of date / corrupted / missing.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_sample.png");
    let inverted = invert_rgba(&rgba);

    for combo in ALL_COMBOS {
        let (_, try_invert, _) = combo;
        let result = decode_combo(&inverted, w, h, combo);
        if try_invert {
            let bytes = result.unwrap_or_else(|| {
                panic!(
                    "in-memory inverted qr_sample expected to decode for combo={:?}",
                    combo
                )
            });
            assert_eq!(bytes.as_slice(), QR_SAMPLE_TEXT);
        } else {
            assert!(
                result.is_none(),
                "in-memory inverted qr_sample expected to NOT decode without try_invert for combo={:?}",
                combo
            );
        }
    }
}

#[test]
fn qr_sample_rotated_speckled_png_requires_try_harder() {
    // Synthetic, clean-source analog of `qr_code_complex_rotated_jpg_requires_try_harder`:
    // qr_sample.png nearest-neighbor rotated 17° (staircase aliasing along
    // finder edges) plus a sparse white-salt mask (1-pixel holes inside dark
    // modules). Generated by `fixtures/regen_synthetic.sh`.
    //
    // rxing's detector is rotation-invariant for clean fixtures (see
    // `rotated_qr_sample_decodes_natively`), so rotation alone isn't enough
    // — the salt is what defeats the original-resolution scan, and the
    // `try_harder = true` morphological close-pass is what fills the holes
    // and rescues detection. Pins the close-pass branch of `decode_qr_codes_luma`
    // without depending on the JPG fixture (which conflates rotation, motion
    // blur, JPEG noise, and low resolution into a single failure mode).
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_sample_rotated_speckled.png");
    assert_requires_try_harder(
        &rgba,
        w,
        h,
        QR_SAMPLE_TEXT,
        "qr_sample_rotated_speckled.png",
    );
}

#[test]
fn rotated_qr_sample_decodes_natively() {
    // rxing's QR finder reorders the three concentric finder patterns into a
    // canonical (TL, TR, BL) tri-corner before sampling, so a clean QR decodes
    // at every 90° orientation. A future regression (e.g. losing the
    // canonical reordering) would surface here as a decode miss.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_sample.png");
    let (rotated, rw, rh) = rotate_rgba_90_cw(&rgba, w, h);

    let bytes = decode_combo(&rotated, rw, rh, (false, false, true))
        .expect("rxing's QR detector should be rotation-invariant for clean fixtures");
    assert_eq!(bytes.as_slice(), QR_SAMPLE_TEXT);
}

/// Diagnostic: prints which `(try_harder, try_invert, use_hybrid_binarizer)`
/// combos decode each fixture. Marked `#[ignore]` so it doesn't run in the
/// normal suite; rerun with `cargo test --test qr_decode probe -- --ignored --nocapture`.
#[test]
#[ignore]
fn probe_fixture_requirements() {
    let fixtures: &[(&str, &[u8])] = &[
        ("qr_sample.png", b"jfghjghjghfkghjkghj"),
        ("qr_code_complex.png", b"https://qr-code-styling.com"),
        ("qr_sample_inverted.png", b"jfghjghjghfkghjkghj"),
        ("qr_sample_small_in_canvas.png", b"jfghjghjghfkghjkghj"),
        (
            "qr_zoo.jpg",
            b"https://zoo.sandiegozoo.org/2024-sdmag-pandas",
        ),
        (
            "qr_code_complex_rotated.jpg",
            b"https://nc.cesdk12.org/ncsd/PXP2_Login_Parent.aspx?regenerateSessionId=True",
        ),
        ("qr_sample_rotated_speckled.png", b"jfghjghjghfkghjkghj"),
        ("qr-complex-2.png", b"http://scnv.io/MsSv?qr=1"),
        ("qr_sample_vignetted.png", b"jfghjghjghfkghjkghj"),
    ];
    for (name, expected) in fixtures {
        let path = format!("tests/fixtures/{name}");
        let (rgba, w, h) = load_image_as_rgba(&path);
        println!("--- {name} ({w}x{h}) ---");
        for combo in ALL_COMBOS {
            let got = decode_combo(&rgba, w, h, combo);
            let label = match &got {
                Some(b) if expected.is_empty() => {
                    format!("ok ({})", String::from_utf8_lossy(b))
                }
                Some(b) if b.as_slice() == *expected => "ok".to_string(),
                Some(_) => "WRONG".to_string(),
                None => "miss".to_string(),
            };
            println!(
                "  hard={} inv={} hyb={} -> {}",
                combo.0, combo.1, combo.2, label
            );
        }
    }
}

#[test]
fn rotated_and_inverted_qr_sample_requires_try_invert() {
    // Compose both transforms. Rotation alone is handled natively (see
    // `rotated_qr_sample_decodes_natively`) but the inversion still requires
    // `try_invert`. Pins that the manual BitMatrix flip in `decode_qr_codes_luma`
    // composes correctly with a rotated source: every try_invert=true combo
    // decodes, every try_invert=false combo (including try_harder=true, which
    // could plausibly rescue via the close-pass / pyramid) misses.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_sample.png");
    let (rotated, rw, rh) = rotate_rgba_90_cw(&rgba, w, h);
    let rotated_inverted = invert_rgba(&rotated);

    for combo in ALL_COMBOS {
        let (_, try_invert, _) = combo;
        let result = decode_combo(&rotated_inverted, rw, rh, combo);
        if try_invert {
            let bytes = result.unwrap_or_else(|| {
                panic!(
                    "rotated-inverted qr_sample expected to decode for combo={:?}",
                    combo
                )
            });
            assert_eq!(
                bytes.as_slice(),
                QR_SAMPLE_TEXT,
                "unexpected bytes for combo={:?}",
                combo
            );
        } else {
            assert!(
                result.is_none(),
                "rotated-inverted qr_sample expected to NOT decode without try_invert for combo={:?}",
                combo
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Multi-symbol decode tests. The production wrapper (`rxing-wasm` + the JS
// `readQrCodesFromRgba` util) hard-codes the count cap to 1 because every
// app consumer reads only `results[0]`. These tests pin the underlying
// `decode_set_number_with_hints` multi-decode capability so a future caller
// that needs >1 symbol per frame won't silently regress.
//
// Fixtures `qr_two_codes.png` and `qr_three_codes.png` are synthetic
// side-by-side composites of `qr_sample.png` + `qr_code_complex.png`
// generated by `fixtures/regen_synthetic.sh`.
// ---------------------------------------------------------------------------

/// Multi-symbol decode. Bypasses the inversion/pyramid/close pipeline used
/// by `decode_qr_codes_luma` because the multi-QR fixtures are clean upright
/// composites. `count` is forwarded through the high-level decode API
/// (0 = unlimited).
fn decode_all(rgba: &[u8], w: usize, h: usize, count: usize) -> Vec<Vec<u8>> {
    let luma = rgba_to_luma(rgba, w, h).expect("luma");
    decode_qr_codes_luma(&luma, w, h, false, false, true, count)
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.bytes)
        .collect()
}

const QR_COMPLEX_TEXT: &[u8] = b"https://qr-code-styling.com";

#[test]
fn decodes_two_qr_codes_in_single_image() {
    // Side-by-side composite of the two clean QR fixtures. Verifies that
    // find_finder_patterns surfaces both independent finder-pattern triples
    // and that the multi-decode loop returns both payloads when the count
    // cap permits.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_two_codes.png");
    let decoded = decode_all(&rgba, w, h, 8);
    assert_eq!(
        decoded.len(),
        2,
        "expected both QR codes to decode, got {} results",
        decoded.len()
    );
    let payloads: HashSet<&[u8]> = decoded.iter().map(|v| v.as_slice()).collect();
    assert!(
        payloads.contains(QR_SAMPLE_TEXT),
        "missing qr_sample payload; got {:?}",
        decoded
            .iter()
            .map(|v| String::from_utf8_lossy(v).into_owned())
            .collect::<Vec<_>>()
    );
    assert!(
        payloads.contains(QR_COMPLEX_TEXT),
        "missing qr_code_complex payload; got {:?}",
        decoded
            .iter()
            .map(|v| String::from_utf8_lossy(v).into_owned())
            .collect::<Vec<_>>()
    );
}

#[test]
fn count_cap_of_one_returns_single_symbol_from_multi_qr_image() {
    // Same two-QR fixture, but with count = 1: the multi-decode loop
    // must short-circuit after the first valid symbol. Pins the
    // single-symbol fast path used by the rxing-wasm wrapper in
    // production (see `MAX_NUMBER_OF_SYMBOLS` in src/utils/rxingWasm.ts).
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_two_codes.png");
    let decoded = decode_all(&rgba, w, h, 1);
    assert_eq!(
        decoded.len(),
        1,
        "count=1 should cap result vec at length 1, got {}",
        decoded.len()
    );
    let payload = decoded[0].as_slice();
    assert!(
        payload == QR_SAMPLE_TEXT || payload == QR_COMPLEX_TEXT,
        "unexpected payload from count-capped decode: {:?}",
        String::from_utf8_lossy(payload)
    );
}

#[test]
fn count_zero_returns_every_symbol_in_multi_qr_image() {
    // count = 0 is documented as "unlimited" in
    // `decode_qr_codes_luma`. Pin that contract against the
    // two-QR fixture.
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_two_codes.png");
    let decoded = decode_all(&rgba, w, h, 0);
    assert_eq!(
        decoded.len(),
        2,
        "count=0 (unlimited) should return both symbols, got {}",
        decoded.len()
    );
}

#[test]
fn decodes_three_qr_codes_in_single_image() {
    // Three-symbol fixture: qr_sample twice plus qr_code_complex.
    // Exercises that the multi-decode loop keeps iterating past two and
    // that two identical payloads in the same image don't collapse into
    // a single result (find_finder_patterns yields three independent
    // triples, decode returns three results).
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_three_codes.png");
    let decoded = decode_all(&rgba, w, h, 0);
    assert_eq!(
        decoded.len(),
        3,
        "expected three QR codes to decode, got {} results: {:?}",
        decoded.len(),
        decoded
            .iter()
            .map(|v| String::from_utf8_lossy(v).into_owned())
            .collect::<Vec<_>>()
    );
    let sample_hits = decoded
        .iter()
        .filter(|v| v.as_slice() == QR_SAMPLE_TEXT)
        .count();
    let complex_hits = decoded
        .iter()
        .filter(|v| v.as_slice() == QR_COMPLEX_TEXT)
        .count();
    assert_eq!(sample_hits, 2, "expected qr_sample payload twice");
    assert_eq!(complex_hits, 1, "expected qr_code_complex payload once");
}

// ---------------------------------------------------------------------------
// QR encoding-mode coverage. The fixtures above all happen to be encoded in
// QR Byte mode (their payloads contain at least one character outside the
// numeric / alphanumeric subsets), so they only exercise
// `decode_byte_segment`. The two fixtures below pin the other two ISO-18004
// data modes that the `qrcode` module actually decodes:
//
// * `qr_numeric.png` — pure-digit payload that the encoder packs into
//   Numeric mode (3 digits per 10 bits). Exercises `decode_numeric_segment`
//   including the trailing-group unpack (the 36-digit payload is exactly
//   12 three-digit groups, so a regression that mis-handles the 2- or
//   1-digit terminal group would not surface here — that's intentional;
//   it isolates the steady-state 10-bit loop).
//
// * `qr_base45_alphanumeric.png` — base45-encoded binary payload that, by
//   construction, uses only the 45-character QR alphanumeric charset
//   (`0-9 A-Z` plus ` $%*+-./:`), so the encoder picks Alphanumeric mode
//   (2 chars per 11 bits). The test base45-decodes the decoded string and
//   asserts byte-for-byte equality with the original bytes, so it pins
//   both `decode_alphanumeric_segment` and the round-trip contract used by
//   real-world consumers (EU Digital COVID Certificate, etc.).
//
// Both fixtures are produced one-off by `/tmp/qrgen` (a small Rust
// generator using the `qrcode` crate's low-level `Bits` API to force the
// encoding mode) and committed to the repo so tests do not depend on the
// generator at run time.
// ---------------------------------------------------------------------------

#[test]
fn decodes_numeric_mode_qr_in_every_combination() {
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_numeric.png");
    let expected = b"012345678901234567890123456789012345";
    for combo in ALL_COMBOS {
        let symbol = decode_combo_symbol(&rgba, w, h, combo)
            .unwrap_or_else(|| panic!("qr_numeric.png failed to decode for combo={:?}", combo));
        assert_eq!(
            symbol.bytes.as_slice(),
            expected.as_slice(),
            "unexpected bytes for combo={:?}",
            combo
        );
        assert_eq!(
            symbol.modes,
            vec![Mode::Numeric],
            "expected Numeric mode only for combo={:?}",
            combo
        );
    }
}

const BASE45_ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// Minimal RFC 9285 base45 decoder. Returns `None` on any out-of-charset
/// character, malformed trailing group, or 16-bit overflow.
fn base45_decode(input: &[u8]) -> Option<Vec<u8>> {
    fn idx(c: u8) -> Option<u16> {
        BASE45_ALPHABET.iter().position(|&b| b == c).map(|i| i as u16)
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 <= input.len() {
        // Per RFC 9285 the value packed into a 3-char group is bounded by
        // u16::MAX (0xFFFF), so a `u16` overflow on the additions would
        // already signal a malformed input. Use a `u32` for the intermediate
        // so we can detect that overflow instead of wrapping silently.
        let n = idx(input[i])? as u32
            + idx(input[i + 1])? as u32 * 45
            + idx(input[i + 2])? as u32 * 45 * 45;
        if n > 0xFFFF {
            return None;
        }
        out.push((n / 256) as u8);
        out.push((n % 256) as u8);
        i += 3;
    }
    match input.len() - i {
        0 => Some(out),
        2 => {
            // Same u16-overflow concern as the 3-char branch above; a single
            // byte's value is bounded by 0xFF so a 2-char group whose decoded
            // value exceeds that is malformed.
            let n = idx(input[i])? as u32 + idx(input[i + 1])? as u32 * 45;
            if n > 0xFF {
                return None;
            }
            out.push(n as u8);
            Some(out)
        }
        _ => None,
    }
}

#[test]
fn decodes_base45_alphanumeric_mode_qr_and_round_trips() {
    let (rgba, w, h) = load_image_as_rgba("tests/fixtures/qr_base45_alphanumeric.png");
    // Raw bytes the generator base45-encoded into the QR alphanumeric payload.
    // The 0x00 / 0xff / 0xde 0xad 0xbe 0xef bytes guarantee the payload cannot
    // round-trip as plain ASCII — the base45 layer is load-bearing.
    let expected_raw: &[u8] =
        b"\x00\xff\x01\xfe rxing-reader base45 round-trip \xde\xad\xbe\xef";
    for combo in ALL_COMBOS {
        let symbol = decode_combo_symbol(&rgba, w, h, combo).unwrap_or_else(|| {
            panic!(
                "qr_base45_alphanumeric.png failed to decode for combo={:?}",
                combo
            )
        });
        // Every byte returned by the decoder must be in the base45 charset —
        // if any wasn't, the encoder would have spilled into Byte mode and
        // this fixture would no longer cover `decode_alphanumeric_segment`.
        for &b in &symbol.bytes {
            assert!(
                BASE45_ALPHABET.contains(&b),
                "decoded byte {:#x} outside base45 charset for combo={:?} \
                 (fixture is no longer pure-alphanumeric)",
                b,
                combo
            );
        }
        assert_eq!(
            symbol.modes,
            vec![Mode::Alphanumeric],
            "expected Alphanumeric mode only for combo={:?}",
            combo
        );
        let decoded = base45_decode(&symbol.bytes).unwrap_or_else(|| {
            panic!(
                "base45 decode failed for combo={:?}; got {:?}",
                combo,
                String::from_utf8_lossy(&symbol.bytes)
            )
        });
        assert_eq!(
            decoded.as_slice(),
            expected_raw,
            "base45 round-trip mismatch for combo={:?}",
            combo
        );
    }
}

// TODO: add a Kanji-mode QR fixture (`qr_kanji.png`) and a matching test
// asserting `symbol.modes == vec![Mode::Kanji]`. rxing-reader implements
// `decode_kanji_segment` (Shift_JIS double-byte → UTF-16BE bytes per QR
// spec) but there is no fixture exercising it end-to-end. Generation
// requires a QR encoder that forces Kanji mode (the `qrcode` crate's
// low-level `Bits::push_kanji_data` does this); commit the rendered PNG
// to `tests/fixtures/qr_kanji.png` and pin both the bytes and the modes
// metadata. The other supported data modes (Numeric, Alphanumeric, Byte)
// are already covered above.

#[test]
fn base45_decode_round_trips_known_vectors() {
    // Sanity test on the in-test base45 decoder so a bug here doesn't masquerade
    // as a QR decode regression. Vectors from RFC 9285 §4.4 ("AB" → "BB8",
    // "Hello!!" → "%69 VD92EX0") plus the empty string.
    assert_eq!(base45_decode(b"").unwrap(), b"");
    assert_eq!(base45_decode(b"BB8").unwrap(), b"AB");
    assert_eq!(base45_decode(b"%69 VD92EX0").unwrap(), b"Hello!!");
    // Out-of-charset and malformed inputs are rejected.
    assert!(base45_decode(b"!!!").is_none(), "rejects non-charset");
    assert!(base45_decode(b"B").is_none(), "rejects 1-char trailing group");
}
