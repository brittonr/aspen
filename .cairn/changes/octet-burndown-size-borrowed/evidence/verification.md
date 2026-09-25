# Verification: Octet burn-down, sink parameters instead of borrowed `&mut Vec`

Base: `105c68f24` (C3a sync/archive) on `integration/stack-20260925` (`70a9f0b53`). Octet: pinned `octet-toolchain`
`fc38f593`. Private `CARGO_TARGET_DIR`; `nice -n 10`, `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`. Cairn: the installed
store binary `/nix/store/z1a7pddlrs9myzyypm10v8ljd0ipfkhd-cairn-0.1.0/bin/cairn`, run with the explicit
`cairn-policy/generated/cairn-policy.json` policy.

## Octet

| Scope | Base findings | After | `borrowed_argument_types` findings (sites) |
|---|---:|---:|---:|
| workspace (`cargo octet check`) | 3052 | 2692 | 360 (180) → 0 |
| `-p molten --lib` | 1287 | 1111 | 176 (176) → 0 |

`octet-lint-diff.txt` shows that no other family changed.

The first candidate run added 2 `unbounded_collection_growth` sites in `molten-node-host`. Inlining the push helpers
had left a `?`-propagated capacity check, which the lint does not treat as a guard. Each listing loop now uses the
recognized direct guard `if collection.len() >= MAX_LOCAL_STORE_ENTRIES { return Err(entry_limit_error()); }`, and the
rerun is level. A per-file site comparison finds no new `path_segment_repetition`, `excessive_file_length`,
`function_length`, or `unbounded_collection_growth` site.

## Shape of the change

- All 180 flagged parameters were `&mut Vec<T>`, and every changed function is private.
- Library helpers take `&mut impl crate::bounded::VecSink<T>`, the existing crate-private sink that the bounded-growth
  helpers already accept. Their bodies call `push_item`, `item_count`, and `extend_cloned_items`, plus the new
  `extend_items`, used at 12 call sites in 6 helpers.
- The two `molten` binary helpers take `&mut impl Extend<T>`, because the library trait is not visible to the binary.
- Ownership changes:
  - `normalize_refs` takes and returns the owned `Vec`, because it sorts and deduplicates.
  - `visit_structural_value` owns its `["$"]` path stack. Its only caller dropped the stack after the call.
- `validate_send_message` takes its diagnostics sink positionally, next to its C3a input struct.
- The node-host `push_bounded_entry` and `push_bounded_name` are inlined with direct guards. The collection never
  exceeds the maximum, so the denial message is byte-identical: `local store entry count {MAX+1} exceeds maximum {MAX}`.

## Rust gates

- `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace --no-fail-fast`: exit 0, 2091 passed, 0 failed, 0 ignored.
- `cargo test --test fabricboundarycompat`: 5 of 5 passed. `cargo test --lib fabric_execution::`: 10 of 10 passed.
- The diff adds no `allow`. `dylint.toml`, baselines, and quarantine files are unchanged, and no `as` cast is added.

## Byte identity against the base binary (`105c68f24`)

- `fixture-receipt-comparison.txt`: all 6 `examples/*.preserves` harness suites give identical exit codes and report
  and gate-receipt BLAKE3 hashes.
- `simulation-fixture-comparison.txt`: `molten fabric-simulation run` (100 files) and `shrink` (3) are identical.

## Flake checks

`flake-checks.txt` lists the touched-surface checks built with `nix build .#checks.x86_64-linux.<name>`. All 12 exit 0:
- `molten-node-host`, `cap-std-store-authority`, `materialization-authority`, and `node-state-authority`.
- The `native-system-extension`, `world-distribution`, and `world-authority` `*-octet-deny-all` checks.
- `wasm-component-profile`, `wasm-component-performance-profile`, and `release-profile-validation`.
- `fabric-port-boundaries` and `requirement-traceability-gate`.

## Review and lifecycle

The no-spec plan review approves the proposal, design, acceptance, and tasks with no findings. The proposal, design,
and tasks gates pass, and `cairn validate --strict` reports no issues.
