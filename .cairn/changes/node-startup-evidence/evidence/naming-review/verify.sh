#!/bin/sh
# Defect reproduction, NOT acceptance of body-text or filename exemptions.
set -eu
[ "$#" -eq 4 ] || { echo 'usage: verify.sh OUT DRIVER LIBRARY COMPILER' >&2; exit 2; }
out=$1 driver=$2 library=$3 compiler=$4
for input in "$@"; do case "$input" in /*) ;; *) exit 2;; esac; done
self=$(CDPATH= cd "$(dirname "$0")" && pwd -P)
test -x "$driver"
test -f "$library"
test -x "$compiler/bin/rustc"
mkdir "$out"
mkdir "$out/home" "$out/tmp" "$out/lints"
ln -s "$library" "$out/lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so"
cd "$out"
b3sum "$driver" "$library" "$compiler/bin/rustc" "$self"/*.rs > before.txt
probe() {
    name=$1 expected=$2
    set +e
    env -i PATH="$compiler/bin:/run/current-system/sw/bin" HOME="$out/home" TMPDIR="$out/tmp" LD_LIBRARY_PATH="$compiler/lib" \
        CARGO_PRIMARY_PACKAGE=1 DYLINT_NO_DEPS=1 DYLINT_TOML='' \
        DYLINT_LIBS="[\"$out/lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so\"]" \
        DYLINT_RUSTFLAGS='-D unknown_lints -D path_segment_repetition' \
        "$driver" --sysroot "$compiler" --crate-name naming_probe --crate-type lib --edition 2024 --emit=metadata \
        -o "$name.rmeta" "$self/$name.rs" > "$name.stdout" 2> "$name.stderr"
    code=$?
    set -e
    printf '%s\n' "$code" > "$name.exit"
    test "$code" -eq "$expected"
    if grep -E 'internal compiler error|thread .*panicked' "$name.stderr"; then exit 1; fi
    if [ "$expected" -eq 0 ]; then test ! -s "$name.stderr"; fi
    printf '%s: exit%s\n' "$name" "$code"
}
probe plain 101
grep -q 'node_label.*repeats' plain.stderr
probe documented 0
probe body 0
probe registry 0
cmp "$self/plain.rs" "$self/registry.rs"
b3sum "$driver" "$library" "$compiler/bin/rustc" "$self"/*.rs > after.txt
cmp before.txt after.txt
printf '%s\n' 'REPRODUCED: body string and registry filename false cleans; no startup authority.'
