# Hyperlight portable runner profile and fail-closed admission

- Change: `implement-hyperlight-runtime-runner`
- Tasks: service-core alignment, portable Hyperlight runner/profile model, fail-closed admission checks, node-local prepare/start/stop/observe surface, secret-safe receipts, positive/negative tests
- Started: `2026-05-07T02:41:31Z`
- Completed: `2026-05-07T02:43:36Z`

## Implementation

- Added portable `HyperlightRuntimeProfile` DTOs in `crates/aspen-runtime-core/src/lib.rs`:
  - ABI profiles;
  - artifact profiles;
  - runner capability report;
  - declared host-call bindings;
  - resource policy;
  - optional redacted output artifact reference.
- Added `admit_hyperlight_profile` to fail closed before exposing host calls or capability handles on:
  - non-Hyperlight host kinds;
  - non-Hyperlight artifacts;
  - missing/invalid image hash or entrypoint mismatch;
  - missing runner capability;
  - unsupported ABI or artifact profile;
  - resource policy overflow;
  - undeclared or ambient host-call bindings;
  - secret-bearing output summaries.
- Added `HyperlightProfileReceipt`, `hyperlight_lifecycle_receipt`, and `admit_hyperlight_receipt` for lifecycle/output/failure evidence with opaque capability handles and redacted diagnostics.

## Verification

- `rustfmt crates/aspen-runtime-core/src/lib.rs`
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core hyperlight --all-targets`

Result: three focused Hyperlight tests passed.
