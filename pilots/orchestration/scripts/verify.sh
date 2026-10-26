#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail
cd "$(dirname "$0")/.."
command -v cargo >/dev/null || { echo 'Rust with cargo, rustfmt and clippy is required.' >&2; exit 127; }
mkdir -p target/validation
exec > >(tee target/validation/verify.log) 2>&1
printf '\n== Toolchain ==\n'
rustc --version
cargo --version
printf '\n== Architecture/source checks ==\n'
python3 check_boundaries.py
printf '\n== Format authored source ==\n'
# This intentionally formats the previously uncompiled source before testing.
cargo fmt --all
printf '\n== Actual Rust unit, adapter, and separate-process tests ==\n'
cargo test --workspace --all-targets --locked
printf '\n== Clippy ==\n'
if [[ "${CLIPPY_STRICT:-0}" == 1 ]]; then
  cargo clippy --workspace --all-targets --locked -- -D warnings
else
  cargo clippy --workspace --all-targets --locked
fi
printf '\n== Build executable ==\n'
cargo build --release -p molten-orchestrator-cli --locked
printf '\nCompleted. Record Cargo.lock and this log with your validation results.\n'
