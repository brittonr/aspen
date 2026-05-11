## Why

Recent full-flake recovery showed that Aspen's focused VM checks can pass while the default full `nix flake check -L` rail can over-schedule heavyweight NixOS VM tests on a single host. The result is ambiguous operator evidence: a parallel VM timeout can look like a product regression even when the focused VM check and serialized full rail both pass.

## What Changes

- Make the repo's default flake check scheduling conservative for heavyweight VM checks by serializing local Nix jobs unless an operator explicitly overrides it.
- Document the evidence policy: treat default-parallel VM contention as a scheduling/resource issue until reproduced by the focused check.
- Preserve targeted and explicit override paths for developers who know their host has sufficient VM capacity.

## Capabilities

### Modified Capabilities
- `test-suite-metadata`: Full-flake VM check execution has a deterministic local scheduling default.
- `dogfood-evidence`: Acceptance evidence distinguishes scheduling contention from focused product/test failures.

## Impact

- **Files**: `flake.nix`, OpenSpec specs/tasks for this change.
- **APIs**: No Rust API changes.
- **Dependencies**: No dependency changes.
- **Testing**: OpenSpec validation, `git diff --check`, Nix eval/metadata proof of flake config, and focused VM check evidence where practical.
