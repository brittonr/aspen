# Design: Aspen Trust Async Default Policy

## Context

`aspen-trust` contains pure Shamir/GF256/HKDF/share-chain/reconfiguration helpers and runtime-facing key-manager/reencryption service APIs. The owner/public API review identified the mixed default surface as a readiness blocker: consumers could not tell whether `tokio`/`async-trait` were acceptable reusable defaults or runtime-service implementation details.

## Decision

Default and no-default `aspen-trust` builds SHALL expose only the reusable pure/helper/state/wire modules. Async service modules are behind the explicit `async` feature:

- `key_manager`
- `reencrypt`

Runtime consumers that need those APIs enable `aspen-trust/async`; `aspen-raft` does so through its `trust` feature.

## Alternatives

- **Accept async in default**: rejected because it would leave the owner/public API review ambiguity unresolved and make `tokio` a reusable default dependency.
- **Split a new trust-runtime crate now**: deferred because feature-gating is the smallest compatible boundary and keeps a future crate split possible.
- **Exclude `aspen-trust` from checker coverage**: rejected because selected pure trust helpers are explicit reusable candidates.

## Risks

- Downstream source imports for `key_manager` or `reencrypt` now need the `async` feature.
- Feature unification could hide a default-edge regression, so evidence includes default/no-default dependency guards.
- Serialization/golden contract evidence remains a separate blocker; this change does not promote the family.
