# Supply-chain boundary audit evidence

Generated: `2026-05-05T12:49:08Z`

Status: `pass`

## Scope

- Nix flake inputs and `flake.lock` pin/hash material.
- Cargo workspace git dependencies and `Cargo.lock` git package revisions.
- Production Nix fetcher calls with nearby fixed-output hashes.
- Aspen-owned Rust unsafe/public-unsafe surfaces.
- Owned and vendored `build.rs` inventory.

## Deterministic checker

Added `scripts/audit-supply-chain-boundaries.py`, a read-only checker that emits a JSON receipt and fails on:

- locked flake nodes without `narHash`;
- GitHub/git/tarball locked inputs without 40-hex `rev`;
- workspace Cargo git dependencies without 40-hex `rev` or with floating branch/tag selectors;
- `Cargo.lock` git packages without locked revisions;
- production Nix fetcher calls without nearby `hash`/`sha256`/`sha512`;
- Aspen-owned public `unsafe fn` APIs.

The checker intentionally excludes vendored and test-fixture Nix files from the production fetcher hash gate, while still inventorying vendored Rust/build-script surfaces separately.

## Results

- Locked flake nodes: `16`; hashed nodes: `16`; revision-pinned nodes: `16`.
- Workspace Cargo git dependencies: `11`.
- `Cargo.lock` git packages with locked revisions: `20`.
- Production Nix fetchers audited: `8`; with nearby fixed-output hashes: `8`.
- Aspen-owned Rust files scanned: `1750`.
- Aspen-owned unsafe blocks inventoried: `50`.
- Aspen-owned public unsafe APIs: `0`.
- Vendored public unsafe APIs: `0`.
- Owned build scripts: `build.rs, crates/aspen-ci/build.rs, crates/aspen-ci-executor-nix/build.rs, crates/aspen-cli/build.rs, crates/aspen-nickel/build.rs`.
- Vendored build scripts: `vendor/cargo-hyperlight/build.rs, vendor/snix-glue/build.rs`.

Findings: `0`.

## Source handles

- `flake.nix:4-67` — flake inputs include GitHub/tarball sources; lockfile carries concrete revisions and NAR hashes.
- `flake.lock` — lock material audited by the checker.
- `Cargo.toml:245-924` — workspace dependency table includes pinned SNIX git dependencies and vendored/path overrides.
- `Cargo.lock` — git package lock entries audited for revision identity.
- `scripts/audit-supply-chain-boundaries.py` — deterministic audit fixture added by this slice.

## Verification

- `python scripts/audit-supply-chain-boundaries.py --json` — pass.

## Residual risk

- This slice verifies pin/hash/public-unsafe/build-script invariants; it does not replace RustSec/cargo-deny vulnerability review.
- Vendored code is inventoried separately from Aspen-owned code; deeper vendored patch review remains manual/source-specific.
- Release artifact signing/attestation is not claimed beyond Nix/Cargo identity pins and build-script inventory.
