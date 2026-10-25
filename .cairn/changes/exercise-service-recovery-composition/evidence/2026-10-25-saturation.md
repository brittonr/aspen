# CPU saturation in the service-recovery composition

## Delivered scope

The deterministic matrix now includes all eight scenarios. CPU saturation uses the admitted time-sliced component
profile and the fabric-time FIFO scheduler. A stateful-service callback yields between fuel quanta, letting the
independent sibling complete before that callback ends. The deliberately broken `SlicingDisabled` variant drains the
callback without scheduling the sibling between yields; the unaffected-services property fails while the callback
still completes within budget.

Implementation: `src/harness/composition/parts/p004/body.rs`, dispatched by `p002/body.rs` and evaluated by
`p003/body.rs`. Regression coverage lives in `src/harness/parts/mod/tests/m000/p011/body.rs` and
`src/wasm/component/tests/slicing/yielding.rs`. The latter drives a real CPU-heavy Wasmtime callback through the same
scenario, checks bounded completion in both variants, and checks sibling progress and its named property separately.
No runtime mechanism was patched to satisfy the composition.

Immediate outcome: the formerly blocked CPU-saturation row is executable. Durable capability: one reusable scenario
accepts modeled fuel-ledger work and real sliced component work, with a negative control for lost scheduling turns.
Maintenance owner: Molten's service-recovery harness and component-runtime maintainers. Repeatability uses the admitted
deterministic profile, fixed workload, bounded scheduler turns, and the checked-in positive/negative fixtures.

## Exercised validation

All Cargo commands ran through `nix develop`, with `TMPDIR=/home/brittonr/.cache/aspen-tmp-20261023` and
`CARGO_TARGET_DIR=/home/brittonr/.cache/aspen-verify-20261023`.

- A throwaway executable called the public `molten::testbed::composition` API for both variants, checked their named
  property and bounded completion, and printed:

  ```text
  variant=correct sibling_before_callback=true bounded=true property=unaffected-services-continue holds=true
  variant=slicing-disabled sibling_before_callback=false bounded=true property=unaffected-services-continue holds=false
  ```

  The executable source was removed after the smoke run.
- `cargo test -p molten --lib -- testbed::tests wasm_component::tests::slicing`: **137 passed**, 0 failed. This includes
  the complete deterministic matrix, the broken fencing/budget/slicing variants, the real-Wasmtime saturation scenario,
  fuel exhaustion, continuation replay, and admission-denial coverage.
- `/home/brittonr/.cache/aspen-tmp-20261023/tracey-guard --root . --baseline
  evidence/tracey/inherited-debt-baseline.txt`: **pass**, 2,955 requirements, 1,021 referenced, 1,934 inherited uncovered,
  0 dangling references. No debt baseline was changed.

The local compiler-cache index reported a disk-I/O error and disabled caching. Compilation and the smoke and focused
suite still completed successfully; cache availability is not behavioral evidence.

## Remaining lifecycle boundary

This addendum supersedes the CPU-saturation deferral in `2026-10-21-verification.md`.
`admit-time-sliced-component-execution` is archived and supplies the admitted profile.
`exercise-node-processes-under-faults` has also landed; it is no longer the live twin's missing prerequisite.
The remaining blocker, recorded in `tasks.md`, is that real `molten node` processes do not host the system-extension
recovery groups, supervision, durable-work readmission, or generation upgrades needed by the twin. The live-twin task
and both-tracks validation task stay open. This change must not be archived on the deterministic track's results alone.

These observations are deterministic-harness and in-process Wasmtime evidence only. They do not establish live-process
agreement, whole-runtime correctness, distributed failure detection, production readiness, or workload performance.
A composition failure belongs to its mechanism owner; this harness does not patch mechanisms to make properties pass.

## Policy and lifecycle gates

- `cargo octet check --artifact-dir target/octet-precommit`: **clean**, 0 findings, 0 warnings, 0 errors, using
  `cargo-octet 0.1.0` and `nightly-2026-03-21-x86_64-unknown-linux-gnu`. The first run found one compound assertion in
  the modeled-saturation fixture. The assertions were split, bounded completion was asserted for the broken variant,
  and the rerun was clean; no lint was disabled.
- `cairn gate proposal`, `cairn gate design`, and `cairn gate tasks` for
  `exercise-service-recovery-composition`: **PASS**, no issues. These structural gates do not discharge the live twin.
- `cairn sync exercise-service-recovery-composition --root . --execute`: applied the existing composition requirement
  to `.cairn/specs/testing-harness/spec.md`; the change remains active with two open tasks.
- `nix run path:../cairn#cairn -- validate --root . --policy ../cairn/cairn-policy/generated/cairn-policy.json --strict`:
  **valid**, `spec_findings: []`, `substance_findings: []`.
- Tracey guard after spec sync: **pass**, 2,956 requirements, 1,022 referenced, 1,934 inherited uncovered, 0 dangling.
  The accepted-spec addition changed the reported totals; the inherited-debt baseline remained unchanged.
- `cargo test --workspace --lib -- --quiet`: **1,739 Molten library tests and 619 core tests passed**, 0 failed;
  the node-host and release-policy library targets contained no tests.
- After the assertion refinement, the exact `testbed::tests::disabled_slicing_starves_the_sibling_and_violates_the_unaffected_services_property`
  fixture was rerun: **1 passed**, 0 failed, including bounded completion in both variants.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo fmt --all -- --check`: exit 0.

The documented Cargo fallback was used instead of retrying the previously recorded nextest/all-features metadata
failure on optional `rad://` sources. The broad execution above is the workspace library suite, not all integration
targets. No dependency, Nix profile, VM adapter, or manifest changed; no hermetic Nix build or live-process/VM result is
claimed for this slice.
