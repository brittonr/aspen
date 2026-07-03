# Change: production-profile-contract-library-split

## Why

The production profile currently mixes reusable contract definitions and the concrete pilot profile in one documentation file. That makes it harder to reuse the same schema for additional node profiles, fixtures, and negative tests without copying contracts.

## What

- Extract reusable production profile Nickel contracts into a dedicated contract module.
- Keep the concrete checked-in pilot profile as a small instance that imports and applies the shared contract.
- Document the boundary between reusable contract definitions, concrete profile data, and generated JSON exports.

## Impact

Multiple production-shaped profiles and fixtures can share one reviewed contract. The operator-facing profile remains simple, and future contract changes happen in one place.
