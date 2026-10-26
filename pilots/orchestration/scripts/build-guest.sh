#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail
cd "$(dirname "$0")/.."
command -v wasm-tools >/dev/null || { echo 'Install wasm-tools to wrap the guest module as a component.' >&2; exit 1; }
cargo build --release --target wasm32-unknown-unknown --manifest-path examples/concat-guest/Cargo.toml
mkdir -p target/guests
wasm-tools component new examples/concat-guest/target/wasm32-unknown-unknown/release/orchestrator_concat_guest.wasm -o target/guests/concat.component.wasm
wasm-tools validate target/guests/concat.component.wasm
