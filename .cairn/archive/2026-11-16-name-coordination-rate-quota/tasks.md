## Phase 1: Implementation

- [x] [serial] Document the per-key lifetime quota in code and name it in the denial diagnostic. r[molten.coordination.rate_limit_semaphore]
- [x] [serial] Prove exhaustion persists across later committed operations and that keys keep separate quotas. r[molten.coordination.rate_limit_semaphore]
- [x] [serial] Update the protocol reference and specification; run focused coordination tests, formatting, Clippy and source/lifecycle gates before review, sync and archive. r[molten.coordination.rate_limit_semaphore]

## Verification (2026-11-16)

Integration tree with the #43, #44, #57 and #58 changes applied: `cargo fmt --check`; `cargo test -p molten --lib coordination` (23 passed, including `rate_limit_is_a_per_key_quota_that_later_operations_do_not_reset`); `cargo clippy -p molten --lib --bins --tests -- -D warnings` passed. These are native test observations, not a live coordination-service qualification.
