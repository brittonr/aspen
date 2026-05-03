# Design

## Context

The aggregate trust/crypto/secrets family includes reusable package surfaces and runtime/service shells. Earlier review intentionally kept the family internal until real checker coverage, async/default policy, and serialization-contract evidence existed. Those blockers are now complete.

## Decision

Promote readiness at package scope, not aggregate scope.

- `aspen-crypto` default/no-default helpers are reusable and transport-free.
- `aspen-trust` default/no-default helper/state/wire surface is reusable; `aspen-trust/async` remains opt-in runtime service API.
- `aspen-secrets-core` owns portable persisted state/config contracts.
- `aspen-secrets` and `aspen-secrets-handler` remain runtime/service or compatibility consumers.

## Non-goals

- No publication/repository-split readiness.
- No promotion of `aspen-secrets`, `aspen-secrets-handler`, `aspen-crypto/identity`, or `aspen-trust/async`.
- No Rust API or dependency changes.

## Risks

The main risk is readers treating the aggregate family row as fully ready. The docs and decision artifact explicitly state that only three real package candidates are promoted.
