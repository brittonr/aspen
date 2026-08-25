# Validation evidence

## Scope

This change removes the static all-zero source-gate reference from the production profile. Export now requires an explicit candidate source reference and binds it as the sole source-gate input.

Base source commit: `276f1327e88cdbb53213d9b07e9b254a009c197d`.

## Baseline

Before the change, `production-profile-fixtures` and `nickel-toolchain-cohort` passed.

`contract-export-drift-gate` failed before the change because the generated Cairn policy differs from its Nickel source. The failure is unrelated to production profile evaluation.

## Candidate input evidence

Direct production profile export without `candidate_source_ref` failed.
Customized export with the deterministic conformance reference passed.
The customized export matched `docs/production-profile-fixtures/valid.ncl` exactly.
All-zero candidate input failed contract validation.
A valid candidate paired with another source-gate reference failed contract validation.

The positive fixture names its deterministic candidate explicitly and states that it is not release evidence.

## Nix evidence

`nix build .#checks.x86_64-linux.production-profile-fixtures .#checks.x86_64-linux.nickel-toolchain-cohort -L --option builders "" --no-link` passed.

The post-change `contract-export-drift-gate` reached the same pre-existing generated Cairn policy drift. It reported no production profile failure.

`nix build .#checks.x86_64-linux.molten -L --option builders "" --no-link` passed.
The broad Nix rail ran 1,414 tests with no failures or skips.
Its CI receipt is `blake3:05aef83ce3a2036a2533ee5b8eb710526565fa14fd32cc9854296869a9101e6a`.

Strict Cairn validation passed before sync.

Final gate receipts before sync:

- proposal: `d96da038b24b4260c13fae50f925840293d39fd269803ba50f345842b61ad877`
- design: `41b58d72a3ed3cf618bc4990ad5ee99b7cdc20c29be53094bb9b913fe0858624`
- tasks: `e34334f11274a2547cbb56cac37e481f1ec7565caf726eb5f9e124974b6a64fb`

The sync dry-run passed with plan `1db6d0a569bb13cc9849f72c0ee8ba60a659c1f7c7e9c6d186a05fdf90030afa`.
The executed sync added both requirements to the accepted node-runtime specification.
The sync receipt is `78ad904a4248c5968a3c7b89ed3820abf0f6bf911fa0c869bf252c79788b3b0e`.
Strict Cairn validation passed after sync.

The archive dry-run passed with plan `aef4f37b33f31024ace9cb59cb1e7a82c38e23fc75e7d8996888c45c76b12eea`.
Archive execution moved the package to `2026-08-25-require-production-profile-candidate-input`.
The archive receipt is `9ce0a2847cf0d4ab5a2c43cba17dd79ac22df19feee66ff6d902b3b325107a01`.
Strict Cairn validation passed after archive execution.

## Non-claims

A supplied candidate reference proves only that the exported profile records the supplied canonical value consistently.

It does not prove source identity, source-gate success, freshness, evidence truth, deployment success, runtime authority, or release eligibility.
