# Validation evidence

## Scope

This change adds nominal Rust reference domains after Preserves wire admission.
It integrates typed context, holder, operation, and key admission into current authority decisions.
It does not grant authority or change canonical wire schemas.

Base source commit: `3de348149bc828db6e79c93c72f5b2e6525b2442`.

## Inventory and declarations

`docs/nominal-authority-references.md` inventories 15 selected reference roles and their bounded migration scopes.
`config/nominal-reference-domains.ncl` declares the entity and canonical families, Rust aliases, scopes, BLAKE3 algorithm, wire authority, and non-claims.
The Nix `nominal-reference-domains` check typechecks and exports this profile.

The current Octet cohort does not yet supply nominal-domain enforcement.
The declarations are future adoption input, not present enforcement evidence.

## Type and wire boundary

`molten_core::nominal` owns private generic entity and canonical reference families.
It exposes exact aliases for principal, node, actor, service, session, authority context, delegation, revocation, key, policy, resource, evidence, artifact, operation, and receipt domains.

Entity constructors enforce a named byte bound and canonical spelling.
Canonical constructors accept only lowercase `blake3:` references with the exact digest length.
Constructors expose checked accessors but contain no authority policy.

`authority::nominal` owns unchanged-string wire DTOs and typed admitted sets for authority, execution, artifact, and historical replay seams.
Wire-to-core-to-wire round trips preserve every external field exactly.
The existing authority currentness path now compares typed principals and typed current key references while preserving its policy, expiry, revocation, capability, and scope checks.

## Positive and negative evidence

Nine pure-core tests passed for construction, wire admission, role separation, exact authority roles, authority denial, and evidence-only replay.
Six application wire-adapter tests passed.
Seven compile-fail doctests passed for session/context, policy/evidence, delegation/revocation, key/context, artifact/receipt, operation/resource, and node/principal substitutions.

Focused authority, capability, node-control, effect, artifact, retention, replay, and provenance suites passed.
They include wrong-holder, wrong-session, stale policy, missing resource, expiry, revocation, and possession-without-authority denials.
Existing canonical authority, receipt, ledger, retention, provenance, and replay fixtures passed unchanged.

`cargo fmt --all --check` passed.
`cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.

`cargo octet check` reported 5,833 existing warning-level findings.
This matches the base cohort and adds no finding.
The warning-only result is not strict Octet acceptance evidence.

`nix build .#checks.x86_64-linux.nominal-reference-domains --no-link -L --option builders ""` passed.
`nix build .#checks.x86_64-linux.molten --no-link -L --option builders ""` passed.
The Nix nextest rail ran 1,411 tests with no failures or skips.
Its CI receipt is `blake3:616f5280230fe2912f9b27f89f109ba4bd98a8a4a5a74291cfa4e1647ec746d0`.

Strict Cairn validation passed with the current Cairn policy.
Final gate receipts before sync:

- proposal: `a4378818a2ffcaeb37a8ac8c83413634f0204dbfef9091e172f1ad2455886b00`
- design: `abc49bc8998c9b7317b1a956f9776f64cf504dba41f48f2600c6fc209e286f56`
- tasks: `81d60d2551c13c04af9bfc5a14b18db53797c604e2629c38e49c0f31061cd3d0`

The sync dry-run passed with plan `e2fa218e0ad957e42e91aa61981fc8decc6bf658d9d2d7e0964d8ee072210898`.
The executed sync added all nine requirements to the accepted authority specification.
The sync receipt is `4be100acc7808a6cf1248b4eabd8261c373fb82770fba79eb63c027e4a0bde7d`.
Strict validation passed after sync.

The archive dry-run passed with plan `15bbb96cb98730aece441341cfe705547b969882fbedefc5c7a6a6271cfa493d`.
Archive execution moved the package to `2026-08-24-type-authority-and-artifact-references`.
The archive receipt is `d09baacdb0a1e2b363a5f98fa2c109dc2e5a686d7840bb73019fb1f1535688c4`.
Strict validation passed after archive execution.

## Non-claims

A typed reference proves local category separation and checked syntax only.
It does not prove current authority, freshness, evidence truth, transport identity, semantic equivalence, runtime correctness, or release eligibility.
