# rxing-reader

Standalone Rust workspace for the QR reader used by `qrcodesecureshare`.

## Crates

- `rxing-vendored`: trimmed QR-reading fork of `rxing`.
- `rxing-wasm`: `wasm-bindgen` wrapper published as `@andrewtheguy/rxing-wasm`.

## Commands

```sh
npm install
npm run build:wasm
npm run clippy
npm test
```

`npm run build:wasm` invokes the `wasm-pack` version pinned in `package-lock.json`
and writes package artifacts to `rxing-wasm/pkg/`.
