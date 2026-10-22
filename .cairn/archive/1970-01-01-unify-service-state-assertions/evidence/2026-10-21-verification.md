# Unify service state assertions: verification

## Scope

The change projects committed service state into one shared assertion set and makes the service
dependency predicate read that set.

- `src/lifecycle/parts/mod/p008/body.rs` holds the single mapping table. Every local value of the
  three vocabularies has one row: the ten lifecycle state-machine states, the nineteen
  system-extension phases, and the two production readiness outcomes.
- `src/lifecycle/parts/mod/p007/body.rs` holds the shared vocabulary (`started`, `ready`,
  `complete`, `failed`, user-defined values), the `up` alias over `ready` or `complete`, and the
  pure projection that reads the table.
- `src/lifecycle/parts/mod/p004/body.rs` and `p005/body.rs` replace the required/ready dependency
  ref pair with `ServiceDependencyInput`, which carries the required assertion
  (`ready`, `complete`, or `up`) and the projected set of that dependency.
- `src/service/parts/runtime/p003/body.rs` projects a dependency status before it counts as ready,
  so the production dependency gate reads the same mapping.
- `src/lifecycle/parts/mod/tests/m000/p005/body.rs` covers the projection, and
  `tests/m000/p006/body.rs` covers the mapping table in both directions and the dependency
  predicate.

The lifecycle state machine remains the transition authority. The projection never drives a
transition, and a user-defined value satisfies no built-in requirement.

## Validation

- `cargo fmt -p molten -- --check`: clean.
- `cargo clippy -p molten --all-targets -- -D warnings`: passed.
- `cargo test -p molten --lib -- phases::tests::`: 49 passed, 0 failed. This includes the twelve
  new tests for the one-shot `complete` projection, the `started`/`ready` union, the `up`
  dependency, the user-defined negative case, the exit-before-readiness `failed` case, the
  projection purity case, the unknown-phase and empty-user-defined rejections, the mapping table
  in both directions, and the dependency predicate over a projected set.
- `cargo test -p molten --lib`: 1651 passed, 0 failed.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings,
  `spec_findings: []` and `substance_findings: []`.
- `cargo octet check --artifact-dir target/octet-precommit` over the workspace: `Status: clean`,
  0 findings, 0 warnings, 0 errors, with `cargo-octet 0.1.0` on `nightly-2026-03-21`. Earlier runs
  reported findings in the new code: one `excessive_file_length` and five `bool_naming` in the new
  parts, then thirteen findings in the new test files. The parts were split, the bindings renamed,
  the enum matches made exhaustive, the accumulators bounded, and the long test split, and the
  re-run is clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test --workspace`: exit 0.

## Environment limitation

`cargo nextest` cannot run in this checkout. `cargo-nextest` always calls
`cargo metadata --all-features`, and that call panics in the pinned cargo
(`package_id_spec.rs:248`, `Option::unwrap()` on `None`) because the optional `rad://` sources
(`executable-extent-*`, `vm-cohort-core`) are not resolvable outside the flake's source
replacement. The panic is reproducible on the unmodified tree with
`cargo metadata --format-version=1 --all-features --filter-platform x86_64-unknown-linux-gnu` and
is unrelated to this change, which touches no manifest. The documented Cargo fallback was used
instead.

## Non-claims

The projection is a pure function over committed state. It grants no authority, decides no
placement, and proves nothing about live adapter behavior, release readiness, or the correctness
of a service's own transitions. Passing tests establish the mapping, the requirement matching,
and the documented rejections only.
