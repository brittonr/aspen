#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Explicit component-dependent, same-host separate-process campaign.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "${RENDER_SKIP_BUILD:-0}" == 1 ]] || bash scripts/render-build.sh
TARGET="${CARGO_TARGET_DIR:-$PWD/target}"
export MOLTEN_RENDER_COMPONENT="$TARGET/guests/render.component.wasm"
[[ -f "$MOLTEN_RENDER_COMPONENT" ]] || { echo 'Build the real renderer component first.' >&2; exit 1; }
# Build-shell TMPDIR is removed on shell exit; retained observations need a separate root.
RUN="$(mktemp -d "${RENDER_RETAIN_ROOT:-/tmp}/aspen-render-faults.XXXXXX")"
chmod 700 "$RUN"
export MOLTEN_BATCH_EVIDENCE_DIR="$RUN/public-observations"
mkdir "$MOLTEN_BATCH_EVIDENCE_DIR"
REPO="$(git rev-parse --show-toplevel)"
{
  printf 'product=Aspen\nscope=same-host-separate-process; not physical-host-loss qualification\n'
  rustc --version --verbose
  cargo --version --verbose
  uname -srm
  stat -f -c 'artifact-filesystem=%T' "$RUN"
  b3sum "$MOLTEN_RENDER_COMPONENT"
} > "$RUN/environment.txt"
{
  git -C "$REPO" ls-files -z --cached --others --exclude-standard -- .cargo rust-toolchain.toml flake.lock pilots/orchestration crates/molten-core/src/committed_stream
  if [[ -f "$REPO/pilots/orchestration/examples/render-guest/Cargo.lock" ]]; then printf '%s\0' pilots/orchestration/examples/render-guest/Cargo.lock; fi
} | sort -zu > "$RUN/source-files.nul"
tar -C "$REPO" --null -T "$RUN/source-files.nul" -cf "$RUN/source.tar"
b3sum "$RUN/source.tar" > "$RUN/source.blake3"
cargo test --locked -p molten-orchestrator-cli --test progress_e2e real_renderer_ -- \
  --ignored --nocapture --test-threads=1 2>&1 | tee "$RUN/render-process-faults.log"
printf 'Retained actual renderer fault source/environment/outcomes: %s\n' "$RUN"
