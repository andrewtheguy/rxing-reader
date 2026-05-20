# rxing-reader

Standalone Rust workspace for the QR reader used by `qrcodesecureshare`.

## Crates

- `rxing-reader`: trimmed QR-reading fork of `rxing`.
- `rxing-wasm`: `wasm-bindgen` wrapper published as `@andrewtheguy/rxing-wasm`.

## Commands

```sh
cargo clippy --workspace --all-targets
cargo test --workspace --release

cd rxing-wasm
npm install
npm run build
```

`npm run build` (inside `rxing-wasm/`) invokes the `wasm-pack` version pinned in
`rxing-wasm/package-lock.json` and writes package artifacts to `rxing-wasm/pkg/`.
