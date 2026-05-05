## Implementation

- [x] I1 Add artifact metadata to CI run receipt job entries. [covers=dogfood-evidence.ci.run-receipt.artifacts]
- [x] I2 Populate receipt artifact metadata from bounded per-job KV scans. [covers=dogfood-evidence.ci.run-receipt.artifacts]
- [x] I3 Update CLI human output to summarize receipt artifact count. [covers=dogfood-evidence.ci.run-receipt.artifacts]

## Validation

- [x] V1 Run `openspec validate add-ci-receipt-artifacts --strict --json`.
- [x] V2 Run focused Rust tests for CI receipt rendering: `cargo test -p aspen-client-api` and `cargo test -p aspen-ci-handler --features forge,blob`.
- [x] V3 Run `git diff --check`.

## Archive

- [x] A1 Archive the OpenSpec change after implementation and validation.
