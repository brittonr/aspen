# Sponsored runtime Nickel-authored policy contracts

- Change: `define-sponsored-runtime-grants`
- Task: Nickel-authored contracts and fixtures for provider offers, sponsor policies, resource class catalogs, and admission profiles
- Started: `2026-05-07T01:42:38Z`
- Completed: `2026-05-07T01:47:05Z`

## Implemented

Added `schemas/sponsored-runtime-policy.ncl` as the Nickel-owned human-authored sponsorship policy/config boundary. It defines contracts for:

- `ProviderOffer`
- `SponsorPolicy`
- `ResourceClassCatalog`
- `AdmissionProfile`
- principal refs constrained by expected sponsor/beneficiary/provider/workload/service roles
- resource limits, isolation classes, settlement kinds, and secret-free settlement/revocation refs

Updated typed Nickel registry docs and machine-readable registry to include `sponsored-runtime-policy` as `nickel-authored`.

Updated typed Nickel fixture checker to typecheck the sponsorship schema and exercise one positive policy bundle plus negative secret-bearing and provider-role fixtures.

## Verification

- `python3 scripts/check-typed-nickel-contract-fixtures.py`
- `python3 scripts/check-typed-nickel-contract-registry.py`

Result: fixture checker passed with 13 typechecks, 8 positive exports, 4 negative exports; registry checker passed with 13 families.
