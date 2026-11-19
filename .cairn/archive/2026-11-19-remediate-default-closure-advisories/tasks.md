## Phase 1: Refresh

- [x] [serial] Refresh the 13 registry crates with a semver-compatible advisory fix and regenerate `Cargo.lock` plus both unit2nix plans. r[molten.dependency_audit.registry_refresh]
  - `zstd` 0.13.4, `zstd-safe` 7.4.1, `zstd-sys` 2.1.3, `cap-primitives` 3.4.6, `hickory-resolver`/`hickory-net`/`hickory-proto` 0.26.2, `h2` 0.4.16, `rustls` 0.23.46, `rustls-webpki` 0.103.16, `ctutils` 0.4.4, `crossbeam-epoch` 0.9.20, `webpki-roots`/`webpki-root-certs` 1.0.10. `unit2nix` regenerated both plans; `cargo check --workspace --all-targets` passed.
- [x] [serial] Record the before/after audit evidence at one pinned RustSec revision and the four advisories that need an upstream move. r[molten.dependency_audit.registry_refresh]
  - The two audits at RustSec `4ef2df8e` (as-of 2026-11-19) recorded 203 findings before and 158 after, advisories 67 -> 23 and vulnerabilities 38 -> 4; see the evidence record and the two advisory tables.

## Phase 2: Verification

- [x] [serial] Run `cargo fmt --check`, Clippy, the Octet source gate, strict Cairn validation and the proposal, design and tasks gates, then sync and archive with this evidence. r[molten.dependency_audit.registry_refresh]
  - fmt, Clippy, the Octet gate through the repository hooks, strict Cairn validation (0 issues) and the proposal, design and tasks gates (0 issues each) passed; `cargo check --workspace --all-targets` compiled the refreshed graph.
