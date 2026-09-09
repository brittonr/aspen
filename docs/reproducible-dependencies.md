# Reproducible release dependencies

Molten release dependency identity is authored in the typed Nickel profile at
[`config/release-dependencies/profile.ncl`](../config/release-dependencies/profile.ncl).
The profile binds each direct Git dependency to its package identity, reviewed
source coordinate, immutable revision, Nix input, release disposition, and
transport policy. The Artifact packages use one exact SSH Git pin and one non-flake Nix input.
Bounded Exec uses its canonical read-only HTTPS source at revision
`29dac88ecded94457572db3fdfaaaab95fa91525` and one matching non-flake input.
Other OnixResearch Cargo sources retain their reviewed pins and immutable
archive inputs for sandboxed Nix builds.

The `molten-release-policy` shell reads the Nickel export, `Cargo.toml`,
`Cargo.lock`, `flake.lock`, configured archive evidence, and distribution
artifacts. It passes normalized in-memory observations to the pure
`molten_core::release_dependency` validator. The core sorts diagnostics and
canonical report material, so input ordering cannot change the result or its
BLAKE3 identity. Filesystem reads, Nickel execution, hashing of supplied
archive files, and diagnostic rendering remain in the shell.

Run the focused check from the repository root:

```sh
nix develop -c cargo run -p molten-release-policy -- \
  --root . \
  --evidence-source valence-integrity=../valence \
  --evidence-source octet-cutover=../octet
```

`nix flake check` runs the same validator against exact Nix source inputs. Local
`--override-input` paths are development conveniences only and cannot replace
reviewed release identities.

## Canonical Valence migration

Molten consumes `valence-core` from standalone Valence revision
`5f1c2ba5072c6f9622fa59b1af20502985f569fd`. This is the revision named by
Octet's completed standalone cutover manifest and contains Valence's archived
`harden-preserves-integrity-boundaries` change. The profile also binds:

- the Valence archive task bytes;
- Octet's archived `complete-standalone-valence-cutover` task bytes;
- Octet's generated cutover manifest and Aspen migration target; and
- the exact Nix revisions used to materialize both evidence sources.

The resolved Cargo graph must contain exactly one `valence-core` source
identity. Octet remains a separately pinned proof-tool provider; it no longer
provides Molten's canonical Valence semantics.

## Updating pins

1. Review the upstream revision and its license/security implications.
2. Update the Nickel profile and the matching Cargo manifest revision.
3. Update the matching `flake.nix` input.
4. Run Cargo to regenerate `Cargo.lock`.
5. Run `nix flake lock` or a targeted Nix lock command to regenerate
   `flake.lock`; never edit it manually.
6. Regenerate `build-plan.json` with the repository's pinned unit2nix tool.
7. Run positive and negative focused checks, then `nix flake check`.

Floating branches, tags without immutable commits, unreviewed SSH-only release
sources, manifest/lock/Nix drift, duplicate canonical package identity, stale archive
bytes, and missing configured distribution evidence fail closed.

## Git source hashes in generated plans

The pinned unit2nix generator at `d4883180de0ce3033b7e4e2ab4216f33134863c5` prefetches Git sources with `--leave-dotGit`.
Its `lib/fetch-source.nix` builder omits that option.
These two source trees have different NAR hashes.
Regeneration alone therefore does not correct this mismatch.

`crate-hashes.json` supplies reviewed, revision-bound hashes for Durable Authority State, Bounded HTTP, ChaosControl, and Kamacite.
The hashes bind the exact source trees without Git metadata, as required by the builder.
Direct `fetchgit` builds verify the Durable Authority State and Bounded HTTP hashes with submodules enabled.
Exact-revision Nix prefetches bind the ChaosControl and Kamacite source trees.
The dependency URLs and revisions remain unchanged.
SHA-256 is required here by the existing Nix source-hash format.

The repository consumes these hashes through the native unit2nix input contract.
The generator applies a revision-bound entry to all packages from that repository and revision.
The following commands regenerate both plans:

```sh
nix develop -c nix run github:brittonr/unit2nix/d4883180de0ce3033b7e4e2ab4216f33134863c5 -- --workspace --force --output build-plan.json
nix develop -c nix run github:brittonr/unit2nix/d4883180de0ce3033b7e4e2ab4216f33134863c5 -- --package molten-release-policy --bin molten-release-policy --force --output release-policy-build-plan.json
```

Successful source fetches do not establish a passing build or release gate.

## AGPL distribution profile

`AGPL-3.0-or-later` is an accepted Molten project choice. The profile records
an immutable project source coordinate/revision, notices, and project-required
source-export artifacts. These records are project-policy evidence only. They
are not legal advice and do not prove compliance in every jurisdiction,
corresponding-source completeness for an unreviewed distribution, upstream
correctness, runtime correctness, or release eligibility.
