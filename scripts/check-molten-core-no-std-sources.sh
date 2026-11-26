#!/usr/bin/env bash
# r[impl molten.project.molten_core_no_std]
# Source-level half of the molten-core no_std gate. Compiling with `--no-default-features` rejects any
# `std` path only while `std` is not linked; this guard rejects the source forms that would link it or
# would hide `std` use behind a feature or test cfg in a pure module:
#   - any `extern crate` except `extern crate alloc;` in `lib.rs`;
#   - any `std` path or `use std` outside the `std`-gated modules (`live_binding`, `world_merge`,
#     `world_branch_authority`) and test files (`tests.rs`, `**/tests/**`).
# Comment lines are ignored.
# Usage: check-molten-core-no-std-sources.sh [MOLTEN_CORE_SRC]   (default: crates/molten-core/src)
set -euo pipefail

src="${1:-crates/molten-core/src}"
if [ ! -f "$src/lib.rs" ]; then
  echo "check-molten-core-no-std-sources: $src/lib.rs not found" >&2
  exit 2
fi
cd "$src"

comment='^[^:]+:[0-9]+:[[:space:]]*//'
allowed_extern='^\./lib\.rs:[0-9]+:extern crate alloc;$'
std_gated='^\./(live_binding|world_merge|world_branch_authority)(/|\.rs:)'
test_paths='^\./([^:]*/)?(tests/[^:]*|tests\.rs):'
std_path='(^|[^A-Za-z0-9_])std[[:space:]]*::|(^|[^A-Za-z0-9_])use[[:space:]]+(::)?std([[:space:]]*;|[[:space:]]+as[[:space:]])'

status=0
externs="$(grep -rnE --include='*.rs' '(^|[^A-Za-z0-9_])extern[[:space:]]+crate([^A-Za-z0-9_]|$)' . \
  | grep -vE "$comment" | grep -vE "$allowed_extern" || true)"
if [ -n "$externs" ]; then
  echo "molten-core may only declare \`extern crate alloc;\` in lib.rs:" >&2
  echo "$externs" >&2
  status=1
fi

std_uses="$(grep -rnE --include='*.rs' "$std_path" . \
  | grep -vE "$comment" | grep -vE "$std_gated" | grep -vE "$test_paths" || true)"
if [ -n "$std_uses" ]; then
  echo "molten-core pure modules must use core/alloc, not std:" >&2
  echo "$std_uses" >&2
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo "check-molten-core-no-std-sources: no std linkage or std paths in pure molten-core modules"
fi
exit "$status"
