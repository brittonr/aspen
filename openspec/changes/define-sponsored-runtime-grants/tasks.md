## Phase 1: Spec foundation

- [x] Create proposal, design, tasks, and delta specs for sponsored runtime resource grants.

## Phase 2: Core model and admission semantics

- [x] Add portable Rust-owned DTOs for principal references, resource grants, grant scopes, sponsor/beneficiary/provider/workload/service principal roles, node/plugin principal refs, resource limits, quota reservations, quota consumption, revocation refs, settlement references, and usage receipts. ✅ 1m 14s (2026-05-07T01:34:06Z → 2026-05-07T01:35:20Z; evidence: `evidence/sponsored-runtime-dtos.md`)
- [x] Add pure tests for bounded resources, validity windows, workload/provider scopes, settlement-reference opacity, redaction, and quota arithmetic. ✅ 1m 04s (2026-05-07T01:36:02Z → 2026-05-07T01:37:06Z; evidence: `evidence/sponsored-runtime-model-tests.md`)
- [x] Add fail-closed admission tests covering missing principal proof, expired grant, revoked grant, provider-principal rejection, unsupported settlement tag, quota exhaustion, isolation mismatch, and workload/service-principal scope mismatch. ✅ 1m 35s (2026-05-07T01:37:39Z → 2026-05-07T01:39:14Z; evidence: `evidence/sponsored-runtime-admission-tests.md`)

## Phase 3: Provider and sponsor policy contracts

- [x] Add Nickel-authored contracts and fixtures for provider offers, sponsor policies, resource class catalogs, and admission profiles. ✅ 4m 27s (2026-05-07T01:42:38Z → 2026-05-07T01:47:05Z; evidence: `evidence/sponsored-runtime-nickel-policy-contracts.md`)
- [x] Add positive/negative Nickel fixture tests proving policy defaults, bounded limits, secret-free settlement refs, and invalid provider/sponsor principal combinations are rejected before Rust runtime side effects. ✅ 49s (2026-05-07T01:47:44Z → 2026-05-07T01:48:33Z; evidence: `evidence/sponsored-runtime-nickel-fixtures.md`)

## Phase 4: Rust-derived evidence contracts

- [x] Register sponsored runtime grant, quota ledger, and usage receipt DTOs as `rust-derived` Nickel contract families. ✅ 2m 10s (2026-05-07T01:49:56Z → 2026-05-07T01:52:06Z; evidence: `evidence/sponsored-runtime-rust-derived-contract-registration.md`)
- [x] Add generated-contract freshness, Rust serialization round-trip, and Nickel validation tests for valid receipts plus malformed, missing, out-of-range, unknown-field, and secret-bearing receipt fixtures. ✅ 3m 32s (2026-05-07T01:53:16Z → 2026-05-07T01:56:48Z; evidence: `evidence/sponsored-runtime-generated-contract-validation.md`)

## Phase 5: Runtime integration and receipts

- [ ] Wire sponsored admission as an optional runtime-service/job/CI placement constraint that does not admit workloads without an accepted grant when sponsorship is required.
- [ ] Emit signed, redacted usage receipts for sponsored runtime execution start, reservation, consumption, completion, failure, and revocation-denial paths.

## Phase 6: Documentation and validation

- [ ] Document the sponsored execution boundary: Aspen enforces resource authority and receipts, while bilateral settlement stays external and currency-neutral.
- [ ] Run focused Rust/Nickel tests, strict OpenSpec validation, and whitespace checks.
