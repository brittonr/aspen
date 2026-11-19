## Phase 1: Pin

- [x] [serial] Adopt Choregraph `318d4932` (preserves `5.0.0-rc.10` plus regenerated Kamacite receipts) and bump Molten to `syndicate 0.44.0-rc.9` and `preserves 5.0.0-rc.10`. r[molten.dependency_audit.pin_surface]
  - `cargo metadata` re-resolved the lock with the old choregraph revision gone (0 stale entries), `cargo check --workspace --all-targets`, `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` passed; versions read back as `syndicate 0.44.0-rc.9`, `preserves 5.0.0-rc.10`, `tracing-subscriber 0.3.23`.
- [x] [serial] Repin the whole revision surface: both manifests, flake input/lock, release profile, four focused workspaces and locks, plans, source hashes, notices and docs. r[molten.dependency_audit.pin_surface]
  - `scripts/git-source-hashes.sh --write` wrote 27 revision-qualified hashes and both unit2nix plans regenerated; `world-distribution-octet` and `world-promotion-octet` resolve, and `world-merge-octet` still needs its check-materialized `vendor/` tree as before.

## Phase 2: Verification

- [x] [serial] Re-run the pinned audit and record the before/after. r[molten.dependency_audit.pin_surface]
  - Same RustSec revision `4ef2df8e`, as-of 2026-11-19: 158 findings and 4 vulnerabilities before, 156 findings and 3 after; every prerequisite reports 0 and `source-policy` is back to 5.
- [ ] [serial] Run strict Cairn validation and the proposal, design and tasks gates, then sync and archive with this evidence. r[molten.dependency_audit.pin_surface]
