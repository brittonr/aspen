# Caller-built chunk manifest count verification

## Reproduction and source-owned behavior

The clean canonical owner branch `preflight-chunk-manifest-count-20260929` adds a single existing-limit check to `validate_chunk_manifest_identity`, before either public caller can allocate range refs or traversal-plan collections. The parser already uses `MAX_CHUNK_STORE_CHUNKS = 65,536` and the same `"chunk manifest chunks"` count error; no policy, parser, schema, pin, waiver, or accepted spec changed.

Before the source guard, `nix develop --command cargo test --lib chunk_manifest_count_preflight -- --test-threads=1` failed both regressions: 65,537 caller-built chunks were admitted by `verify_remote_range_readback`, and `plan_chunk_traversal_sync` reported the unsupported strategy instead of rejecting the excess count. After the guard, both passed. After adding an admitted malformed-ref precedence assertion, `nix develop --command cargo test -p molten --lib blocks::semantic_store::tests:: -- --test-threads=1` passed all 21 focused chunk-store tests. The exact 65,536 boundary read back verified `b"abcd"` using one four-byte slice; 65,537 was denied before the range collection. Admitted invalid refs still take precedence over strategy failures.

No production CLI calls either public helper internally. A separately compiled disposable Rust package depending on `molten::blocks` created a real two-chunk manifest via `put_bytes` / `parse_manifest_value`, verified the first four bytes through public readback, then grew only the chunk-ref metadata to 65,537 and observed `chunk manifest chunks count 65537 exceeds maximum 65536` from **both** public helper calls. Output: `external public API: verified small range and rejected 65537 chunks on both entry points`. Its temporary source project was removed after exercise; Cargo artifacts remain in the shared target cache.

## Isolated scoped checks and composition

- `nix develop --command cargo fmt --package molten --check`: exit 0.
- Initial all-target Clippy on the shared global Cargo target failed with five unrelated missing `molten_core` constructor errors while an independently locked external Cargo package used that target. Without changing those sources, the **independent** `CARGO_TARGET_DIR=/home/brittonr/.cargo-target/aspen-chunk-owner-clippy-20260929 nix develop --command cargo clippy -p molten --all-targets -- -D warnings` passed. Shared-target collision as the cause is an inference, not a proven compiler root cause.
- `nix develop --command cargo octet check --artifact-dir /tmp/molten-chunk-manifest-owner-lib-20260929 -- -p molten --lib`: `status.json` reports `warning-only`, **835 warnings, zero errors and zero autofixable**. This library-only Octet profile is not the original full-catalog gate and cannot be subtracted from its 3,615 rows.
- `nix develop --command cairn validate --root . --policy cairn-policy/generated/cairn-policy.json --strict`: 53 isolated packages, empty issue lists, valid. The change-local `cairn gate proposal`, `design` and `tasks` commands each returned `valid: true`, advisory, no issues; they are not acceptance or release receipts.
- Disjoint 63-package composition borrowed only the one-line count guard, two tests, docs and this change-local Cairn. Its independent-target `cargo test -p molten --lib chunk_manifest_count_preflight -- --test-threads=1` passed both cases; composed library Clippy with `-D warnings` passed; scoped `cargo fmt --package molten --check` exited 0 (Nix emitted an ignored busy eval-cache warning). Composed `cargo octet check --artifact-dir /tmp/molten-chunk-manifest-composition-lib-20260929 -- -p molten --lib` reports `warning-only`, **1,493 warnings**, zero errors/autofixable; its profile differs from the owner's and the original full catalog. Strict Cairn validation reported **64** composed packages, empty issue lists and `valid: true`.

No full known-failing strict-ci retry, waiver, baseline, accepted-spec, release-policy, pin, or unrelated X-site change is claimed. The next X sites require separate owners.

## Supplied readback tuple follow-up

The second review identified an independent path after manifest admission:
`input.chunk_bytes.iter().cloned().collect::<OrderedMap<_, _>>()` cloned
every caller-supplied `String` and `Vec<u8>`, even for irrelevant extras, while
the input slice lacked a count cap. Before the source fix, the permanent
`remote_readback_preflights_supplied_chunk_count` regression failed at
`65,537 supplied tuples must be rejected before lookup` (`nix develop
--command cargo test -p molten --lib remote_readback_preflights_supplied_chunk_count
-- --test-threads=1`, 1 failed); the 65,536 admitted case and overflowing
range precedence in that same test passed before reaching the failing
assertion. Only tiny unused buffers and one four-byte required buffer were
used.

The readback entry point now checks the supplied tuple count immediately
after existing manifest admission, using `MAX_CHUNK_STORE_CHUNKS = 65,536`.
It builds `OrderedMap<&str, &[u8]>` from the caller's input without cloning
refs or buffers; ordered-map collection still overwrites duplicate keys
with the last supplied tuple. Admitted inputs still check required-range
arithmetic before lookup. Required buffers are length-checked before
hashing against existing `MAX_CHUNK_STORE_OBJECT_BYTES = 1 GiB`, and
verified output is appended through `extend_bytes_bounded` under that
same existing byte limit. `hash_chunk` formats numeric `chunk_size` into
its domain and hashes the provided slice, without allocating from the
numeric size; required-range collection uses checked `u64` arithmetic
and at most the already-capped manifest chunk count. No new byte/count
policy, public signature, receipt shape, pin, waiver, accepted spec, or
unrelated X site changed.

After the source fix, isolated `nix develop --command cargo test -p
molten --lib blocks::semantic_store::tests:: -- --test-threads=1` passed
**23/23**, including the exact-bound, excess tuple, admitted overflowing
range, duplicate-last deny/empty-bytes and pass/`b"abcd"` cases. Scoped
`cargo fmt --package molten --check` exited 0 in the isolated owner and
the 64-package composition.

A separately compiled disposable Rust package consumed the public
`molten::blocks` API, created an actual two-chunk manifest via `put_bytes`
and `parse_manifest_value`, and verified the last of two supplied duplicate
buffers decides `deny`/empty bytes when corrupt and `pass`/`b"abcd"` when
valid. With 65,535 irrelevant empty tuples and one required four-byte
tuple, 65,536 passed; adding one irrelevant empty tuple returned
`remote byte-source supplied chunks count 65537 exceeds maximum 65536`.
The executable printed `external supplied-byte readback: last duplicate
wins; exact 65536 passes; 65537 denies`. Temporary source was removed.

Independent-target owner `cargo clippy -p molten --all-targets --
-D warnings` passed (Nix emitted an ignored busy eval-cache warning).
Scoped owner `cargo octet check --artifact-dir
/tmp/molten-chunk-supplied-owner-lib-20260929 -- -p molten --lib`
returned `warning-only` with **835 warnings**, zero errors and zero
autofixable findings. As with the first pass, a scoped library-only
warning-only Octet run does not prove the full strict-ci gate.

In the composition, independent-target `cargo test -p molten --lib
remote_readback_ -- --test-threads=1` passed both changed readback
regressions, including the duplicate-verification test after its final
name change. Scoped formatting exited 0 after that rename in both trees.
The isolated proposal/design/tasks Cairn gates each reported advisory
`valid: true` with no issues. Final strict Cairn validation of the extended
change-local requirement, scenarios, and evidence passed with **53** isolated
and **64** composed packages, empty issue lists and `valid: true`; these
are structural validation results, not release acceptance.

Independent-target composed `cargo clippy -p molten --lib -- -D
warnings` passed. An initial composed scoped Octet run in the shared
target reported `integration-failure` from a nightly incremental
compiler error in unchanged `molten-node-host`; an independent-target
retry with `CARGO_INCREMENTAL=0` is the relevant Octet evidence, rather
than treating that compiler failure as a source finding.

The independent-target retry `CARGO_INCREMENTAL=0
CARGO_TARGET_DIR=/home/brittonr/.cargo-target/aspen-chunk-composition-octet-20260929
nix develop --command cargo octet check --artifact-dir
/tmp/molten-chunk-supplied-composition-lib-retry-20260929 -- -p molten
--lib` exited 0 and its `status.json` reports `warning-only`, **1,493
warnings**, zero errors and zero autofixable findings. This scoped result
is not a full strict-ci retry and does not change the other owners'
critical-site catalog.

After the final test-name-only change, isolated `cargo test -p molten
--lib remote_readback_ -- --test-threads=1` passed **2/2**; the
composition ran the same final two tests successfully. No full strict-ci
profile, waiver, or accepted-spec mutation was exercised.
