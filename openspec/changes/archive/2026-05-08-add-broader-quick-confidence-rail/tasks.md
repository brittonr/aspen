## Phase 1: Rail Definition

- [x] [serial] Create the OpenSpec baseline for a broader quick confidence rail.
- [x] [serial] Inventory existing cheap checks and choose the initial bounded command set.

## Phase 2: Implementation

- [x] [depends:inventory] Implement the rail as the narrowest appropriate script, Nix app/check, or harness profile.
- [x] [depends:rail] Add structured summary output that lists included checks, pass/fail status, and skipped gated proofs.
- [x] [depends:summary] Add docs or operator help text describing what the rail proves and does not prove.

## Phase 3: Verification

- [x] [depends:docs] Run the quick confidence rail, targeted tests for its summary behavior, OpenSpec validation, and whitespace checks.
