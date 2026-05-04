#!/usr/bin/env bash
# Run Tiger Style lints on Aspen via cargo-tigerstyle.
#
# Runs the pinned upstream flake by default; set TIGERSTYLE_USE_PATH=1 to use a devShell binary.
#
# Configuration: dylint.toml (lint thresholds/levels) and
# [workspace.metadata.tigerstyle] in Cargo.toml (default scope/args).
#
# CI: `nix flake check` runs checks.tigerstyle automatically.
#
# Usage:
#   scripts/tigerstyle-check.sh                    # workspace, all targets
#   scripts/tigerstyle-check.sh -p aspen-core      # single crate
#   scripts/tigerstyle-check.sh -p aspen-raft -- --lib  # lib only

set -euo pipefail

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/tigerstyle-check.sh [cargo-tigerstyle check args]

Runs all Tiger Style lints. Configuration in dylint.toml.

Examples:
  scripts/tigerstyle-check.sh                           # full workspace
  scripts/tigerstyle-check.sh -p aspen-core             # single crate
  scripts/tigerstyle-check.sh -p aspen-raft -- --lib    # lib only
EOF
  exit 0
fi

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "error: not inside the Aspen git repository" >&2
  exit 1
}
cd "$repo_root"

if [[ "${TIGERSTYLE_USE_PATH:-0}" == "1" ]] && command -v cargo-tigerstyle >/dev/null 2>&1; then
  echo "[tigerstyle] using cargo-tigerstyle from PATH"
  exec cargo-tigerstyle check "$@"
fi

# Build from the pinned upstream flake
_TIGERSTYLE_FLAKE="${TIGERSTYLE_FLAKE:-github:brittonr/tigerstyle-rs}"
echo "[tigerstyle] running ${_TIGERSTYLE_FLAKE}#cargo-tigerstyle..."
exec nix run "${_TIGERSTYLE_FLAKE}#cargo-tigerstyle" -- check "$@"
