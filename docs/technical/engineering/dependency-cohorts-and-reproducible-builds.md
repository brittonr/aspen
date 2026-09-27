# Dependency Cohorts and Reproducible Builds

This article explains how Molten relates reviewed dependency identities, resolved build inputs, and release-policy evidence. It assumes familiarity with Cargo manifests and Nix inputs. The governing [reproducible dependencies guide](../../reproducible-dependencies.md) defines the update procedure; this companion explains why its several representations cannot be reduced to a single lockfile. Return to the [Technical companion](../README.md).

## A cohort is a consistency obligation

Here, *cohort* means the related representations that must move together during a dependency update, not a new repository object or API. A dependency has a manifest key, package name and version, reviewed source coordinate, immutable revision, corresponding Nix input, transport policy, and release disposition. The inspected `DependencyExpectation` represents these separately in the [pure validator](../../../crates/molten-core/src/release_dependency.rs). Separating them matters: two manifest keys can refer to the same package, while the same package name and version can resolve from distinct source identities.

`DependencyObservation` records manifest, lock, and Nix observations rather than treating any one as authoritative by itself. `ResolvedPackageIdentity` adds graph-level package/source/revision information. The validator checks both profile rows and resolved package identities, so reviewing a direct declaration is not a substitute for inspecting what the graph actually selected.

The governing guide gives a concrete example: canonical `valence-core` comes from standalone Valence, while Octet remains a separately pinned proof-tool provider. Exactly one canonical Valence source identity is expected. Upstream migration evidence and the package implementing the semantics therefore have different roles even when they participated in the same cutover.

## Observation belongs outside the pure decision

The release-policy executable loads the profile, observes repository material, constructs `ReleaseDependencyInput`, and calls `validate_release_dependencies`. The [shell implementation](../../../crates/molten-release-policy/src/main.rs) then hashes `report.canonical_material` with BLAKE3 and renders pass or denial diagnostics. Reading files and obtaining observations are not hidden inside the pure validator.

The validator checks bounds, rows, resolved identities, unprofiled Git dependencies, canonical Valence, archived evidence, and distribution observations. It sorts and deduplicates diagnostics before constructing canonical report material. This is an in-memory decision over supplied facts. It does not independently fetch a repository, establish that an archive was honestly measured, or authorize a release.

That distinction also clarifies reproducibility. A stable report identity is an identity for the report material under this contract. It is not a binary reproducibility certificate, a signature from an upstream maintainer, or proof that arbitrary build scripts lack environmental inputs. The [modularity inventory](../../modularity-boundaries.md) keeps policy/evidence processing separate from runtime authority for the same reason.

## Three different identities in one update

Source revisions, source-tree hashes, and report hashes answer different questions:

- The immutable Git revision identifies the reviewed upstream revision. The validator requires the configured exact revision form rather than accepting a moving branch as equivalent.
- Metadata-free NAR hashes bind the Git-source material used by Nix fetching and build plans. The governing update procedure specifically warns that a checkout containing `.git` is not the same material as `pkgs.fetchgit` produces after removing it.
- The BLAKE3 report identity binds normalized validation material and diagnostics, not the output binary or a complete runtime execution.

Consequently, a successful local build from an already-present Nix store path can conceal a source-hash mismatch that a fresh builder encounters. The documented `git-source-hashes.sh` workflow addresses that distinct failure mode; changing only `Cargo.lock` does not.

## Worked failure scenario

Consider an illustrative update that changes a reviewed dependency revision in the Nickel profile and Cargo manifest but leaves the matching Nix lock input at the old revision. Cargo might build the new source locally, yet the cohort is inconsistent. The validator has a dedicated `NixDrift` diagnostic category because Cargo success cannot establish agreement with the Nix observation.

Now suppose the engineer updates the Nix input but lets a prefetched checkout containing Git metadata supply the build-plan source hash. The release identity comparison and the fetch hash problem are different obligations. Repairing revision agreement does not establish that the metadata-free source hash is correct. Conversely, fixing the source hash without updating reviewed expectations would not justify the new revision.

The appropriate review follows the governing sequence: review the upstream change, update profile and manifest, update the matching input, regenerate locks through their tools, regenerate source hashes and both build plans, then exercise positive and negative checks. This is a coordinated identity change, not several unrelated bookkeeping edits.

## Verification and limits

Suggested verification is the focused `molten-release-policy` invocation in the governing guide with explicit Valence and Octet evidence-source paths, followed by its source-hash binding checks and Nix checks. Those commands are not reported as executed here. Review denial behavior as well as a passing fixture: stale evidence bytes, a second canonical package source, a missing distribution artifact, and unprofiled Git input are materially different failures.

Local `--override-input` paths remain development conveniences; they do not replace reviewed release identities. The AGPL distribution profile records project-policy artifacts and non-claims, not legal advice or universal compliance. None of this grants execution authority, validates upstream correctness, or establishes production readiness.

## Sources

- [Reproducible release dependencies](../../reproducible-dependencies.md)
- [Modularity boundary inventory](../../modularity-boundaries.md)
- [Release dependency validator and embedded tests](../../../crates/molten-core/src/release_dependency.rs)
- [Release-policy shell](../../../crates/molten-release-policy/src/main.rs)
- [Technical companion](../README.md)
