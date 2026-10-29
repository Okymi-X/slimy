#!/usr/bin/env bash
# Compile the simulation to WASM and generate the JS glue next to the web app.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p slimy-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir apps/desktop/web/pkg target/wasm32-unknown-unknown/release/slimy_wasm.wasm
