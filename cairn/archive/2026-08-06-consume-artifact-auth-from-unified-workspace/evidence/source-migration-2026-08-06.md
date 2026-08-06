# Unified Artifact source migration evidence

Date: 2026-08-06

## Result

Molten now resolves authentication and binding from one immutable Artifact source.

- Repository: `ssh://git@github.com/OnixResearch/onix-artifact.git`
- Revision: `c932138d880ddf4c2967f4c024b489b5c0022bf1`
- NAR hash: `sha256-XGQLG60DNeY9FUYcOmn6cfYnhCIJzyqf+VW9yofDYFU=`
- Archive BLAKE3: `3878cdb892bfd4a8eac5779023ba32871d61bc3e8ec7c9ef5f7ab790a8acfeb9`

The source has four workspace packages. Molten selects these packages:

- `artifact-auth-core`
- `artifact-auth-ed25519`
- `artifact-binding-core`

Molten does not select `artifact-transfer-core`.

## Generated state

The following files were generated after the manifest changes:

- `Cargo.lock`
- `flake.lock`
- `build-plan.json`
- `release-policy-build-plan.json`

Both unit2nix plans resolve only the selected packages from the unified source. The release policy records one immutable source row for each selected package.

## Evidence

The typed source receipt is in `evidence/source/artifact-workspace-migration-v1.ncl`. Its exported JSON and BLAKE3 sidecar are in the same directory.

The validator accepts the exact migration receipt. Negative tests reject:

- a stale revision
- a stale NAR hash
- a missing package
- a widened package set
- an incorrect consumer binding set
- incorrect auth entry hashes
- a mismatched legacy revision

The former Radicle cutover receipt remains historical evidence. It is not used as current source identity.

## Validation

The predecessor baseline passed all four focused authentication tests and its source check.

The following post-change checks passed:

```text
nix develop -c cargo fmt --all -- --check
nix develop -c cargo test -p molten-core artifact_auth --lib
nix develop -c cargo test -p molten-core live_binding --lib
nix develop -c cargo clippy -p molten-core --all-targets -- -D warnings
nix build .#checks.x86_64-linux.artifact-auth-radicle-cutover --no-link -L
```

Results:

- Authentication tests: 4 passed.
- Binding tests: 14 passed.
- Clippy: passed with warnings denied.
- The Nix source check verified the source lock, package metadata, unit2nix plans, release policy, migration receipt, and predecessor-source absence.

## Cairn validation

The change-local proposal and design gates use the current canonical Cairn policy. Repository-wide validation remains blocked by an unrelated malformed dependency marker in `cairn/changes/add-collaboration-scope-system-extension/tasks.md:14`.

The repository-local policy also uses an obsolete schema. This migration does not alter either unrelated lifecycle surface.

## Claim boundary

This evidence proves source selection, package identity, consumer graph shape, and local test results. It does not prove whole-system correctness, release eligibility, revocation freshness, or external authority.
