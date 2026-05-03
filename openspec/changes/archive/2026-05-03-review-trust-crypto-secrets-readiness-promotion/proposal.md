# Review trust crypto secrets readiness promotion

## Why

The known blockers from the owner/public API review are now complete: real crate checker coverage, explicit `aspen-trust` async/default policy, and deterministic trust/secrets serialization contracts. The readiness decision needs to be recorded without overstating runtime/service surfaces.

## What Changes

- Promote only the real reusable package candidates: `aspen-crypto`, default/no-default `aspen-trust`, and `aspen-secrets-core`.
- Keep the aggregate `trust-crypto-secrets` family `workspace-internal` because runtime/service/compatibility surfaces remain excluded.
- Record evidence tying the decision to checker output, dependency boundaries, serialization contracts, and runtime compatibility.

## Impact

- **Files**: crate-extraction policy, inventory, trust/crypto/secrets manifest, OpenSpec spec and change evidence.
- **APIs**: no Rust API changes.
- **Dependencies**: no dependency changes.
- **Testing**: readiness checker, targeted crate checks, serialization tests, compatibility checks, OpenSpec validation, markdownlint, and diff checks.
