# Validation evidence

<!-- r[verify molten.runtime_spine.preserves_boundary_profile.final_validation] -->
<!-- r[verify molten.runtime_spine.preserves_boundary_profile.final_validation.fixtures] -->

## Goal and completion boundary

The goal was to reduce inherited runtime-spine debt through a bounded review of nine Preserves boundary-profile requirements.
Completion required direct implementation and verification markers, typed candidate evidence, persistent fixture checks, exact baseline regeneration, zero dangling references, and full repository validation.

## Canonical input

Base revision:

`9842cc6453a00ed943a866549bb4307c8502d575`

Pinned Cairn revision:

`3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`

Starting inherited debt: 1,933 requirements.
Starting runtime-spine debt: 396 requirements.
Starting Preserves boundary-profile debt: 9 requirements.

## Approach registry

### Typed profile contract

Mechanism: inspect the Nickel profile contract and valid data for field shape, admitted families, unique family rows, consumers, canonical bytes, BLAKE3 refs, adapter ownership, typed DTOs, and non-claims.

Result: the contract records every required field for node-control envelopes, tickets, workflow bundles, receipts, and evidence envelopes.

### Pure Rust validation

Mechanism: inspect and run `molten_core::preserves_profile` tests over canonical measurements and explicit failure classes.

Result: the pure deterministic core accepts the five valid surfaces and rejects non-canonical bytes, missing labels, stale refs, unsupported consumers, missing non-claims, and raw core coupling.

### Dependency boundary audit

Mechanism: inspect `crates/molten-core/Cargo.toml` and structurally search the crate for direct Preserves imports and paths.

Result: the crate has no Preserves dependency or direct Preserves import.
The raw-coupling measurement remains an explicit supplied fact and is not promoted into an ambient source-scan claim.

### Fixture and documentation audit

Mechanism: export the valid Nickel fixture, require all four invalid fixtures to fail, and inspect the modularity guide.

Result: one positive and four negative Nickel fixtures match their expected outcomes.
The guide limits profile success to canonical boundary identity and adapter placement.

### Persistent validation audit

Mechanism: add the profile fixtures and documentation assertions to the repository-owned Nix contract export gate.
Bind typed direct-repair evidence and exact baseline removal to the inherited-debt check.

Result: both focused Nix checks and the full Nix suite pass.

The serial search passes are correlated.
No subagents were used.

## Accepted repairs

The typed manifest records nine direct repairs:

- `molten.runtime_spine.preserves_boundary_profile.contract`;
- `molten.runtime_spine.preserves_boundary_profile.docs`;
- `molten.runtime_spine.preserves_boundary_profile.docs.non_claims`;
- `molten.runtime_spine.preserves_boundary_profile.final_validation`;
- `molten.runtime_spine.preserves_boundary_profile.final_validation.fixtures`;
- `molten.runtime_spine.preserves_boundary_profile.fixtures.negative`;
- `molten.runtime_spine.preserves_boundary_profile.fixtures.positive`;
- `molten.runtime_spine.preserves_boundary_profile.validation`;
- `molten.runtime_spine.preserves_boundary_profile.validation.core_coupling`.

No candidate was rejected.
The patch adds direct markers, deterministic evidence, and persistent fixture validation.
Production runtime behavior did not change.

## Final inventory

The comprehensive guard reports:

- requirements: 2,508;
- referenced: 584;
- uncovered: 1,924;
- dangling: zero;
- verdict: pass against the exact baseline.

The grouped classifier reports:

- classified entries: 1,924;
- specification groups: 35;
- source area groups: 107;
- runtime-spine entries: 387;
- Preserves boundary-profile entries: zero;
- verdict: pass.

The inherited baseline decreased by nine entries.
The runtime-spine queue decreased from 396 to 387 entries.

## Identities

Baseline BLAKE3:

`c67c691d1fb65706aa916d5d906d91a3081d3b2aad947a99a698ac222bef1c19`

Classification TSV BLAKE3:

`0d0b2f33c9414d9d993eb314604f97a877be9fa7aeaff4bb28e71732a035d0fc`

Classification summary BLAKE3:

`38f676813c4d388cdf1187508c18d4697fa714551c4d650f2bb00453c1a8a107`

Generated baseline JSON BLAKE3:

`82210862f84dacc39e98432df7935b3285126c29c12f282eb398cbcaf607443e`

Generated classification JSON BLAKE3:

`da5c59588d2a99dc52384943512f4d73b4839c9a677cd92a24688e08e9ef40c8`

Generated Preserves boundary-profile repair JSON BLAKE3:

`41838bf01611a44df760ff549948109951655246fe974ce3e732901d47d02234`

## Validation

The following checks passed:

- pre-change focused Rust profile tests: 3 passed;
- post-change focused Rust profile tests: 3 passed;
- pre-change Nickel fixtures: 1 positive and 4 negative outcomes passed;
- post-change Nickel fixtures: 1 positive and 4 negative outcomes passed;
- inherited debt guard tests: 4 passed;
- classification tests: 4 passed;
- typed Nickel manifest checks and deterministic JSON exports;
- exact candidate, marker, and baseline checks;
- focused `contract-export-drift-gate` Nix check;
- focused `inherited-tracey-debt` Nix check;
- Cargo formatting;
- `cargo tigerstyle check` with the repository baseline;
- pinned Cairn validation;
- proposal, design, and tasks gates;
- full `nix flake check path:$PWD -L`;
- Nix nextest: 1,365 passed;
- `git diff --check`.

Full Nix CI test receipt:

`blake3:8bea6774482dc38daf6410da55fc93c3d493c792c5d391724402646b7699fbdd`

Lifecycle gate receipts before archive:

- proposal: `ba6dbf76e3552de1998aec58163c84a90d7b792165c182066c5d3f5d06a37afb`;
- design: `5b9904b93102da5a69ef5eb786945efa97b75582493959ac31a2c07e289fb570`;
- tasks: `66ccf92168c91106d5d9f78a00ff7430ebfba63298f912ea9521612829edc9c9`.

Sync mutation manifest:

`0baf4e5ea58f583d500f0595e3a115c6a53330a3a33272c6a2a51d54d76c7c3d`

Sync receipt:

`91231c5ae5f10dd91147bb59cc15a808f9a5918410c805060913ca9f7ce117d8`

## Compatibility checker boundary

The pinned compatibility checker reports 2,508 requirements, 233 references, 2,275 missing requirements, and zero dangling references.
It still fails because it scans only `crates/` and `tools/`.
The comprehensive repository guard scans the admitted source, test, tool, documentation, script, and flake roots.

## Terminal result and non-claims

The search budget ended after five serial mechanisms, focused tests, adversarial audit, and full deterministic validation.
All nine candidates are validated.

This batch does not prove transport liveness, actor authority correctness, replay completeness, Valence Evidence IR acceptance, universal adapter separation, complete runtime-spine coverage, release readiness, or whole-system correctness.
