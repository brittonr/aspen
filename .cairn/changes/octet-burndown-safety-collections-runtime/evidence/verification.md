# Verification: Octet burn-down, collection growth in runtime and adapters

Base: `octet-burndown-safety-collections-validators` (`9008d70db`). Octet: pinned `octet-toolchain` `fc38f593`.
Private `CARGO_TARGET_DIR`; `nice -n 10`, `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`.

## Octet

| Scope | Base findings | After | `unbounded_collection_growth` findings (sites) |
|---|---:|---:|---:|
| workspace (`cargo octet check`) | 3372 | 3267 | 105 (56) → 0 (0) |
| `-p molten --lib` | 1437 | 1388 | 49 (49) → 0 (0) |

`octet-lint-diff.txt` shows that no other family changed. The first candidate run added one `function_length` site in
`content_store_adapter/local.rs` (`execute_local_stream_get`). The reservation was compacted, and the rerun is
level (`function_length` 204 → 204).

## Review of the collected repair

Structural repairs are the same as in the validators slice. The review changed four parts of the collected diff:

- `parse_run_index`: the collected bound was a literal `8_192`. It is now molten-core's `MAX_RUN_ARTIFACTS + 1`, and
  `MAX_RUN_ARTIFACTS` is now `pub`. An oversized index is still parsed far enough to reach the core
  too-many-artifacts diagnostic.
- The reference-world `max-choices` bound: the collected hunk saturated an unconvertible `u64` to `usize::MAX`. It now
  denies instead. A new test checks that exactly `max-choices` records are accepted and one past is denied. The denial
  arrives first as the core scheduler's `ChoiceBoundExceeded`.
- Materialization `list_regular_files_recursive` (`max_members`) and world-distribution closure commits
  (`max_closure_objects`) gained at-limit and one-past tests.
- `MAX_CLUSTER_MANIFEST_NODES` reuses `MAX_CLUSTER_LIFECYCLE_ITEMS`. `MAX_WORLD_HEAD_CONFLICT_RECORDS` (256) is the
  one new constant, because no existing limit covers stored conflict sets. Both have at-limit and one-past tests. The
  one-past case returns an error and no partial list.

## Rust gates

- `cargo fmt --check`: exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- Focused: `cargo test -p molten --lib -- fabric_simulation cluster_harness cluster:: materialization world_distribution
  world_head prolly_map content_store_adapter`: 68 passed, 0 failed.
- `cargo test --workspace --no-fail-fast`: exit 0, 2077 passed, 0 failed, 0 ignored. That is 2071 plus the six new bound
  tests.
- The diff adds no `allow` attribute. `dylint.toml`, baselines, and quarantine files are unchanged.

## Receipt identity

- `fixture-receipt-comparison.txt`: every `examples/*.preserves` suite run through `molten test run` and `test gate
  check` gives identical exit codes and BLAKE3 hashes of reports and gate receipts, on the base binary (`8cb948d92`) and
  this tree (6 of 6).
- `simulation-fixture-comparison.txt`: `molten fabric-simulation run` (100 artifacts) and `shrink` (3 artifacts) are
  byte-identical between base and candidate. `molten fabric-time run-fixture` differs in four live-clock observation
  files. The same four files differ between two runs of the base binary, so that difference is live-clock
  nondeterminism, not this change. The deterministic time artifacts are identical.

## Flake checks

`nix build .#checks.x86_64-linux.<check>` passed: `materialization-authority`, `world-distribution-octet-deny-all`,
`world-head-octet-deny-all`, `prolly-map-octet-deny-all`, `prolly-map-profile`, `content-store-adapter-profile`,
`wasm-component-performance-profile`, `fabric-port-boundaries`, and `native-system-extension-host-profile`.

## Review and lifecycle

The no-spec plan review approves the proposal, design, acceptance, and tasks with no findings. The proposal, design,
and tasks gates pass, and `cairn validate --strict` reports no issues.
