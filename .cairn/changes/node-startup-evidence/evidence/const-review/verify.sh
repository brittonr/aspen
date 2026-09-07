#!/bin/sh
# Diagnostic only: no builds of tools and no startup admission.
set -eu
out=${1:?OUT} driver=${2:?DRIVER} library=${3:?LIBRARY} compiler=${4:?COMPILER}
for path in "$out" "$driver" "$library" "$compiler"; do
  case "$path" in /*) ;; *) echo 'absolute paths required' >&2; exit 2;; esac
done
test ! -e "$out"
umask 077
mkdir -p "$out/home"
fixtures=$(CDPATH= cd -- "$(dirname -- "$0")/fixtures" && pwd)
cp "$fixtures/"*.rs "$out/"
ln -s "$library" "$out/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so"
alias="$out/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so"
b3sum "$driver" "$library" "$compiler/bin/rustc" > "$out/identities-before.txt"
"$compiler/bin/rustc" -vV > "$out/compiler.txt"
run() {
  name=$1 mode=$2 expected=$3
  program="$compiler/bin/rustc"
  if test "$mode" = lint; then program=$driver; fi
  set +e
  env -i PATH="$compiler/bin:/run/current-system/sw/bin" HOME="$out/home" \
    LD_LIBRARY_PATH="$compiler/lib" DYLINT_LIBS="[\"$alias\"]" DYLINT_NO_DEPS=1 \
    CARGO_PRIMARY_PACKAGE=1 DYLINT_RUSTFLAGS='-D unknown_lints -D missing_const_fn' \
    "$program" --sysroot "$compiler" --crate-name probe --crate-type lib --edition 2024 \
    --emit=metadata -o "$out/$name-$mode.rmeta" "$out/$name.rs" \
    > "$out/$name-$mode.stdout" 2> "$out/$name-$mode.stderr"
  status=$?
  set -e
  printf '%s\n' "$status" > "$out/$name-$mode.exit"
  if grep -q 'internal compiler error' "$out/$name-$mode.stderr"; then
    echo 'compiler crash is not a lint result' >&2; exit 1
  fi
  test "$status" -eq "$expected"
  printf '%s %s: exit %s\n' "$name" "$mode" "$status"
}
for name in display local locator positive; do
  run "$name" plain 0
  run "$name" lint 101
  grep -q 'private helper appears const-compatible' "$out/$name-lint.stderr"
  set +e
  diff -u "$out/$name.rs" "$out/$name-const.rs" > "$out/$name.diff"
  status=$?
  set -e
  test "$status" -eq 1
done
run display-const plain 1
grep -q 'error\[E0379\]' "$out/display-const-plain.stderr"
for prefix_case in local locator; do
  run "$prefix_case-const" plain 1
  grep -q 'error\[E0015\]' "$out/$prefix_case-const-plain.stderr"
done
run positive-const plain 0
run positive-const lint 0
b3sum "$driver" "$library" "$compiler/bin/rustc" > "$out/identities-after.txt"
cmp "$out/identities-before.txt" "$out/identities-after.txt"
printf '%s\n' '13 diagnostic checks passed; three reported const suggestions rejected by rustc. No startup authority.'
