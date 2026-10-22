# Admit a time-sliced component execution profile: verification

## What changed

The time-sliced component profile is admitted as its own profile identity, with the
synchronous component profile untouched.

- `docs/wasm-component-runtime/contracts.ncl`: the profile contract admits the
  time-sliced identity only with a valid slicing binding, and keeps the synchronous profile free of one.
- `docs/wasm-component-runtime/profile-template.ncl`: the template gains the optional `slicing` binding.
- `docs/wasm-component-runtime/time-sliced-profile.ncl` and
  `docs/wasm-component-runtime/generated/time-sliced-profile.json`: the new admitted profile, binding a scheduling
  quantum of 1,000 fuel and an independent total execution budget of 100,000 fuel.
- `docs/wasm-component-runtime/generated/profile.json`: regenerated, because the template now emits the absent
  `slicing` binding as `null`. The base profile's identity and bindings are unchanged.
- `flake.nix`: the component profile check now also regenerates and diffs the time-sliced export.
- `src/wasm/component/parts/model/p000/body.rs`: `SlicingProfile` carries the quantum and the total budget;
  `RuntimeProfile` carries the optional binding; `RequestedExecutionProfile` gains the time-sliced request.
- `src/wasm/component/profile.rs`: the time-sliced profile loads and validates; the slicing binding is checked for a
  positive quantum, a positive budget, and a budget that covers at least one quantum.
- `src/wasm/component/parts/slicing/p000/body.rs`: the pure contract and the admission boundary. A time-sliced request
  under a synchronous profile is denied with the profile denial class, and a synchronous request under the time-sliced
  profile is denied the same way. The slice ledger records the quantum boundaries and refuses to
  exceed the total budget; the pending turn publishes nothing before the callback completes and publishes exactly once;
  the yielded continuation is re-enqueued as an ordinary runnable with the admitted scheduler; the replay comparison
  names the first boundary where the recomputed and recorded slicings differ.

## Validation

- `cargo test -p molten --lib wasm_component::tests::slicing`: 7 passed, 0 failed. The fixtures cover the admitted
  profile and its bindings, the missing and malformed binding denials, the mismatched request denials in both
  directions, the slice ledger and its budget bound, the boundary-named replay divergence, the pending turn, and the
  continuation's scheduler command.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test --workspace`: exit 0. The molten library suite reported 1,664 passed, the core suite 492 passed, and the
  native system-extension integration suite 9 passed.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings, 0 errors, with
  `cargo-octet 0.1.0` on `nightly-2026-03-21`. The first runs reported findings in the new module: 27 on the first pass
  (file length, module file count, bool naming, explicit defaults, borrowed argument types, non-trait imports, and a
  fallible integer fallback) and 10 on the second. The slicing module moved under the parts tree, the boundary index
  became platform independent, the profile validation uses the bounded blocker sink, and the re-run is clean.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings, `spec_findings: []` and
  `substance_findings: []`.
- `cargo test --workspace --doc` with a fresh `CARGO_TARGET_DIR`: the doc-test phase passes (7 doc-tests in the core
  suite, none elsewhere). The shared target directory needs a clean state for that phase: the Octet producer runs under
  its own pinned toolchain in the same target directory, and the doc-test build then cannot load the externs it finds
  there. This is a target-directory interaction, not a source defect; the base commit reproduces it.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls `cargo metadata --all-features`, and that
  call panics in the pinned cargo on the optional `rad://` sources. The panic reproduces on the unmodified tree and is
  unrelated to this change, which touches no manifest. The documented Cargo fallback was used.

## Deferred scope

The guest-side yielding is not implemented. The component's `invoke` export is a synchronous WIT export, and the
pinned cohort's fuel-based yielding needs the async entrypoints that an async component ABI would introduce. The
delivered contract is the admitted profile, the admission boundary, the slice ledger, the pending turn, the
continuation identity, and the replay comparison. No component has yet yielded at a quantum boundary, and the change
keeps the sliced-execution tasks open with that dependency recorded.

## Non-claims

- A quantum is a work-accounting bound, not elapsed CPU time or a latency guarantee.
- The time-sliced profile is not the default. The synchronous component profile is unchanged and keeps its identity,
  and no existing consumer moved to the new profile.
- Time slicing adds no memory or authority isolation, and yielded continuations do not prove fairness of the whole
  system; fairness stays scoped to the admitted scheduler policy.
