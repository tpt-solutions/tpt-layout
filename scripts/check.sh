#!/usr/bin/env sh
# CI-style check. Run from the repository root: sh scripts/check.sh
set -eu

echo "== no_std build (default features)"
cargo build --no-default-features

echo "== std feature build"
cargo build --features std

echo "== tests (unit, integration, golden, pagination hash, randomized)"
cargo test --all-targets

echo "== clippy"
cargo clippy --all-targets -- -D warnings
echo "== fmt"
cargo fmt --check
echo "== doc"
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

echo "== examples"
cargo build --examples

echo "== unsafe audit (only the crate-level forbid may mention it)"
if grep -rn "unsafe" src/ --include=*.rs | grep -v "forbid(unsafe_code)"; then
    echo "unsafe code found"
    exit 1
fi

echo "== wasm32 build (no_std)"
if rustup target list --installed 2>/dev/null | grep -qx 'wasm32-unknown-unknown'; then
    cargo build --no-default-features --target wasm32-unknown-unknown
else
    echo "skipped: wasm32-unknown-unknown not installed (rustup target add wasm32-unknown-unknown)"
    exit 1
fi

echo "all checks passed"
