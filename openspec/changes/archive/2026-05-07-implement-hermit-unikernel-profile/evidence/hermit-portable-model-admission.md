# Hermit portable model and fail-closed admission

- Change: `implement-hermit-unikernel-profile`
- Tasks: service-core alignment, portable Hermit profile model, fail-closed admission checks, launch-compatibility surface, secret-safe receipts, positive/negative tests
- Started: `2026-05-07T02:18:25Z`
- Completed: `2026-05-07T02:20:46Z`

## Implementation

- Added `RuntimeArchitecture`, `HermitGuestAbi`, `HermitLaunchProfileKind`, `HermitLoaderArtifact`, `HermitLaunchProfile`, `HermitInputChannel`, and `HermitUnikernelArtifact` in `crates/aspen-runtime-core/src/lib.rs`.
- Kept Hermit as `RuntimeArtifact::Unikernel { HermitOs }` under `RuntimeHostKind::MicroVm`, with Uhyve and loader/QEMU compatibility resolved through `MicroVmEngine`.
- Added `admit_hermit_profile` to fail closed before launch on:
  - non-Hermit artifacts;
  - non-microVM host kinds;
  - runner/profile mismatch;
  - missing runner capability;
  - unverified image or loader identities;
  - undeclared input capability handles;
  - secret-bearing or non-secret-safe boot/input channels.
- Added `HermitProfileReceipt`, `hermit_lifecycle_receipt`, and `admit_hermit_receipt` for lifecycle/output/failure evidence with opaque capability handles and redacted serial diagnostics.

## Verification

- `rustfmt crates/aspen-runtime-core/src/lib.rs`
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core hermit --all-targets`

Result: four focused Hermit tests passed.
