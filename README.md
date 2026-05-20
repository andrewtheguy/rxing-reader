# rxing-reader

Standalone Rust workspace providing a trimmed, QR-reading-only fork of
[`rxing`](https://crates.io/crates/rxing) v0.9.0 together with a thin
`wasm-bindgen` wrapper.

## Crates

- `rxing-reader`: the trimmed decoder library. Other formats and the
  encoder paths from upstream `rxing` have been removed.
- `rxing-wasm`: `wasm-bindgen` wrapper around `rxing-reader`. Distributed
  as a `.tgz` attached to each [GitHub release](https://github.com/andrewtheguy/rxing-reader/releases)
  (not published to the npm registry).
- `rxing-cli`: command-line wrapper around `rxing-reader`. Decodes QR
  codes from a local image file or an http(s) URL. Prebuilt binaries
  for `linux-amd64`, `linux-arm64`, `macos-arm64`, and `windows-amd64`
  are attached to each [GitHub release](https://github.com/andrewtheguy/rxing-reader/releases).

## Using `rxing-reader` from Rust

`Cargo.toml`:

```toml
[dependencies]
rxing-reader = { git = "https://github.com/andrewtheguy/rxing-reader" }
image = "0.25"
```

`src/main.rs`:

```rust
use image::ImageReader;
use rxing_reader::{decode_qr_codes_luma, rgba_to_luma};

fn main() {
    let img = ImageReader::open("qr.png").unwrap().decode().unwrap().to_rgba8();
    let (w, h) = (img.width(), img.height());
    let luma = rgba_to_luma(img.as_raw(), w, h).unwrap();

    let results = decode_qr_codes_luma(
        &luma, w, h,
        /* try_harder            */ false,
        /* try_invert            */ false,
        /* use_hybrid_binarizer  */ true,
        /* max_number_of_symbols */ 0,
    ).unwrap();

    for bytes in results {
      println!("{}", String::from_utf8_lossy(&bytes));
    }
}
```

## Using `@andrewtheguy/rxing-wasm` from JavaScript

The wasm package is not on the npm registry. Install it from the `.tgz`
asset attached to a [GitHub release](https://github.com/andrewtheguy/rxing-reader/releases).
Pick a release tag, copy the asset URL, and install directly from it:

```sh
npm install https://github.com/andrewtheguy/rxing-reader/releases/download/v0.0.3/andrewtheguy-rxing-wasm-0.0.3.tgz
```

Or pin it in `package.json` so `npm ci` is reproducible:

```json
{
  "dependencies": {
    "@andrewtheguy/rxing-wasm": "https://github.com/andrewtheguy/rxing-reader/releases/download/v0.0.3/andrewtheguy-rxing-wasm-0.0.3.tgz"
  }
}
```

The tarball is the exact `wasm-pack` output (the contents of
`rxing-wasm/pkg/`); npm unpacks it under the scoped package name from
its internal `package.json`, so imports keep using `@andrewtheguy/rxing-wasm`:

```js
import init, { read_qr_codes_rgba } from "@andrewtheguy/rxing-wasm";

await init();
const { data, width, height } = ctx.getImageData(0, 0, canvas.width, canvas.height);
const symbols = read_qr_codes_rgba(
    data, width, height,
    /* try_harder            */ false,
    /* try_invert            */ false,
    /* use_hybrid_binarizer  */ true,
    /* binarizer_fallback    */ false,
    /* max_number_of_symbols */ 0,
);
for (const bytes of symbols) {
    console.log(new TextDecoder().decode(bytes));
}
```

See `rxing-wasm/src/lib.rs` for the full flag semantics (retry order,
binarizer fallback, multi-symbol cap).

## Using `rxing-cli`

Download a prebuilt tarball from a [GitHub release](https://github.com/andrewtheguy/rxing-reader/releases)
and extract the `rxing-cli` binary:

```sh
curl -L https://github.com/andrewtheguy/rxing-reader/releases/download/v0.0.6/rxing-cli-v0.0.6-linux-amd64.tar.gz \
  | tar -xz
./rxing-cli --help
```

Or build from source:

```sh
cargo install --git https://github.com/andrewtheguy/rxing-reader rxing-cli
```

The CLI accepts a local file path or an http(s) URL and supports two
output formats:

```sh
# Plain text (default). Caps decode at 1 result; non-UTF-8 payloads
# are printed as `base64:<b64>`. Exits 1 when no QR is found.
rxing-cli qr.png
# jfghjghjghfkghjkghj

# JSON. Returns every detection as an array; each entry has a
# `text` field (for UTF-8 payloads) or `bytes_b64` (for binary).
rxing-cli --format json https://example.com/qr.png
# [{"text":"https://qr-code-styling.com"},{"text":"jfghjghjghfkghjkghj"}]
```

Decode-pipeline flags (`try_harder`, `try_invert`, binarizer choice,
binarizer fallback) are not exposed — the CLI hard-codes the
"one-shot image upload" defaults recommended in `rxing-wasm/src/lib.rs`.

## Build and test

```sh
cargo clippy --workspace --all-targets
cargo test  --workspace --release

cd rxing-wasm
npm install
npm run build
```

`npm run build` invokes the `wasm-pack` version pinned in
`rxing-wasm/package-lock.json` and writes package artifacts to
`rxing-wasm/pkg/`. Always invoke `wasm-pack` through `npm`/`npx` so the
pinned version is used; do not call a system-wide cargo-installed
binary.

## License

Apache-2.0. See `LICENSE`.
