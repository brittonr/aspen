## Why

Issue #28's audit is a deny, not a clean result: 203 findings at `b6971ceab`,
including 38 reachable vulnerabilities in the default dependency closure. The
findings were recorded but never triaged, so the repository could not say which
ones a registry refresh fixes and which need an upstream move.

## What Changes

- Refresh every registry crate whose advisory has a semver-compatible fix inside
  the current pins: `zstd`/`zstd-safe`/`zstd-sys`, the cap-std `cap-primitives`,
  the iroh-resolved `hickory-*` family, `h2`, `rustls`, `rustls-webpki`,
  `ctutils`, `crossbeam-epoch` and `webpki-roots`/`webpki-root-certs`.
- Regenerate `Cargo.lock` and both unit2nix plans through the documented pin
  procedure; no git pin, `flake.lock` or source hash changes.
- Record the before/after audit at one pinned RustSec revision and name the four
  advisories that cannot be fixed here because they need an upstream move.

Non-goals: no advisory exception is added, no source/SPDX/notice finding is
resolved, and the audit still denies. This does not make the closure clean.

## Impact

- **Files**: `Cargo.lock`, `build-plan.json`, `release-policy-build-plan.json`,
  this change's evidence.
- **Testing**: `cargo check --workspace --all-targets`, the audit app before and
  after, `cargo fmt --check`, Clippy, Octet, and strict Cairn validation with the
  proposal, design and tasks gates.
