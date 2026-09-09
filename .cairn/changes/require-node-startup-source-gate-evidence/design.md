## Context and evidence

F02 applies to source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

`src/cli/ops/node/lifecycle.rs:136-143` routes normal startup into the daemon. `src/node/parts/daemon/p018/body.rs:18-30` supplies the synthetic receipt. `src/octet/parts/gate/p002/body.rs:182-212` creates its passing fields and test toolchain label.

Trigger: initialize valid node state, then use normal `molten node run` without independently validated source-gate artifacts. The daemon supplies zero findings and `nightly-test-toolchain` through the fixture helper. Expected behavior denies absent real evidence. The current control flow instead gives runtime admission an invented passing input.

The evidence is static only. The audit did not run this startup sequence. Its 359 passing core baseline tests and eight failing unrelated regression assertions do not validate this path.

## Boundary and data flow

The startup shell loads exact gate artifacts through existing capability-rooted storage or explicit operator inputs. The pure gate evaluator receives artifact values, expected candidate identity, and policy bindings. It returns acceptance or diagnostics without reading files, environment variables, or clocks.

Normal startup accepts no synthetic replacement. Content hashes alone prove identity, not that Octet executed successfully. Admission requires the existing validator to establish complete gate artifact linkage and candidate freshness.

The implementation review must identify the authoritative source-to-binary candidate binding and existing gate artifact producer. The CLI and public compatibility wrappers must converge on that binding. Package version alone is not candidate identity. Missing evidence fails closed rather than selecting an implicit fixture.

The shell validates evidence before activation, passing adapter-start receipts, or destructive restart preparation. Denial preserves an existing active lock and prior clean-shutdown evidence. This ordering matters because `verify_restart_state` currently removes the shutdown file before source-gate admission.

## Test-only composition

Explicit test composition can supply fixture values to exercise domain decisions. The fixture path remains unavailable to normal startup, including compatibility wrappers. A toolchain label blacklist alone is insufficient because synthetic values can use another label.

Tests must distinguish a fixture-only admission test from a normal-path artifact validation test. Neither fixture metadata nor a digest establishes real gate execution.

## Compatibility and receipts

Keep checked profile exports and runtime-free Nickel semantics unchanged. Startup receipts bind accepted source-gate refs and their exact candidate context. Legacy synthetic receipts remain readable as historical diagnostic artifacts, but cannot satisfy fresh normal startup admission.

Review the evidence input API and any receipt schema extension before implementation. A missing artifact can now reject startup that previously accepted synthetic evidence. Document this intentional fail-closed compatibility change and the operator refresh procedure. Replay retains historical provenance and does not upgrade fixture results into real checks.

## Validation and ownership

Baseline existing node lifecycle and runtime source-gate tests before edits. Add normal repository regressions for valid real artifacts and missing, stale, malformed, tampered, wrong-candidate, and synthetic inputs. Adapter tests cover missing files, read errors, and unchanged lifecycle state after rejection.

Run focused Octet and Clippy error gates, workspace tests, relevant Nix source-gate and node-state checks, and all required Cairn gates. No policy weakening or fabricated evidence is allowed.

Node-runtime maintainers own startup composition. Octet integration maintainers own gate validation. Existing gate and node-host components remain the reuse candidates. No new dependency is required by this plan.

F01 owns shutdown admission. F14 owns current-run observations and can share startup identity facts after this evidence contract is clear. F03 owns ingress publication. Review shared fixture changes first without circular package dependencies.

## Nonclaims

Passing Octet evidence is not production readiness, runtime authority, complete binary correctness, or proof of live adapter behavior. This document records a plan, not execution or implementation approval.
