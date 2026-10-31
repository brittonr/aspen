## Why

F02 identifies invented source-gate acceptance in normal startup at revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

`molten node run` calls `run_local_with_root`. That function supplies `synthetic_clean_octet_gate_receipt_for_tests` to runtime admission. The helper returns `decision="pass"`, zero findings, and `nightly-test-toolchain` without a test-only restriction.

Expected startup requires independently validated evidence for the exact candidate. Actual startup supplies a synthetic passing receipt. This is static evidence from a normal call chain, not an executed startup reproduction. The audit reports eight executed bugs and six static findings. F02 belongs to the static group.

## What Changes

- Load real source-gate artifacts through the normal startup shell.
- Validate exact candidate, policy, profile, toolchain, and required artifact bindings before activation.
- Reject missing, stale, malformed, mismatched, or synthetic evidence in normal startup.
- Preserve explicit test-only fixture composition without a production fallback.

## Impact

- Current consumer: `molten node run` and daemon startup through `run_local_with_root`.
- Maintenance owner: Molten node-runtime and Octet gate integration maintainers.
- Source scope: `src/node/parts/daemon/p018/body.rs`, `src/octet/parts/gate/p002/body.rs`, and `src/cli/ops/node/lifecycle.rs`.
- Spec delta: package-local `node-runtime` requirements.
- Durable capability: exact startup evidence admission and repeatable acceptance/rejection tests.
- Repeatability evidence: planned normal startup adapter tests and source-gate validation fixtures.

## Non-goals and authority

This plan grants no implementation permission. Source-gate evidence does not prove whole-node correctness, live adapter readiness, deployment success, or authority. Existing production profile and release gates remain required. This package does not change Octet policy or introduce a warning allowance.
