#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly TMPDIR_PATH="${TMPDIR:-/run/user/${UID}/tmp}"
readonly CARGO_ENV_PREFIX=(env -u CARGO_INCREMENTAL RUSTC_WRAPPER= TMPDIR="${TMPDIR_PATH}")
readonly CLIPPY_SCOPE=(
  -p aspen-time
)
readonly RUSTDOC_SCOPE=(
  -p aspen-time
)

run_clippy_policy() {
  (
    cd "${REPO_ROOT}"
    "${CARGO_ENV_PREFIX[@]}" cargo clippy "${CLIPPY_SCOPE[@]}" --all-targets -- -D warnings
  )
}

run_rustdoc_policy() {
  (
    cd "${REPO_ROOT}"
    RUSTDOCFLAGS="-D warnings" "${CARGO_ENV_PREFIX[@]}" cargo doc --no-deps "${RUSTDOC_SCOPE[@]}"
  )
}

main() {
  local mode="${1:-all}"
  case "${mode}" in
    clippy)
      run_clippy_policy
      ;;
    rustdoc)
      run_rustdoc_policy
      ;;
    all)
      run_clippy_policy
      run_rustdoc_policy
      ;;
    *)
      printf 'usage: %s [all|clippy|rustdoc]\n' "$0" >&2
      return 64
      ;;
  esac
}

main "$@"
