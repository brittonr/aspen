#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Authored rail: actual invocation is required before claiming qualification.
set -euo pipefail
INVOCATION_CWD="$PWD"
printf -v INVOCATION '%q ' bash "$0" "$@"
cd "$(dirname "$0")/../../.."
ROOT="$PWD"
OUT="${1:-$ROOT/pilots/orchestration/target/progress-evidence}"
mkdir -p "$OUT"
RUN="$(mktemp -d "$OUT/run.XXXXXX")"
chmod 700 "$RUN"
printf 'cwd=%s\ncommand=%s\n' "$INVOCATION_CWD" "$INVOCATION" > "$RUN/invocation.txt"
trap 'status=$?; printf "exit=%s\n" "$status" > "$RUN/exit.txt"' EXIT
export MOLTEN_BATCH_EVIDENCE_DIR="$RUN/public-observations"
mkdir "$MOLTEN_BATCH_EVIDENCE_DIR"
{
  printf 'product=Aspen\nprofile=standalone-single-controller-import-free-component-batch-v1\n'
  printf 'scope=same-host-separate-process; no machine-loss or cross-host durability qualification\n'
  printf 'source_head='; git rev-parse HEAD
  printf 'kernel='; uname -srm
  rustc --version --verbose
  cargo --version --verbose
  printf 'filesystem='; stat -f -c '%T' "$RUN"
  for name in CARGO_TARGET_DIR CARGO_BUILD_JOBS CARGO_INCREMENTAL CARGO_PROFILE_DEV_DEBUG CARGO_PROFILE_TEST_DEBUG CARGO_NET_OFFLINE CC CXX; do
    printf '%s=%s\n' "$name" "${!name-}"
  done
  printf 'unmeasured=physical-host-loss,LAN-transfer,peak-memory,per-phase-scheduling-cost\n'
} > "$RUN/environment.txt"
# Include untracked authored source too; HEAD/diff alone is not exact-source evidence.
{
  git ls-files -z --cached --others --exclude-standard -- .cargo rust-toolchain.toml flake.lock pilots/orchestration crates/molten-core/src/committed_stream
  if [[ -f pilots/orchestration/examples/render-guest/Cargo.lock ]]; then printf '%s\0' pilots/orchestration/examples/render-guest/Cargo.lock; fi
} | sort -zu > "$RUN/source-files.nul"
tar --null -T "$RUN/source-files.nul" -cf "$RUN/source.tar"
b3sum "$RUN/source.tar" > "$RUN/source.blake3"
CARGO="pilots/orchestration/Cargo.toml"
cargo test --locked --manifest-path "$CARGO" -p molten-orchestrator-adapters --test progress -- --nocapture \
  2>&1 | tee "$RUN/storage.log"
cargo test --locked --manifest-path "$CARGO" -p molten-orchestrator-cli --test progress_e2e -- \
  --nocapture --test-threads=1 --skip progress_retention_overflow_requires_a_new_consistent_snapshot \
  2>&1 | tee "$RUN/process.log"
cargo test --locked --manifest-path "$CARGO" -p molten-orchestrator-cli --test progress_e2e \
  progress_retention_overflow_requires_a_new_consistent_snapshot -- --exact --nocapture --test-threads=1 \
  2>&1 | tee "$RUN/retention.log"
printf 'Retained actual source/environment/outcomes: %s\n' "$RUN"
