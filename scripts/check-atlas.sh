#!/usr/bin/env bash
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
python3 scripts/test_generate_atlas.py
nickel export --format json docs/atlas.ncl > "$tmp"
python3 scripts/generate-atlas.py "$tmp" --check
