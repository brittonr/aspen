#!/bin/sh
# Six reduced compiler-integration controls; no package or runtime acceptance.
set -eu
umask 077
[ "$#" -eq 5 ] || { echo 'usage: verify-marker-route.sh OUT DRIVER LIBRARY NIGHTLY STABLE' >&2; exit 2; }
out=$1 driver=$2 library=$3 nightly=$4 stable=$5
for input in "$@"; do case "$input" in /*) ;; *) exit 2;; esac; done
self=$(CDPATH= cd "$(dirname "$0")" && pwd -P)
for executable in "$driver" "$nightly/bin/rustc" "$stable/bin/rustc"; do
    test -x "$executable" || { printf 'missing executable: %s\n' "$executable" >&2; exit 2; }
done
test -f "$library" || { printf 'missing library: %s\n' "$library" >&2; exit 2; }
mkdir "$out"
mkdir "$out/home" "$out/lints"
ln -s "$library" "$out/lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so"
cd "$out"
b3sum "$driver" "$library" "$nightly/bin/rustc" "$stable/bin/rustc" "$self/marker-route.rs" "$self/verify-marker-route.sh" > before.txt
probe() {
    name=$1 compiler=$2 tool=$3 expected=$4
    shift 4
    set +e
    env -i PATH="$compiler/bin:/run/current-system/sw/bin" HOME="$out/home" LD_LIBRARY_PATH="$compiler/lib" \
        CARGO_PRIMARY_PACKAGE=1 DYLINT_NO_DEPS=1 DYLINT_TOML='' \
        DYLINT_LIBS="[\"$out/lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so\"]" \
        DYLINT_RUSTFLAGS='-D unknown_lints -D fragile_exhaustive_enum_match' \
        "$tool" --sysroot "$compiler" --crate-type lib --edition 2024 --emit=metadata \
        --check-cfg 'cfg(dylint_lib, values("octet"))' --check-cfg 'cfg(disable_marker)' --check-cfg 'cfg(future)' \
        -D warnings -o "$name.rmeta" "$self/marker-route.rs" "$@" > "$name.stdout" 2> "$name.stderr"
    status=$?
    set -e
    printf '%s\n' "$status" > "$name.exit"
    test "$status" -eq "$expected"
    if grep -E 'internal compiler error|thread .*panicked' "$name.stderr"; then exit 1; fi
    if [ "$expected" -eq 0 ]; then test ! -s "$name.stderr"; fi
    printf '%s: expected exit%s\n' "$name" "$status"
}
probe stable "$stable" "$stable/bin/rustc" 0
probe nightly "$nightly" "$nightly/bin/rustc" 0
probe active "$nightly" "$driver" 0
probe inactive-marker "$nightly" "$driver" 101 --cfg disable_marker
grep -q 'same-crate enum' inactive-marker.stderr
probe stable-future "$stable" "$stable/bin/rustc" 1 --cfg future
grep -q 'E0004' stable-future.stderr
probe active-future "$nightly" "$driver" 101 --cfg future
grep -q 'E0004' active-future.stderr
b3sum "$driver" "$library" "$nightly/bin/rustc" "$stable/bin/rustc" "$self/marker-route.rs" "$self/verify-marker-route.sh" > after.txt
cmp before.txt after.txt
printf '%s\n' 'PASS: six reduced route controls; no RUSTC_BOOTSTRAP; no production or startup acceptance.'
