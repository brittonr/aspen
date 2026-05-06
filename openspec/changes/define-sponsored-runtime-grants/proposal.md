## Why

Aspen's runtime roadmap needs a neutral way for one party to authorize another party's hosted service, job, CI run, or application workload without baking in a currency, payment rail, or marketplace. The runtime should enforce resource authority and emit receipts; bilateral settlement should remain outside Aspen and up to the parties.

This change defines sponsored runtime resource grants as the contract between workload owners, sponsors, and providers. It also records where Nickel typing and contracts fit: human-authored policies and provider offers can be Nickel-authored, while runtime-emitted grants, ledgers, and usage receipts remain Rust-owned DTOs with generated Nickel validation contracts.

## What Changes

- **Sponsored resource grants**: Define a runtime sponsorship model for delegated resource authority across sponsor, beneficiary/workload owner, and provider/operator roles.
- **Currency-neutral settlement references**: Require settlement/payment references to be opaque metadata that Aspen stores and redacts but does not interpret.
- **Admission and quota semantics**: Require runtime admission checks against grant scope, provider acceptance policy, resource limits, validity windows, revocation, and remaining quota.
- **Usage receipts**: Require signed, redacted usage receipts that can support external settlement or audit without making Aspen a payment processor.
- **Nickel contract boundary**: Extend the typed Nickel contract plan with sponsorship policy/offer configs as Nickel-authored and resource grant/usage receipt DTOs as Rust-derived.

## In Scope

- Specification of sponsor, beneficiary, provider, workload, resource grant, quota ledger, provider offer, and usage receipt concepts.
- Runtime admission invariants for sponsored workloads.
- Currency-neutral settlement metadata rules.
- Nickel ownership split for human policy/config versus Rust-owned runtime evidence DTOs.
- Future verification tasks for positive/negative Nickel fixtures and Rust schema/round-trip tests.

## Out of Scope

- Built-in fiat, crypto, credit, escrow, tax, dispute, or exchange-rate logic.
- A marketplace UI or global pricing model.
- Live billing integrations.
- Provider reputation scoring.
- Full runtime service implementation; this change depends on the runtime service core but does not implement it.

## Verification

- `openspec validate define-sponsored-runtime-grants --strict`
- Future Rust unit tests for grant admission, quota accounting, revocation, receipt redaction, and settlement opacity.
- Future Nickel positive/negative fixtures for provider offer, sponsor policy, and admission profile configs.
- Future generated-contract freshness checks for Rust-owned grant, ledger, and usage receipt DTOs.
- `git diff --check`
