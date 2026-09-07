#!/bin/sh
# Focused arithmetic diagnostic, not a complete source gate or startup approval.
set -eu
umask 077
[ "$#" -eq 4 ] || { echo 'usage: verify.sh OUT DRIVER LIBRARY COMPILER' >&2; exit 2; }
out=$1 driver=$2 library=$3 compiler=$4
for input in "$@"; do case "$input" in /*) ;; *) exit 2;; esac; done
self=$(CDPATH= cd "$(dirname "$0")" && pwd -P)
test -x "$driver" && test -f "$library" && test -x "$compiler/bin/rustc"
mkdir "$out"
mkdir "$out/home" "$out/lints"
ln -s "$library" "$out/lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so"
b3sum "$driver" "$library" "$compiler/bin/rustc" "$self"/fixtures/*.rs > "$out/before.txt"
probe() {
  name=$1 source=$2 tool=$3 expected=$4
  set +e
  env -i PATH="$compiler/bin:/run/current-system/sw/bin" HOME="$out/home" \
    LD_LIBRARY_PATH="$compiler/lib" CARGO_PRIMARY_PACKAGE=1 DYLINT_NO_DEPS=1 \
    DYLINT_LIBS="[\"$out/lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so\"]" \
    DYLINT_RUSTFLAGS='-D unknown_lints -D raw_arithmetic_overflow' \
    "$tool" --sysroot "$compiler" --crate-type lib --edition 2024 --emit=metadata \
    -o "$out/$name.rmeta" "$self/fixtures/$source.rs" > "$out/$name.stdout" 2> "$out/$name.stderr"
  status=$?
  set -e
  printf '%s\n' "$status" > "$out/$name.exit"
  test "$status" -eq "$expected"
  if grep -E 'internal compiler error|thread .*panicked' "$out/$name.stderr"; then exit 1; fi
  printf '%s: expected exit%s\n' "$name" "$status"
}
probe nested-rustc nested "$compiler/bin/rustc" 0
probe nested-dylint nested "$driver" 101
grep -q 'unprotected unsigned multiplication may overflow' "$out/nested-dylint.stderr"
probe literal literal "$driver" 0
probe runtime runtime "$driver" 101
grep -q 'unprotected unsigned addition may overflow' "$out/runtime.stderr"
probe overflow overflow "$compiler/bin/rustc" 1
grep -q 'E0080' "$out/overflow.stderr"
b3sum "$driver" "$library" "$compiler/bin/rustc" "$self"/fixtures/*.rs > "$out/after.txt"
cmp "$out/before.txt" "$out/after.txt"
printf '%s\n' 'PASS: five targeted probes; no complete-gate or startup claim.'
