## ADDED Requirements

### Requirement: Currency-Neutral Resource Sponsorship [r[runtime-sponsorship.currency-neutral-resource-sponsorship]]

Aspen MUST model third-party sponsored hosting and execution as delegated resource authority rather than as a currency, payment processor, exchange-rate engine, or marketplace ledger.

#### Scenario: Non-monetary grant is valid [r[runtime-sponsorship.currency-neutral-resource-sponsorship.non-monetary-grant]]

- GIVEN a sponsor authorizes a beneficiary to run a workload using a bounded resource grant whose settlement reference is `none:internal-grant`
- WHEN the runtime evaluates sponsored admission
- THEN Aspen MUST treat the grant as eligible if capability proof, provider policy, quota, scope, revocation, and validity checks pass
- AND Aspen MUST NOT require a fiat, crypto, credit, escrow, or token payment reference

#### Scenario: Payment-specific data remains opaque [r[runtime-sponsorship.currency-neutral-resource-sponsorship.opaque-payment-data]]

- GIVEN a resource grant includes an opaque settlement reference such as an invoice id, transaction id, purchase order, voucher, offline contract hash, or private bilateral reference
- WHEN Aspen stores, validates, logs, or emits receipts for the grant
- THEN Aspen MUST preserve only bounded opaque metadata needed for policy/audit
- AND Aspen MUST NOT dereference the payment rail, validate balances, compute exchange rates, or embed raw payment credentials

### Requirement: Sponsored Runtime Grant Model [r[runtime-sponsorship.grant-model]]

Aspen MUST define Rust-owned sponsored runtime grant DTOs for sponsor identity, beneficiary identity, workload scope, provider scope, resource limits, validity window, revocation reference, settlement reference, and policy tags.

#### Scenario: Grant scope constrains a workload [r[runtime-sponsorship.grant-model.scope-constrains-workload]]

- GIVEN a workload presents a sponsorship grant
- WHEN the workload owner, workload identity, requested resources, provider, host/isolation class, or requested time window falls outside the grant scope
- THEN admission MUST fail closed before runtime side effects occur

#### Scenario: Revoked or expired grant is denied [r[runtime-sponsorship.grant-model.revoked-or-expired]]

- GIVEN a grant is expired or its revocation reference resolves as revoked
- WHEN a workload attempts to reserve or consume sponsored resources
- THEN Aspen MUST deny the reservation and emit only a redacted denial receipt when receipt emission is enabled

### Requirement: Provider Acceptance Policy [r[runtime-sponsorship.provider-acceptance-policy]]

Aspen MUST let providers/operators declare local acceptance policy for sponsor identities, grant issuers, workload classes, resource classes, isolation modes, settlement-reference method tags, maximum exposure, and receipt/attestation requirements.

#### Scenario: Provider rejects unsupported sponsor or settlement tag [r[runtime-sponsorship.provider-acceptance-policy.rejects-unsupported]]

- GIVEN a valid grant from the sponsor's perspective
- WHEN the selected provider policy does not accept the sponsor, issuer, workload class, settlement-reference tag, isolation mode, or exposure level
- THEN Aspen MUST reject sponsored placement on that provider without trying another provider unless the scheduler has an explicitly allowed candidate set

#### Scenario: Provider offer stays policy, not price law [r[runtime-sponsorship.provider-acceptance-policy.offer-is-policy]]

- GIVEN a provider publishes a provider offer or resource class catalog
- WHEN Aspen validates the offer
- THEN the offer MUST describe accepted classes, bounds, isolation, evidence requirements, and opaque settlement tags
- AND it MUST NOT define a global price, currency conversion rule, or settlement obligation for all Aspen providers

### Requirement: Sponsored Quota Ledger [r[runtime-sponsorship.quota-ledger]]

Aspen MUST account sponsored reservations and consumption against grant limits using durable fail-closed quota ledger semantics before admitting sponsored runtime work.

#### Scenario: Reservation prevents overrun [r[runtime-sponsorship.quota-ledger.reservation-prevents-overrun]]

- GIVEN a grant has finite CPU, memory-time, storage-time, network, concurrency, or duration limits
- WHEN a workload requests a reservation that would exceed remaining quota or maximum concurrency
- THEN Aspen MUST deny admission before execution starts

#### Scenario: Consumption emits auditable state [r[runtime-sponsorship.quota-ledger.consumption-auditable]]

- GIVEN a sponsored execution reserves and consumes resources
- WHEN the execution completes, fails, or is stopped
- THEN Aspen MUST record enough quota state to distinguish reserved, consumed, released, and denied resources for later audit

### Requirement: Sponsored Usage Receipts [r[runtime-sponsorship.usage-receipts]]

Aspen MUST emit Rust-owned, signed, redacted usage receipts for sponsored execution evidence when sponsorship is used for runtime admission or quota accounting.

#### Scenario: Receipt supports external settlement without processing payment [r[runtime-sponsorship.usage-receipts.external-settlement]]

- GIVEN a sponsored workload starts, reserves resources, consumes resources, completes, fails, or is denied by revocation/quota/policy
- WHEN Aspen emits a usage receipt
- THEN the receipt MUST include schema version, execution id, workload id, provider id, sponsor/grant reference, bounded resource measurements, timestamps, outcome, artifact/log refs when available, isolation/attestation summary when available, and redacted settlement metadata
- AND the receipt MUST NOT claim Aspen processed payment or guarantee external settlement

#### Scenario: Receipt redacts secrets [r[runtime-sponsorship.usage-receipts.redacts-secrets]]

- GIVEN grant, provider, workload, or settlement metadata references tokens, keys, private URLs, cluster cookies, capability proofs, or payment credentials
- WHEN Aspen serializes a usage receipt
- THEN the receipt MUST contain only redacted handles, hashes, tags, or bounded opaque refs
- AND it MUST NOT include raw bearer secrets or raw payment credentials

### Requirement: Nickel Contract Boundary for Sponsorship [r[runtime-sponsorship.nickel-contract-boundary]]

Aspen MUST use Nickel contracts for declarative sponsorship policy/config and generated Nickel validation contracts for Rust-owned sponsorship evidence without making Nickel the owner of runtime authorization, metering, or distributed state transitions.

#### Scenario: Human-authored policies are Nickel-authored [r[runtime-sponsorship.nickel-contract-boundary.policy-configs]]

- GIVEN a provider offer, sponsor policy template, resource class catalog, or admission profile is maintained by an operator
- WHEN the config is consumed by Rust
- THEN Nickel typecheck and contract validation MUST run before Rust uses the exported data for runtime side effects

#### Scenario: Runtime evidence is Rust-derived [r[runtime-sponsorship.nickel-contract-boundary.rust-derived-evidence]]

- GIVEN a resource grant DTO, quota ledger DTO, or usage receipt DTO is serialized by Aspen runtime code
- WHEN Nickel validation is needed for evidence review or fixture testing
- THEN the Nickel contract MUST be generated from the Rust-owned schema and freshness-checked against the Rust source

#### Scenario: Nickel does not own runtime behavior [r[runtime-sponsorship.nickel-contract-boundary.runtime-behavior-stays-rust]]

- GIVEN a proposed Nickel config attempts to encode authorization proof verification, quota mutation ordering, revocation resolution, scheduler placement behavior, metering internals, or Raft state transitions
- WHEN the sponsorship contract boundary is reviewed
- THEN the proposal MUST be rejected or rewritten so Nickel validates data shape/configuration/evidence and Rust retains behavior ownership
