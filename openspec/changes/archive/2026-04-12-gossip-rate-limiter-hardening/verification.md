# Verification Evidence

## Implementation Evidence

- Changed file: `crates/aspen-cluster/src/gossip/rate_limiter.rs`
- Changed file: `crates/aspen-cluster/src/verified/mod.rs`
- Changed file: `crates/aspen-cluster/src/verified/rate_limiter.rs`
- Changed file: `openspec/specs/gossip-rate-limiter/spec.md`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/.openspec.yaml`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/proposal.md`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/design.md`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/specs/gossip-rate-limiter/spec.md`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/tasks.md`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/verification.md`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/cargo-check-aspen-cluster.txt`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/cargo-test-rate-limiter.txt`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/implementation.diff`
- Changed file: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/openspec-preflight.txt`

## Task Coverage

- [x] 1.1 Add a delta spec for `gossip-rate-limiter` covering atomic shared-budget accounting, deterministic timestamp-driven transitions, and encapsulated limiter state.
  - Evidence: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/specs/gossip-rate-limiter/spec.md`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/proposal.md`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/design.md`, `openspec/specs/gossip-rate-limiter/spec.md`
- [x] 1.2 Extend `crates/aspen-cluster/src/verified/rate_limiter.rs` with the pure transition helpers needed for gossip limiter admission decisions and monotonic time handling.
  - Evidence: `crates/aspen-cluster/src/verified/rate_limiter.rs`, `crates/aspen-cluster/src/verified/mod.rs`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/implementation.diff`
- [x] 2.1 Refactor `crates/aspen-cluster/src/gossip/rate_limiter.rs` to use the verified helper path and avoid consuming global budget on per-peer rejection.
  - Evidence: `crates/aspen-cluster/src/gossip/rate_limiter.rs`, `crates/aspen-cluster/src/verified/rate_limiter.rs`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/implementation.diff`
- [x] 2.2 Reduce visibility of invariant-bearing peer-entry state so callers must go through `GossipRateLimiter` APIs.
  - Evidence: `crates/aspen-cluster/src/gossip/rate_limiter.rs`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/implementation.diff`
- [x] 3.1 Add deterministic tests proving a noisy peer cannot exhaust global budget with traffic that is already rejected by the per-peer limiter.
  - Evidence: `crates/aspen-cluster/src/gossip/rate_limiter.rs`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/cargo-test-rate-limiter.txt`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/implementation.diff`
- [x] 3.2 Add deterministic parity tests for backward timestamps and LRU monotonicity across the verified core and runtime wrapper.
  - Evidence: `crates/aspen-cluster/src/gossip/rate_limiter.rs`, `crates/aspen-cluster/src/verified/rate_limiter.rs`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/cargo-test-rate-limiter.txt`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/implementation.diff`
- [x] 3.3 Run targeted `aspen-cluster` checks/tests that cover the refactor and save implementation evidence when the change is worked.
  - Evidence: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/cargo-check-aspen-cluster.txt`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/cargo-test-rate-limiter.txt`, `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/openspec-preflight.txt`

## Review Scope Snapshot

### `git diff HEAD -- crates/aspen-cluster/src/verified/mod.rs crates/aspen-cluster/src/verified/rate_limiter.rs crates/aspen-cluster/src/gossip/rate_limiter.rs`

- Status: captured
- Artifact: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/implementation.diff`

## Verification Commands

### `cargo check -p aspen-cluster`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/cargo-check-aspen-cluster.txt`

### `cargo test -p aspen-cluster rate_limiter --lib`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/cargo-test-rate-limiter.txt`

### `scripts/openspec-preflight.sh openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-12-gossip-rate-limiter-hardening/evidence/openspec-preflight.txt`

## Notes

- Delta spec synced to `openspec/specs/gossip-rate-limiter/spec.md` before archiving.
