# MicroVM portable runner profile and fail-closed admission

- Change: `implement-microvm-runtime-runner`
- Tasks: service-core alignment, portable microVM runner/profile model, fail-closed admission checks, node-local prepare/start/stop/observe surface, secret-safe receipts, positive/negative tests
- Started: `2026-05-07T02:53:07Z`
- Completed: `2026-05-07T02:55:44Z`

## Implementation

- Added portable `MicroVmRuntimeProfile` DTOs in `crates/aspen-runtime-core/src/lib.rs`:
  - engine and virtualization backend reporting;
  - supported guest artifact profiles;
  - runner capability/version;
  - verified guest artifact identities for kernel/initrd/rootfs/disk/guest-image inputs;
  - declared mount/block/network/vsock/metadata/capability/output launch bindings;
  - finite resource policy, lease, heartbeat, log limit, and redacted output artifacts.
- Added `admit_microvm_profile` to fail closed before boot or handle exposure on:
  - non-microVM host kinds;
  - runner/engine mismatch;
  - missing runner launch capability;
  - invalid or mismatched guest artifact identities;
  - unsupported guest profiles;
  - resource policy overflow;
  - undeclared or ambient launch bindings;
  - invalid lease/heartbeat metadata;
  - secret-bearing output summaries.
- Added `MicroVmProfileReceipt`, `microvm_lifecycle_receipt`, and `admit_microvm_receipt` for lifecycle/output/failure evidence with artifact identities, runner identity, lease/attempt/heartbeat state, opaque capability handles, bounded log summary, and redacted diagnostics.

## Verification

- `rustfmt crates/aspen-runtime-core/src/lib.rs`
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core microvm --all-targets`

Result: three focused microVM tests passed.
