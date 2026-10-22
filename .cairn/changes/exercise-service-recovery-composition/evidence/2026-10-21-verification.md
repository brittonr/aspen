# Exercise a service recovery composition end to end: verification

## What changed

The deterministic service recovery composition lives in `src/harness/parts/composition/`, mounted as
`crate::harness::composition`:

- `p000/body.rs`: the shared definitions. One durable stateful service, one dependent worker ordered after it in one
  recovery group, and one independent sibling outside the group. The stateful service uses a state-machine facade, the
  worker and the sibling use service facades. The property vocabulary names the five properties the composition
  asserts.
- `p001/body.rs`: the scenario vocabulary, the variant vocabulary, the facts one scenario records, and the shared
  fixtures.
- `p002/body.rs`: the scenario matrix. Every scenario drives the landed mechanisms: the recovery-group plan orders the
  restarts, the lifecycle transition advances the incarnation, the facade admits or rejects a fired timer, the readmission
  contract admits durable work under a current claim, the durable effect transaction surfaces an uncertain outcome, and
  the resource envelope denies an overload.
- `p003/body.rs`: the property evaluation.
- `src/harness/parts/mod/tests/m000/p011/body.rs`: the composition fixtures.

The composition is the test: each property is a named assertion, and two deliberately broken variants prove the
assertions are not vacuous. The fencing-removed variant restarts without advancing the incarnation, so the delayed
pre-restart delivery is admitted; the group-budget-reset variant clears the restart history on every replacement, so
the storm never reaches a terminal outcome. The harness fails the matching property for each.

## Validation

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test -p molten --lib -- testbed::tests`: 124 passed, 0 failed, including the five composition fixtures:
  the definitions bind three services and one recovery group, the correct composition holds every property in every
  scenario, the broken fencing variant violates the stale-instance property, the broken budget variant violates the
  storm property, and the recovery group orders the dependent worker after the stateful service while the restarted
  members advance their incarnation.
- `cargo test --workspace`: exit 0. The molten library suite reported 1,657 passed, the core suite 492 passed, and the
  native system-extension integration suite 9 passed.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings, 0 errors, with
  `cargo-octet 0.1.0` on `nightly-2026-03-21`. The first runs reported findings in the new module: 163 on the first pass
  (non-trait imports, module file count, unwraps in production code, unbounded collection growth, explicit defaults, and
  path-segment repetition) and 32 on the second. The module now uses module glob imports and qualified owner paths, the
  production code returns results instead of unwrapping, the parts are split, the item names avoid the module word, and
  the re-run is clean.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings, `spec_findings: []` and
  `substance_findings: []`.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls `cargo metadata --all-features`, and that
  call panics in the pinned cargo on the optional `rad://` sources. The panic reproduces on the unmodified tree and is
  unrelated to this change, which touches no manifest. The documented Cargo fallback was used.

## Deferred scope

Two parts of the change are not implemented here, because their owning changes are not landed:

- The CPU-saturation scenario under the time-sliced execution profile needs the admitted time-sliced profile from
  `admit-time-sliced-component-execution`.
- The real-process twin needs the outside-in process-fault track and its observation conventions from
  `exercise-node-processes-under-faults`.

Both are recorded as open tasks in the change. The composition's other scenarios and the broken variants do not depend
on them.

## Non-claims

- A passing composition does not prove whole-runtime correctness, distributed failure detection, or production workload
  performance.
- The composition composes the landed contracts deterministically. It is not the real-process twin, and it does not prove
  that the live adapters are deterministic.
- A failing property is evidence against the mechanism that owns it. This change records the failure and does not patch
  mechanism code.
