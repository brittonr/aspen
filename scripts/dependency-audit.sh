#!/usr/bin/env bash
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"
as_of="${1:-$(date -u +%F)}"
shift || true
metadata="$(mktemp)"
policy="$(mktemp)"
trap 'rm -f "$metadata" "$policy"' EXIT
cargo metadata --locked --format-version 1 > "$metadata"
nickel export --format json config/dependency-audit/policy.ncl > "$policy"
cargo -q -Zscript tools/dependency-audit.rs \
  --metadata "$metadata" \
  --policy "$policy" \
  --as-of "$as_of" \
  "$@"
