# Design

## Context

The public API review identified `aspen-crypto` and `aspen-secrets-core` as canonical reusable surfaces. It explicitly did not promote `aspen-trust`, `aspen-secrets`, or `aspen-secrets-handler` as reusable package roots.

## Decision

Map the `trust-crypto-secrets` candidate family to two real candidate keys, `aspen_crypto` and `aspen_secrets_core`, each with a real Cargo package mapping and policy entry.

## Rationale

This removes the pseudo-candidate warning while avoiding false promotion. `aspen-trust` stays outside the checker mapping until async/default dependency policy and serialization-contract evidence are settled. Runtime crates remain representative consumers or compatibility surfaces rather than reusable roots.

## Risks

- Mapping only one crate would under-check the family.
- Including `aspen-trust` now would overstate readiness.
- Promotion state must remain `workspace-internal`.
