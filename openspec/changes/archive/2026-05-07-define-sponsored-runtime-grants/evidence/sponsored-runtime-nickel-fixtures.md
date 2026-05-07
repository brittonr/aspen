# Sponsored runtime Nickel fixture coverage

- Change: `define-sponsored-runtime-grants`
- Task: positive/negative Nickel fixture tests
- Started: `2026-05-07T01:47:44Z`
- Completed: `2026-05-07T01:48:33Z`

## Implemented

Expanded `scripts/check-typed-nickel-contract-fixtures.py` with sponsorship fixtures proving:

- policy defaults are accepted for `ProviderOffer.max_concurrent`;
- bounded resource limits reject zero limits;
- settlement references reject raw secret-like material;
- provider principal combinations reject non-provider roles;
- sponsor policy combinations reject non-sponsor roles;
- admission profile costs reject negative values before Rust runtime side effects.

## Verification

- `python3 scripts/check-typed-nickel-contract-fixtures.py`

Result: fixture checker passed with 13 typechecks, 9 positive exports, and 7 negative exports.
