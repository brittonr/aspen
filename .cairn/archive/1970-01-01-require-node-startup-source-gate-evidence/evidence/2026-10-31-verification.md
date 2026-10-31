# Verification: F02 real startup evidence (2026-10-31)

Integration worktree `/home/brittonr/.cargo-target/aspen-open-cairns-20261029`. Commands and
results are copied from their output. The producer and its signing are recorded in the
prerequisite change `produce-exact-node-startup-binding`
(`evidence/2026-10-31-verification.md`); the authority used is the explicitly experimental
signer, not production operator approval.

## Static finding and its repair

The 2026-10-21 review found `molten node run` supplying
`octet_gate::synthetic_clean_octet_gate_receipt_for_tests` to runtime admission. Normal startup
now loads `source-gate-trust.json`, authenticates the selected producer output, re-measures the
executing ELF, retained artifacts and complete build closure, and admits a versioned binding
before any lifecycle write. `src/node/parts/daemon/p018/body.rs::run_local_with_root` calls
`source_binding::load(root)`; `synthetic_clean_octet_gate_receipt_for_tests` is now reached only
from `#[cfg(test)]` code (`source_binding::fixture`, runtime/daemon test modules) and from the
Octet gate's own tests.

## Real normal-startup reproduction and control

`scripts/smoke-startup-source-binding.py` ran against two real producer outputs (`c267d062c`
and `ea61f0463`). Each run printed, for its own output:

```
real-normal-startup negative=missing denied exit=1 lifecycle=unchanged
real-normal-startup negative=stale denied exit=1 lifecycle=unchanged
real-normal-startup negative=wrong-candidate denied exit=1 lifecycle=unchanged
real-normal-startup negative=tamper denied exit=1 lifecycle=unchanged
real-normal-startup negative=missing-gate-closure denied exit=1 lifecycle=unchanged
real-normal-startup negative=synthetic denied exit=1 lifecycle=unchanged
real-normal-startup negative=read-error denied exit=1 lifecycle=unchanged
real-normal-startup positive=passed authority=explicitly-experimental production-approval=none
```

Two further controls on `c267d062c` (throwaway script, retained log):

```
real-normal-startup negative=unsigned denied exit=1 lifecycle=unchanged reason=error: invalid harness artifact: source output has no signature by the independently selected key
real-normal-startup negative=wrong-key denied exit=1 lifecycle=unchanged reason=error: invalid harness artifact: source output has no signature by the independently selected key
```

The positive is a real-artifact control: `node init`, `node run`, `node stop` with the signed
output's own binary, and the startup receipt must contain the accepted `binding_ref`. Every
negative compares all lifecycle bytes before and after the denied start.

## Repository adapter tests

| Test | Covers |
|---|---|
| `src/node/source-binding/tests.rs::normal_startup_evidence_errors_preserve_active_lock_and_clean_shutdown` | missing, read-error and synthetic trust carriers deny and leave lock, shutdown and startup evidence untouched, with no receipts or validation artifact prepared |
| `molten_core::startup_binding::tests::admission_requires_authenticated_exact_executing_candidate` | unauthenticated output, wrong executing candidate and stale binding |
| `molten_core::startup_binding::tests::cohort_rejects_tamper_missing_and_duplicate_roles` | tampered, missing and duplicate gate artifacts |
| `molten_core::startup_binding::tests::historical_and_fixture_producer_profiles_are_not_promoted` | unsupported receipt schema and fixture producer profile; policy and profile mismatch |
| `src/node/source-binding/tests.rs::verifier_process_executes_the_selected_binary_under_valid_bounds` | the real bounded verifier process boundary |

All ran in the workspace suite below.

## Versioning, replay and operator documentation

`docs/node-state-filesystem-authority.md` "Startup source evidence and operator refresh"
documents independent trust selection, the protected provisioning channel, refresh after any
candidate change, the version-1 binding/validation boundary, legacy synthetic receipts as
historical diagnostics only, and the non-claims (no compiler correctness, no activation
authority). Fixture validation uses a separate `node-source-fixture-validation-v1` schema that
normal wrappers cannot select.

## Executed checks

| Command | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` on `c267d062c` | 2911 passed, 0 failed, 28 ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` (default, `spacewasm-pilot`, `component-rpc-telemetry`) | each exit 0 |
| `cargo octet check` primary and `-p molten -- --lib` | `Status: clean`, `Findings: 0` each |
| `cargo fmt --check` | exit 0 |
| `nix build path:.#checks.x86_64-linux.{node-state-authority,native-system-extension-host-profile,release-dependency-profile,inherited-tracey-debt}` | exit 0; tracey `verdict=pass`, `dangling=0` |
| signed CLI suite `cargo test -p molten --test cliharness -- --ignored` | 10 passed, 3 cluster-harness tests failed on a loaded host with `local-executable-child-timeout` (300 s per-child ceiling) |
| same three tests after the ceiling change, on the signed output | see "Signed rails" below |
| signed outside-in suite `cargo test -p molten --test outside_in -- --ignored` | 9 passed, 0 failed (4102 s) |

## Signed rails

`MOLTEN_TEST_SOURCE_TRUST` selected the `ea61f0463` producer output. The three signed
cluster-harness tests were rerun after `MAX_CLUSTER_CHILD_TIMEOUT_MS` rose from 300 s to 30 min:
a signed child start authenticates the whole producer output and re-measures its 21,043-path
build closure, so the old ceiling only fit an unloaded host. The new bound is still bounded and
matches the outside-in scenario ceiling. Regression test
`cluster_harness::tests::child_timeout_ceiling_admits_signed_startup_and_still_bounds_a_child`
shows the ceiling passes timeout validation and one millisecond past it is denied.

Rerun with the new ceiling and the same signed output:

```
test cli_cluster_harness_admits_exact_bound_fixture_with_actual_child ... ok
test cli_cluster_harness_executes_checked_fixture_and_verifies_offline ... ok
test cli_cluster_harness_offline_verifier_denies_tampered_artifact ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 61 filtered out; finished in 2034.66s
```

So the signed CLI suite is 13 of 13 (10 + these 3) and the signed outside-in suite 9 of 9.

## Traceability

The inherited-tracey-debt guard first failed on `molten.audit_f02.real_evidence` (no source reference); markers were added on the pure admission, the loader module and the lifecycle-preservation test, after which the guard reported `verdict=pass`, `dangling=0` (`evidence/2026-10-31-closeout/tracey.log`).
