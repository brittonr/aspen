## Why

After replay and clippy validation land, release review needs fresh evidence that binds the new candidate tree. The existing README records prior Nix nextest and dogfood output paths, but those refs will be stale once the replay changes are committed. A dedicated change should capture the evidence refresh and readback requirements without mixing them into the replay implementation.

## What Changes

- Regenerate hermetic nextest evidence for the candidate tree.
- Regenerate dogfood local-node release evidence that depends on nextest.
- Verify release bundle, signed-member checks, promotion, promotion summary, export manifest, evidence archive, and export verification for the same candidate.
- Update documentation/readback refs only after the evidence graph passes.

## Impact

- **Files**: release evidence references in README or operator notes, generated target evidence during validation, and possibly dogfood fixtures if current checks expose stale assumptions.
- **Testing**: Nix `nextest`, `dogfood-local-node`, release bundle verify/promote/summary/export/verify commands, and current Rust validation gates.
