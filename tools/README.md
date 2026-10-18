# tools

Separate crates built on tpt-layout. The core crate stays dependency-free, so each tool has its own
`Cargo.toml` and its own `[workspace]`, and is built on its own, not as part of the core workspace.

| Crate | What it is |
| --- | --- |
| [spec](spec/) | `tpt-layout-spec`: a JSON description of a box tree, built and laid out, with SVG and layout JSON output. |
| [cli](cli/) | `tpt-layout-cli`: a command-line tool over the spec crate. |
| [web](web/) | `tpt-layout-web`: WebAssembly bindings and a browser playground. |

## Build and test

```sh
cd tools/spec && cargo test
cd tools/cli && cargo run -- svg sample.json -o sample.svg
cd tools/cli && cargo run -- layout sample.json
cd tools/web && cargo test
```

## Playground

```sh
cd tools/web
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/tpt_layout_web.wasm
python -m http.server 8000   # then open http://localhost:8000
```

`pkg/` is generated and not committed. The page needs `wasm-bindgen` to match the crate version in
`web/Cargo.lock`; install the matching CLI with `cargo install wasm-bindgen-cli --version <version>`.
