# Verification: Octet burn-down, collection growth in validators

Base: `octet-burndown-safety-unwrap` (`8cb948d92`). Octet: pinned `octet-toolchain` `fc38f593`. Private
`CARGO_TARGET_DIR`; commands run under `nice -n 10`, `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`.

## Octet

| Scope | Base findings | After | `unbounded_collection_growth` findings (sites) |
|---|---:|---:|---:|
| workspace (`cargo octet check`) | 3481 | 3372 | 214 (111) → 105 (56) |
| `-p molten --lib` | 1490 | 1437 | 102 (102) → 49 (49) |

None of the 24 touched files appears in the remaining `unbounded_collection_growth` list. The remaining 56 sites belong
to `octet-burndown-safety-collections-runtime`. `octet-lint-diff.txt` shows that no other family changed.

## Review of the collected repair

The first compile of the collected diff failed in `src/harness/parts/schema/p026/body.rs`. In that part-file scope,
`Vec::iter()` on `Observation::events` and on the authority ref vectors resolves to the in-scope Preserves value
iterator instead of the slice iterator. The chain now iterates `&observation.events` and the ref vectors' `as_slice()`,
which visit the same elements in the same order as the original `for` loops. `src/harness/parts/gate/p002/body.rs`
uses `as_slice()` in the same way. Literal per-element multipliers became named constants
(`DIAGNOSTICS_PER_BUNDLE_MEMBER`, `DIAGNOSTICS_PER_REQUESTED_ROLE`, `DIAGNOSTICS_PER_CONFIG_FIELD`,
`MISSING_COVERAGE_DIAGNOSTICS_PER_REQUIREMENT`). No new denial bound is introduced, so no bound test is required. The
inputs remain bounded upstream: `ensure_bound` against `MAX_ITEMS` in `testing/hardening.rs`, `MAX_LABEL_COUNT` in
resources, and the parsed record limits elsewhere.

Two single-shot diagnostics were checked against their callers. `evaluate_admission_chain` breaks at the first denial.
`materialization::verify_archive` rejects a duplicate normalized member (`materialization.rs` `seen.insert`), so the
release archive manifest is read at most once.

## Rust gates

- `cargo fmt --check`: exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace --no-fail-fast`: exit 0, 2071 passed, 0 failed, 0 ignored. An earlier full run hit the known
  `fabric_execution::tests::live::live_adapter_preserves_rejected_exit_and_descendant_teardown` EPIPE flake (`WriteStdin
  failed: Broken pipe`) in a module this change does not touch. The rerun above passed with no retries.
- The diff adds no `allow` attribute. `dylint.toml`, baselines, and quarantine files are unchanged.

## Receipt identity

`fixture-receipt-comparison.txt`: the base binary (`8cb948d92`) and this tree ran every `examples/*.preserves` harness suite
(`molten test run` then `molten test gate check`). Exit codes and BLAKE3 hashes of every report and gate receipt are
identical: 6 of 6 suites. This covers the harness gate and report aggregate refs that were rewritten.

## Flake checks

`nix build .#checks.x86_64-linux.<check>` passed: `coordination-delivery-octet-deny-all`, `coordination-delivery-profile`,
`cap-std-store-authority`, and `fabric-port-boundaries`.

## Review and lifecycle

The no-spec plan review approves the proposal, design, acceptance, and tasks with no findings. The proposal, design,
and tasks gates pass, and `cairn validate --strict` reports no issues.
