# Fence runtime-local work across restarts: verification

## Conformance verdict

The permanent restart-fencing conformance trace lives in
`crates/molten-core/src/system_extension/incarnationtests.rs`. It installs and starts instance A, creates
runtime-local work addressed to A (a timer, a pending callback completion, and a failure notification),
restarts the instance as B without changing the implementation generation, and then delivers every
delayed item.

Before the fix, the trace reported that the replacement instance accepted all three deliveries:

```text
the replacement instance accepted stale runtime-local deliveries:
["delayed timer", "callback completion", "failure notification"]
```

The lifecycle-sequence propagation of the addressable-actor profile does not reach the system-extension
callback boundary, so the design's second stage applied: the lifecycle gained a runtime incarnation.

## What changed

- `crates/molten-core/src/system_extension/lifecycle.rs`: `LifecycleState` carries `incarnation`, which
  advances on every restart-entry transition (`BeginRestart` and `BeginRecovery`). `LifecycleEvent`
  binds the incarnation that produced it, and a mismatched incarnation is a typed
  `LifecycleIssue::StaleIncarnation`.
- `crates/molten-core/src/system_extension/dispatch.rs`: `CallbackEvent`, `CallbackInvocation`, and
  `TypedEffectRequest` bind the incarnation. Dispatch denies a stale timer or message with
  `DispatchIssue::StaleIncarnation`, effect validation denies a stale effect with
  `EffectIssue::StaleIncarnation`, and a completion whose invocation predates the executing instance is
  denied with `CallbackOutcomeIssue::StaleIncarnation`.
- `crates/molten-core/src/system_extension/readmission.rs`: the durable readmission contract. Durable
  work is admitted only under a fresh delivery claim bound to the current generation and incarnation;
  runtime-local work presented as durable work is denied, and durable work presented without a current
  claim is denied.
- The shell binds the executing incarnation into every event, callback, and effect it builds, validates
  outcomes and effects against the current instance, and records the previous and next incarnation in
  the lifecycle receipt (`system-extension-lifecycle-v2`, `molten.system-extension.lifecycle.v2`).
- The native callback protocol carries the incarnation: the callback envelope, the effect record, the
  outcome, and the persisted instance state move to version 3, and the native ALPN moves to
  `molten/system-extension/native/v3`. The native host binds the materialized effect to the invocation
  it answers, and the node registry rejects a recovered instance with a stale incarnation.

## Validation

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test -p molten-core --lib system_extension::incarnationtests`: 6 passed, 0 failed. The trace
  asserts the typed stale-incarnation issue for the delayed timer, the callback completion, and the
  failure notification, and names any delivery the replacement instance would accept.
- `cargo test -p molten --lib system_extension`: 14 passed, 0 failed, including the shell regression
  `restart_fences_runtime_local_work_from_the_previous_incarnation`.
- `cargo test --workspace`: exit 0. The molten library suite reported 1,652 passed, the core suite
  481 passed, and the native system-extension integration suite 9 passed, including
  `separate_process_service_runs_lifecycle_effect_restart_and_removal`, which drives the real
  separate native process through the version 3 callback envelope, effect, and instance-state
  records.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings,
  0 errors, with `cargo-octet 0.1.0` on `nightly-2026-03-21`. The first run reported two
  `excessive_file_length` findings in the deterministic fixture; the restart-fencing probe moved to
  its own part, and the re-run is clean.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings,
  `spec_findings: []` and `substance_findings: []`.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls
  `cargo metadata --all-features`, and that call panics in the pinned cargo on the optional `rad://`
  sources. The panic reproduces on the unmodified tree and is unrelated to this change, which touches
  no manifest. The documented Cargo fallback was used.

## Non-claims

- The incarnation is a runtime-local fence. It grants no authority, decides no placement, and does not
  prove that every host adapter propagates the token.
- Durable-work survival stays scoped to the delivery and durability contracts. The readmission contract
  admits a durable item under a fresh claim; it does not prove that a durable log survived, was
  complete, or was replayed.
- The native protocol version bump is a compatibility break: a native extension built against version 2
  is not admitted by a version 3 host, and a native host instance state written by version 2 is not
  resumed by version 3.
- The conformance trace covers the pure dispatch, outcome, and lifecycle boundaries. It does not prove
  whole-runtime correctness or live adapter behavior.
