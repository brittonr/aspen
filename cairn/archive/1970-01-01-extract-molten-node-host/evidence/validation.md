# Molten node-host extraction validation

## Scope

This evidence covers the first `molten-node-host` extraction slice.
The new crate owns the shared error type, capability-rooted node state, and typed local stores.
The root crate keeps exact compatibility re-exports.
CLI, daemon, service, workload, NixOS, release, transport, and network authority remain in the parent crate.

The implementation started from published `origin/molten` revision `ee3998eca2fc8a1d119407e3d58cc501212a1be3` in an isolated worktree.
The dirty primary checkout was not changed.

## Dependency and authority boundary

`crates/molten-node-host/Cargo.toml` has exactly these production dependencies:

- `cap-fs-ext`
- `cap-std`
- `molten-core`

The boundary test admits that exact set. It rejects a forbidden host dependency, a missing `molten-core` dependency, and a malformed manifest.
The crate accepts open capability directories. It does not accept ambient host paths.

The parent facades preserve exact type identity for:

- `molten::error`
- `molten::node_state`
- `molten::local_store`

The facade tests use compile-time type equality and shared behavior checks.

## Source identities

The crate-tree identity is BLAKE3 `cd7b540ded45603dcda1e58e736cdb397a89378042b80b3b01aaca92f25420e9`.
It is the BLAKE3 hash of the sorted per-file BLAKE3 manifest for every file under `crates/molten-node-host`.
The manifest binds each relative path and file content.

The integration identity is BLAKE3 `004ead521e9e954a414297623d5b8b85ba5b1355ce90bc049caaf8e78c59b51b`.
It is the BLAKE3 hash of the per-file BLAKE3 manifest for the workspace manifests, generated plans, Nix integration, parent facades, facade tests, Octet hash helper, and changed architecture documents.

The settled Cargo lock SHA-256 recorded by unit2nix is `4c259d5abf426ea23c000e9081afdddf8dd3483d4cbbccf91a22aa1ea4a14a91`.
SHA-256 is used here because unit2nix defines this interoperability field.

## Cargo and Clippy

Pueue task `19251` ran the final post-layout code rail through `nix develop path:$PWD` with isolated `CARGO_TARGET_DIR=/home/brittonr/.cargo-target/molten-node-host`:

- package-scoped `cargo fmt --check`
- `cargo test --workspace --locked`
- `cargo clippy --workspace --all-targets --locked --no-deps -- -D warnings`
- final `cargo octet check`

All commands passed.
The workspace test suite ran 1,548 tests: 1,548 passed and none failed.
The seven tests added by this change are five crate-boundary tests and two facade tests.
They include positive and negative cases.

## Octet and Tiger Style

The published-source baseline was rerun from the separate baseline worktree in pueue task `19229`.
It reported warning-only status, 5,837 warnings, and zero errors.

- baseline config: `b3:f46bcd074f1429a664e8c38d48b7f1b2d7faf81e659e346fdcc09c059b2492a1`
- baseline profile: `b3:3411e1932dfc8832ce673a51bfc6ca916eeaf3789eb75237be36237b8abf13a1`

The final extraction report from pueue task `19251` has warning-only status, 5,844 warnings, and zero errors.

- final config: `b3:7785d20c1afa36c6228a4482b5743abe576cab9ba8368c7f6259cc5541f0b50e`
- final profile: `b3:0f3ef6eff558b20e7d997122120c62fae11909f3673580182ab36ac6762eb369`

The total delta is seven warnings.
Only two lint totals changed: `path_segment_repetition` increased by two, and `underscore_in_module_filename` increased by five.
Every other lint total stayed equal.
The two path findings bind the exact public `MoltenError` variant/type identity in its new crate context.
The underscore delta is package-relative source attribution, including remapped dependency files and the new package/test topology.
It is not a new runtime or authority behavior.
The boundary test target itself has no Octet findings.

The extracted and retained implementation/test surfaces keep the same 173 source findings when the three local-store test findings that remain in the parent are counted with the extracted crate.
No Octet baseline was regenerated.

`octet-precommit-check` is not available in the dev shell.
Pueue task `19230` ran the repository fallback, `cargo tigerstyle check`, through the Nix shell.
It completed successfully with the same warning-only Octet result and zero errors.

## Generated plans

The final pinned unit2nix generation produced:

- default plan: 633 crates, three named workspace members plus the root package, 884 build units, and 898 test units
- release-policy plan: 49 crates, two workspace members, and 66 build units

Both plans bind the settled Cargo lock hash.
The package-scoped release plan was regenerated after the lock changed.
No generated plan or lock file was edited by hand.

## Nix validation

Pueue task `19173` passed these focused checks:

- `nix build path:$PWD#molten-node-host -L`
- `nix build path:$PWD#checks.x86_64-linux.molten-node-host -L`
- `nix build path:$PWD#checks.x86_64-linux.node-state-authority -L`

The configured SSH builder was unavailable, and Nix completed the builds locally.

Pueue task `19252` ran the final post-layout `nix flake check path:$PWD -L`.
All checks passed.
The hermetic nextest check ran 1,365 tests with no failures or skips.
Its CI receipt is `blake3:119863d095ddf1be044ed24a3830bf32718ccf3ca7a90745607cea4550ecbf83` with decision `pass`.
The dogfood release-evidence path also passed, including bundle verification, signed promotion verification, and export verification.

The flake check evaluated the supported `x86_64-linux` system.
It reported the existing incompatible-system omission for Darwin and `aarch64-linux`.

## Cairn and traceability

All lifecycle commands used pinned Cairn revision `3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`.
The current Cairn release rejects this repository's legacy `cairn/` layout, so no unrelated lifecycle migration was attempted.

Pueue task `19282` returned valid repository state with no change or spec issues.
All stage gates passed:

- proposal: `1bde96f25ba4fdf2d1422ad8a78038c411e3637ac8f565b38df3f2046cf89867`
- design: `380bfd4acb59ab4b44e9570d4b60811dd27a4387f63910fb19c4730346cab086`
- tasks: `ad1ea95b002692202aa024ba9d4259594dcb5fb8f0a2c003c861fc8b56831368`

Pre-sync Tracey returned nonzero because the repository has inherited missing coverage and one unrelated dangling marker.
The new `crate_boundary` and `facade_compatibility` markers were dangling because their requirements were still in the active delta.

Pueue task `19283` synced the three accepted `node-runtime` requirements.

- sync plan: `f1e6fc26b860b261e9ce5016f1780e67d2b5d59eb2c01f9a58dda93e046e07f6`
- mutation manifest: `33b14628dcb65f745716d9324111bb4228f46b02979dc7a455e3ce3c3275c4a6`
- sync receipt: `2d8c6d975427786e8137692a48036b31ceb41f5f6ff49143d3712bd510140a36`
- accepted spec content: `c93126ab7fe01d702602ae450d337d297b0d69e69e0ecd23b14fdb9e5eb95faa`

Post-sync validation remained valid with no issues.
Post-sync Tracey still returns nonzero for inherited repository debt, but no `molten.node_host.*` requirement is missing or dangling.
The global Tracey result is not a repository-wide coverage claim.

Pueue task `19303` archived the change under `cairn/archive/1970-01-01-extract-molten-node-host/`.
The pinned Cairn policy uses a deterministic `1970-01-01` archive date.

- archive plan: `94e8a3b659331c700c5683ec98286778eddbcdce9c13050dc89cef00de367a0c`
- archive mutation manifest: `4f9f29e19963fd30c158d043402ec8ec0bd958ee5801a56adf20a53c2c624d42`
- archive receipt: `95948b834c34741432be16eb1171b18e44858b50799f5896a7e13127a95a3b6b`

Post-archive validation returned valid state with no issues.
The active change is absent, the archive is present, and 15 unrelated active changes remain.
Post-archive Tracey has no missing or dangling `molten.node_host.*` requirement.

## Claim boundary

These results show dependency shape, facade identity, capability-root behavior, deterministic local-store behavior, source-policy observations, and the listed build/test outcomes.
They do not grant filesystem authority, network authority, release eligibility, deployment approval, transport trust, semantic correctness, or whole-system correctness.
