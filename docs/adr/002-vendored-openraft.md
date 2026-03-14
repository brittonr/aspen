# 2. Vendored openraft v0.10

**Status:** accepted

## Context

Aspen uses Raft consensus for cluster-wide linearizable operations. The openraft library (v0.10) provides a well-tested, async-native Raft implementation in Rust. Aspen's storage layer (redb), networking layer (Iroh QUIC), and state machine have tight integration requirements with the Raft engine.

Upstream openraft releases on crates.io may not align with Aspen's needs — patches for performance, debugging, or API adjustments would require waiting for upstream releases or maintaining a fork.

## Decision

Vendor openraft v0.10 at `openraft/openraft` in the repository. Aspen depends on the local path, not crates.io.

Update procedure:

1. Review upstream releases for relevant changes
2. Cherry-pick or rebase local modifications onto the new version
3. Run the full test suite (especially madsim deterministic tests)
4. Document modifications in commit messages

Alternatives considered:

- (+) crates.io dependency: no maintenance burden, automatic updates
- (-) crates.io dependency: blocked on upstream release cadence, can't patch locally
- (+) Git dependency with branch: can point at specific commits
- (-) Git dependency: flaky in CI, nix evaluation fetches on every lock update
- (+) Vendored: full control, local patches, reproducible builds
- (-) Vendored: manual update process, risk of falling behind upstream

## Consequences

- Local patches (storage optimizations, debugging hooks) can land immediately
- Nix builds are fully reproducible with no network fetches for openraft
- Updating requires manual cherry-pick/rebase work
- The `openraft/openraft` directory must be clearly marked as vendored to avoid confusion
- Madsim tests exercise the vendored version, catching regressions from local patches
