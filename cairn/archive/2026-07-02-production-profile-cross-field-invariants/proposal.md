# Change: production-profile-cross-field-invariants

## Why

Scalar validation is not enough for production deployment profiles. Some profile fields are only meaningful in relation to others: source-gate evidence must be present, required adapter sets must include the production spine, layout directories must not collide, and resource limits must be internally coherent.

## What

- Add Nickel contracts or checked derived fields that validate non-empty evidence arrays, required adapter coverage, distinct state layout directories, and coherent resource-limit relationships.
- Fail profile export when cross-field invariants are violated.
- Keep live capacity, filesystem existence, and adapter health checks in runtime receipts rather than in Nickel.

## Impact

Operators get early deterministic failures for inconsistent profiles. Production readiness receipts no longer need to interpret structurally valid but internally incoherent profile exports.
