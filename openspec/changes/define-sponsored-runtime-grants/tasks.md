## Phase 1: Spec foundation

- [x] Create proposal, design, tasks, and delta specs for sponsored runtime resource grants.

## Phase 2: Core model and admission semantics

- [x] Add portable Rust-owned DTOs for principal references, resource grants, grant scopes, sponsor/beneficiary/provider/workload/service principal roles, node/plugin principal refs, resource limits, quota reservations, quota consumption, revocation refs, settlement references, and usage receipts. ✅ 1m 14s (2026-05-07T01:34:06Z → 2026-05-07T01:35:20Z; evidence: `evidence/sponsored-runtime-dtos.md`)
- [ ] Add pure tests for bounded resources, validity windows, workload/provider scopes, settlement-reference opacity, redaction, and quota arithmetic.
- [ ] Add fail-closed admission tests covering missing principal proof, expired grant, revoked grant, provider-principal rejection, unsupported settlement tag, quota exhaustion, isolation mismatch, and workload/service-principal scope mismatch.

## Phase 3: Provider and sponsor policy contracts

- [ ] Add Nickel-authored contracts and fixtures for provider offers, sponsor policies, resource class catalogs, and admission profiles.
- [ ] Add positive/negative Nickel fixture tests proving policy defaults, bounded limits, secret-free settlement refs, and invalid provider/sponsor principal combinations are rejected before Rust runtime side effects.

## Phase 4: Rust-derived evidence contracts

- [ ] Register sponsored runtime grant, quota ledger, and usage receipt DTOs as `rust-derived` Nickel contract families.
- [ ] Add generated-contract freshness, Rust serialization round-trip, and Nickel validation tests for valid receipts plus malformed, missing, out-of-range, unknown-field, and secret-bearing receipt fixtures.

## Phase 5: Runtime integration and receipts

- [ ] Wire sponsored admission as an optional runtime-service/job/CI placement constraint that does not admit workloads without an accepted grant when sponsorship is required.
- [ ] Emit signed, redacted usage receipts for sponsored runtime execution start, reservation, consumption, completion, failure, and revocation-denial paths.

## Phase 6: Documentation and validation

- [ ] Document the sponsored execution boundary: Aspen enforces resource authority and receipts, while bilateral settlement stays external and currency-neutral.
- [ ] Run focused Rust/Nickel tests, strict OpenSpec validation, and whitespace checks.
