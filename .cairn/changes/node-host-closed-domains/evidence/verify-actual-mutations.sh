#!/bin/sh
# CASES contains full node-host crate copies: baseline/store/namespace/observation.
# Each non-baseline copy adds only Future to the named actual enum declaration.
# DEPS is an existing compatible March-21 dependency metadata directory; never build it here.
set -eu
umask 077
[ "$#" -eq 6 ] || { echo 'usage: verify-actual-mutations.sh OUT CASES DRIVER LIBRARY COMPILER DEPS' >&2; exit 2; }
out=$1 cases=$2 driver=$3 library=$4 compiler=$5 deps=$6
for input in "$@"; do case "$input" in /*) ;; *) exit 2;; esac; done
test -x "$driver"
test -f "$library"
test -x "$compiler/bin/rustc"
self=$(CDPATH= cd "$(dirname "$0")" && pwd -P)/$(basename "$0")
# Reject missing or ambiguous dependencies instead of selecting an arbitrary artifact.
set -- "$deps"/libcap_std-*.rmeta
test "$#" -eq 1
test -f "$1"
cap_std=$1
set -- "$deps"/libcap_fs_ext-*.rmeta
test "$#" -eq 1
test -f "$1"
cap_fs_ext=$1
set -- "$deps"/libcap_tempfile-*.rmeta
test "$#" -eq 1
test -f "$1"
cap_tempfile=$1
set -- "$deps"/libmolten_core-*.rmeta
test "$#" -eq 1
test -f "$1"
molten_core=$1
mkdir "$out"
mkdir "$out/home" "$out/tmp" "$out/lints"
ln -s "$library" "$out/lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so"
cd "$out"
find "$cases" -type f -exec b3sum '{}' + > sources-before.txt
b3sum "$self" > script-before.txt
b3sum "$driver" "$library" "$compiler/bin/rustc" "$cap_std" "$cap_fs_ext" "$cap_tempfile" "$molten_core" > before.txt
probe() {
    case_name=$1 mode=$2 expected=$3 errors=$4
    tool="$compiler/bin/rustc"
    if [ "$mode" = active ]; then tool=$driver; fi
    name="$case_name-$mode"
    set +e
    env -i PATH="$compiler/bin:/run/current-system/sw/bin" HOME="$out/home" TMPDIR="$out/tmp" LD_LIBRARY_PATH="$compiler/lib" \
        CARGO_MANIFEST_DIR="$cases/$case_name" CARGO_PRIMARY_PACKAGE=1 DYLINT_NO_DEPS=1 DYLINT_TOML='' \
        DYLINT_LIBS="[\"$out/lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so\"]" \
        DYLINT_RUSTFLAGS='-D unknown_lints -D fragile_exhaustive_enum_match' \
        "$tool" --sysroot "$compiler" --crate-type lib --crate-name molten_node_host --edition 2024 --emit=metadata \
        --check-cfg 'cfg(dylint_lib, values("octet"))' --check-cfg 'cfg(test)' \
        -L "dependency=$deps" --extern "cap_std=$cap_std" --extern "cap_fs_ext=$cap_fs_ext" \
        --extern "cap_tempfile=$cap_tempfile" --extern "molten_core=$molten_core" \
        -o "$name.rmeta" "$cases/$case_name/src/lib.rs" > "$name.stdout" 2> "$name.stderr"
    status=$?
    set -e
    printf '%s\n' "$status" > "$name.exit"
    test "$status" -eq "$expected"
    if grep -E 'internal compiler error|thread .*panicked' "$name.stderr"; then exit 1; fi
    count=$(grep -c '^error\[E0004\]' "$name.stderr" || true)
    test "$count" -eq "$errors"
    if [ "$errors" -gt 0 ]; then grep -q 'Future.*not covered' "$name.stderr"; fi
    case "$case_name" in
        store) grep -q '/src/local_store/mod.rs:' "$name.stderr";;
        namespace) grep -q '/src/node/state/authority.rs:' "$name.stderr";;
        observation) grep -q '/src/node/state/filesystem.rs:' "$name.stderr"; grep -q '/src/node/state/namespace.rs:' "$name.stderr";;
    esac
    printf '%s: exit%s E0004=%s\n' "$name" "$status" "$count"
}
probe baseline plain 0 0
probe baseline active 0 0
for case_name in store namespace observation; do
    errors=1
    if [ "$case_name" = observation ]; then errors=2; fi
    probe "$case_name" plain 1 "$errors"
    probe "$case_name" active 101 "$errors"
done
b3sum "$driver" "$library" "$compiler/bin/rustc" "$cap_std" "$cap_fs_ext" "$cap_tempfile" "$molten_core" > after.txt
cmp before.txt after.txt
find "$cases" -type f -exec b3sum '{}' + > sources-after.txt
b3sum "$self" > script-after.txt
cmp sources-before.txt sources-after.txt
cmp script-before.txt script-after.txt
printf '%s\n' 'PASS: eight actual node-host metadata probes; no whole-workspace or runtime acceptance.'
