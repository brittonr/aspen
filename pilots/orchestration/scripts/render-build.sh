#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail
cd "$(dirname "$0")/.."
# Use render/shell.nix: the toolchain already contains the guest target.
# Never mutate ambient rustup configuration or substitute WASI imports.
command -v wasm-tools >/dev/null || { echo 'Use the render/shell.nix shell (wasm-tools required).' >&2; exit 1; }
TARGET="${CARGO_TARGET_DIR:-$(pwd)/target}"
export CARGO_TARGET_DIR="$TARGET"
cargo build --locked --release -p molten-orchestrator-cli -p molten-render-pilot
cargo build --locked --release --target wasm32-unknown-unknown --manifest-path examples/render-guest/Cargo.toml
mkdir -p "$TARGET/guests"
# Wrap the intended import-free core module; never strip imports or widen admission.
wasm-tools component new "$TARGET/wasm32-unknown-unknown/release/orchestrator_render_guest.wasm" -o "$TARGET/guests/render.component.wasm"
wasm-tools validate "$TARGET/guests/render.component.wasm"
wasm-tools component wit "$TARGET/guests/render.component.wasm" > "$TARGET/guests/render.component.wit"
python3 - "$TARGET/guests/render.component.wit" <<'PY'
import pathlib,re,sys
world=pathlib.Path(sys.argv[1]).read_text()
if re.search(r"(?m)^\s*import\b",world):
    raise SystemExit("Renderer exposes component imports; the import-free runtime profile denies this artifact.")
PY
printf 'Guest component: %s\n' "$TARGET/guests/render.component.wasm"
