#!/usr/bin/env bash
# Regression tests for run-forge-web demo repository preparation.
set -euo pipefail

LOCK_VERSION="7"

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
RUN_FORGE_WEB_SCRIPT="$REPO_ROOT/scripts/run-forge-web.sh"
DEMO_HELPERS="$REPO_ROOT/scripts/lib/run-forge-web-demo.sh"
TMP_ROOT=""

log() { printf '[run-forge-web-demo-test] %s\n' "$*"; }
ok() { printf '  ok: %s\n' "$*"; }
err() { printf '  error: %s\n' "$*" >&2; }

cleanup() {
  if [ -n "$TMP_ROOT" ]; then
    rm -rf "$TMP_ROOT"
  fi
}
trap cleanup EXIT

fail() {
  err "$*"
  exit 1
}

make_fake_nix() {
  local bin_dir="$1"
  local bash_path="${BASH:-}"

  if [ -z "$bash_path" ]; then
    bash_path=$(command -v bash)
  fi

  mkdir -p "$bin_dir"
  printf '#!%s\n' "$bash_path" > "$bin_dir/nix"
  cat >> "$bin_dir/nix" << 'INNER_NIX'
set -euo pipefail

LOCK_VERSION="${LOCK_VERSION:-7}"

if [ "${1:-}" != "--extra-experimental-features" ] || [ "${3:-}" != "flake" ] || [ "${4:-}" != "lock" ]; then
  printf 'unexpected nix invocation:' >&2
  printf ' %q' "$@" >&2
  printf '\n' >&2
  exit 1
fi

cat > flake.lock << INNER_LOCK
{
  "nodes": {},
  "root": "root",
  "version": $LOCK_VERSION
}
INNER_LOCK
INNER_NIX
  chmod +x "$bin_dir/nix"
}

configure_demo_git_identity() {
  git config user.name "Aspen Forge Demo Test"
  git config user.email "forge-demo-test@example.invalid"
}

# shellcheck source=scripts/lib/run-forge-web-demo.sh
# shellcheck disable=SC1091
source "$DEMO_HELPERS"

test_demo_commit_writes_required_lockfiles() {
  local repo_dir="$TMP_ROOT/positive-demo"
  local fake_bin="$TMP_ROOT/fake-bin"

  make_fake_nix "$fake_bin"
  mkdir -p "$repo_dir"
  (
    cd "$repo_dir"
    git init -q
    configure_demo_git_identity
    PATH="$fake_bin:$PATH" \
      LOCK_VERSION="$LOCK_VERSION" \
      CARGO_INCREMENTAL=1 \
      RUSTC_WRAPPER=sccache \
      commit_forge_web_demo_repo

    git ls-files --error-unmatch Cargo.lock >/dev/null
    git ls-files --error-unmatch flake.lock >/dev/null
    git ls-files --error-unmatch flake.nix >/dev/null
    git rev-parse --verify HEAD >/dev/null
    test -s Cargo.lock
    test -s flake.lock
  )
  ok "demo prep commits Cargo.lock and flake.lock despite inherited cargo wrapper"
}

test_missing_staged_file_rejected() {
  local repo_dir="$TMP_ROOT/negative-missing-lock"

  mkdir -p "$repo_dir"
  if (
    cd "$repo_dir"
    git init -q
    configure_demo_git_identity
    require_staged_file Cargo.lock >/tmp/run-forge-web-demo-test.err 2>&1
  ); then
    fail "require_staged_file accepted missing Cargo.lock"
  fi
  ok "missing staged lockfile is rejected"
}

test_demo_prep_is_not_nonfatal_if_block() {
  if awk '
    /Pushing demo content via git-remote-aspen/ { in_demo = 1 }
    in_demo && /^[[:space:]]*if \([[:space:]]*$/ { bad_if = 1 }
    in_demo && /commit_forge_web_demo_repo/ { saw_commit_helper = 1 }
    in_demo && /git -C "\$TMPGIT" push/ { saw_push = 1; exit !(saw_commit_helper && !bad_if) }
    END { if (!saw_push) exit 1 }
  ' "$RUN_FORGE_WEB_SCRIPT"; then
    ok "demo prep subshell is outside nonfatal push handling"
  else
    fail "demo prep helper call or push block is missing or nonfatal-wrapped"
  fi
}

main() {
  TMP_ROOT=$(mktemp -d)

  bash -n "$RUN_FORGE_WEB_SCRIPT"
  bash -n "$DEMO_HELPERS"
  test_demo_commit_writes_required_lockfiles
  test_missing_staged_file_rejected
  test_demo_prep_is_not_nonfatal_if_block
}

main "$@"
