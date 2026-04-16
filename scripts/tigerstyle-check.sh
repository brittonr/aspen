#!/usr/bin/env bash
# Run Tiger Style lints on Aspen via cargo-tigerstyle.
#
# In `nix develop`, cargo-tigerstyle is available directly.
# Outside the devShell, builds cargo-tigerstyle from the sibling tigerstyle repo.
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

Runs all 32 Tiger Style lints. Configuration in dylint.toml.

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

if command -v cargo-tigerstyle >/dev/null 2>&1; then
  echo "[tigerstyle] using cargo-tigerstyle from PATH"
  exec cargo-tigerstyle check "$@"
fi

# Fallback: build from flake
echo "[tigerstyle] cargo-tigerstyle not in PATH, building from flake..."
exec nix run ../tigerstyle#cargo-tigerstyle -- check "$@"
