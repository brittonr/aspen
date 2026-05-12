## Why

Aspen has fresh evidence for core clustering, restored stock cluster VM rails, `nextest -P quick`, and the bounded quick-confidence rail. The remaining confidence gaps are explicitly gated or expensive: runtime-host execution, Hermit/uHyve, Hyperlight, broader microVM/VM networking, full dogfood/self-hosting, and full `nix flake check`.

We need an OpenSpec change so the proof campaign has durable scope, proof boundaries, failure classification, redaction rules, and ordered tasks instead of ad hoc chat-only commands.

## What Changes

- **Staged proof campaign**: Define a lowest-risk-to-highest-cost execution order for cheap product-path checks, VM/microVM checks, runtime-host E2E, Hermit/uHyve, Hyperlight, dogfood, and full flake.
- **Evidence contract**: Preserve command logs and structured summaries under bounded local evidence paths while keeping raw tickets, credentials, and connection secrets out of committed artifacts.
- **Boundary classification**: Every result must be classified as static readiness, product-path marker, VM boot, runtime-host receipt, dogfood acceptance, host blocker, build-input drift, or product failure.
- **Follow-up routing**: Real product failures or multi-component repairs get their own OpenSpec before implementation; narrow build/input drift may be repaired directly with focused verification.

## Capabilities

### Modified Capabilities
- `test-harness-runtime`: Adds a staged gated-runtime proof sweep contract and failure-classification expectations for skipped quick-confidence boundaries.

## Impact

- **Files**: OpenSpec artifacts under `openspec/changes/prove-gated-runtime-confidence/`; possible later evidence summaries under ignored `target/runtime-proof/` and committed spec/task updates only after proof execution.
- **APIs**: None expected for the spec-only campaign. Implementation follow-ups may create separate changes if product behavior must change.
- **Dependencies**: None expected for the OpenSpec itself.
- **Testing**: Validate with `openspec validate prove-gated-runtime-confidence --strict --json`, `openspec validate --all --strict --json`, and `git diff --check`. Execution tasks define their own proof commands and classification gates.

## Out of Scope

- Claiming production readiness from quick/static checks alone.
- Treating cached Nix success as fresh VM execution when proof markers are absent.
- Committing raw VM logs that contain cluster tickets or secret-bearing connection strings.
- Fixing any discovered product failure inside this change unless the repair is trivial, local, and does not broaden scope.
