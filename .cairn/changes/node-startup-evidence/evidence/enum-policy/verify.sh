#!/bin/sh
# Review-only probes: a doc-marker false clean is a defect, never admission.
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
cd "$out"
b3sum "$driver" "$library" "$compiler/bin/rustc" "$self/fixture.rs" > before.txt
probe() {
    name=$1 tool=$2 expected=$3
    shift 3
    set +e
    env -i PATH="$compiler/bin:/run/current-system/sw/bin" HOME="$out/home" LD_LIBRARY_PATH="$compiler/lib" \
        CARGO_PRIMARY_PACKAGE=1 DYLINT_NO_DEPS=1 \
        DYLINT_LIBS="[\"$out/lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so\"]" \
        DYLINT_RUSTFLAGS='-D unknown_lints -D fragile_exhaustive_enum_match' \
        "$tool" --sysroot "$compiler" --crate-type lib --edition 2024 --emit=metadata \
        -o "$name.rmeta" "$self/fixture.rs" "$@" > "$name.stdout" 2> "$name.stderr"
    status=$?
    set -e
    printf '%s\n' "$status" > "$name.exit"
    test "$status" -eq "$expected"
    if grep -E 'internal compiler error|thread .*panicked' "$name.stderr"; then exit 1; fi
    printf '%s: expected exit%s\n' "$name" "$status"
}
probe plain "$compiler/bin/rustc" 0
probe unmarked "$driver" 101
test "$(grep -c 'error: same-crate enum' unmarked.stderr)" -eq 4
probe expanded "$compiler/bin/rustc" 1 --cfg future
test "$(grep -c 'error\[E0004\]' expanded.stderr)" -eq 4
probe sealed "$driver" 0 --cfg real_marker
test ! -s sealed.stderr
probe sealed-expanded "$compiler/bin/rustc" 1 --cfg real_marker --cfg future
test "$(grep -c 'error\[E0004\]' sealed-expanded.stderr)" -eq 4
probe doc-only "$driver" 0 --cfg doc_marker
test ! -s doc-only.stderr
b3sum "$driver" "$library" "$compiler/bin/rustc" "$self/fixture.rs" > after.txt
cmp before.txt after.txt
printf '%s\n' 'REPRODUCED DEFECT: doc text alone suppresses four diagnostics. Not acceptable sealing evidence.'
