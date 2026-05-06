## Context

Aspen already has auth/capability primitives, jobs/CI, Forge, runtime host-loading OpenSpecs, active runtime service core work, dogfood/CI receipts, and a typed Nickel contract baseline. Those pieces are close to what sponsored execution needs, but the product concept needs an explicit boundary: Aspen should verify delegated resource authority and record metered usage, not decide how two parties settled value.

The model should support organization budgets, sponsored open-source CI, customer-funded hosted services, peer-to-peer compute favors, and future marketplace adapters without committing Aspen core to any one payment rail.

## Goals

- Keep Aspen currency- and settlement-neutral.
- Represent sponsored hosting/execution as capability-backed resource authority.
- Let providers accept or reject sponsors, workload classes, settlement references, and isolation modes under local policy.
- Emit signed, redacted usage receipts for external audit/settlement.
- Use Nickel where it is strongest: typed human-authored provider/sponsor policies, resource class catalogs, and validation fixtures.
- Keep Rust as the source of truth for runtime-emitted grants, ledger updates, revocation state, and receipts.

## Non-Goals

- Do not add a payment processor, token, exchange-rate engine, or global marketplace to Aspen core.
- Do not encode bilateral business terms beyond opaque settlement references and policy tags.
- Do not let Nickel own authorization, cryptographic verification, metering behavior, or distributed state transitions.
- Do not block non-monetary sponsorship such as internal grants, favors, test allocations, or research credits.

## Decisions

### 1. Sponsorship is resource authority, not money

**Choice:** Aspen models `ResourceGrant` authority from a sponsor to a beneficiary/workload scope with limits, validity, provider scope, revocation, and opaque settlement metadata.

**Rationale:** This gives the scheduler and runtime a concrete object to enforce while keeping settlement entirely outside Aspen.

### 2. Providers keep local acceptance policy

**Choice:** Providers/operators declare what sponsor identities, grant issuers, workload classes, isolation modes, resource classes, settlement-reference families, and maximum exposure they accept.

**Rationale:** A provider may accept invoices, UCAN vouchers, prepaid accounts, internal org budgets, or no external settlement at all. Aspen should not define a universal trust or pricing model.

### 3. Admission is fail-closed

**Choice:** A sponsored workload is admitted only when the proof chain, provider policy, workload scope, resource request, isolation requirement, validity window, revocation status, and remaining quota all pass.

**Rationale:** Sponsored execution creates third-party cost exposure; failure must deny execution rather than best-effort run.

### 4. Receipts are Rust-owned and Nickel-validated

**Choice:** Runtime usage receipts are Rust-owned serialized DTOs with generated Nickel contracts for validation. Receipts include identifiers, resource measurements, grant references, outcome, artifact/log refs, isolation/attestation summaries, and redacted settlement references, but never raw credentials or secrets.

**Rationale:** Rust owns runtime behavior and canonical serialization. Nickel gives operators a typed way to validate evidence artifacts without becoming the runtime source of truth.

### 5. Human-authored policy uses Nickel

**Choice:** Provider offers, sponsor policy templates, resource class catalogs, and admission profile fixtures are Nickel-authored configs exported to Rust after typecheck and contract validation.

**Rationale:** These are declarative, reviewed by humans, need defaults/docs/overlays, and fit Nickel's merge system and contracts.

### 6. Settlement references are opaque and redacted

**Choice:** `settlement_reference` values are typed only as bounded opaque references plus optional method tags; Aspen MUST NOT dereference, validate balances, compute exchange rates, or embed raw payment credentials.

**Rationale:** The parties own settlement. Aspen only needs stable references for audit and policy matching.

## Risks / Trade-offs

- **Payment-scope creep**: Mitigate with explicit non-goals and opaque settlement-reference requirements.
- **Receipt leakage**: Mitigate with redaction tests and generated Nickel negative fixtures.
- **Policy/source-of-truth drift**: Mitigate with a contract registry that classifies sponsorship families as `nickel-authored` or `rust-derived`.
- **Double-spend or overrun of quota**: Mitigate with Raft-backed reservation/consumption updates and fail-closed admission checks in future implementation.
- **Provider fraud or weak evidence**: Mitigate with signed receipts, artifact hashes, optional host attestations, and audit hooks; do not make receipts a guarantee of external settlement.

## Validation Plan

1. Validate the OpenSpec package strictly.
2. Add pure model tests for grant scopes, bounded resources, validity windows, settlement opacity, and redaction.
3. Add admission tests for missing proof, expired/revoked grant, provider rejection, quota exhaustion, and isolation mismatch.
4. Add Nickel positive/negative fixtures for provider offers, sponsor policies, and resource class catalogs.
5. Add generated Nickel contract freshness and round-trip tests for Rust-owned grant, quota ledger, and usage receipt DTOs.
