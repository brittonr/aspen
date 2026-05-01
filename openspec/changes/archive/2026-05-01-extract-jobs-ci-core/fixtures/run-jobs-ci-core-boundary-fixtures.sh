#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)
positive="$repo_root/openspec/changes/archive/2026-05-01-extract-jobs-ci-core/fixtures/jobs-ci-core-portable-smoke/Cargo.toml"
negative="$repo_root/openspec/changes/archive/2026-05-01-extract-jobs-ci-core/fixtures/jobs-ci-runtime-negative/Cargo.toml"
forbidden=(
  aspen
  aspen-jobs
  aspen-ci
  aspen-rpc-handlers
  aspen-job-handler
  aspen-ci-handler
  aspen-dogfood
  aspen-ci-executor-shell
  aspen-ci-executor-vm
  aspen-ci-executor-nix
  aspen-jobs-worker-shell
  tokio
  iroh
  redb
  aspen-transport
)

printf '== positive fixture: cargo check ==\n'
cargo check --manifest-path "$positive"

printf '\n== positive fixture: dependency boundary ==\n'
tree=$(cargo tree --manifest-path "$positive" -e normal --prefix none)
printf '%s\n' "$tree"
for crate in "${forbidden[@]}"; do
  if grep -E "^${crate}( |$)" <<<"$tree" >/dev/null; then
    printf 'forbidden dependency present: %s\n' "$crate" >&2
    exit 1
  fi
done
printf 'no forbidden runtime dependencies found in positive fixture\n'

printf '\n== negative fixture: runtime imports must fail ==\n'
set +e
negative_output=$(cargo check --manifest-path "$negative" 2>&1)
negative_status=$?
set -e
printf '%s\n' "$negative_output"
if [ "$negative_status" -eq 0 ]; then
  printf 'negative fixture unexpectedly compiled\n' >&2
  exit 1
fi
for crate in aspen_ci aspen_jobs aspen_ci_executor_shell aspen_ci_executor_vm aspen_ci_executor_nix aspen_job_handler aspen_ci_handler aspen_jobs_worker_shell; do
  if ! grep -F "use of unresolved module or unlinked crate \`${crate}\`" <<<"$negative_output" >/dev/null; then
    printf 'negative fixture did not prove %s unavailable\n' "$crate" >&2
    exit 1
  fi
done
printf 'negative fixture failed for expected unresolved runtime imports\n'
