## MODIFIED Requirements

### Requirement: Typed Nickel Contract Registry [r[typed-nickel-contracts.registry]]

Aspen MUST maintain a registry of typed Nickel contract families that identifies each contract's source of truth, owner, generated artifact path, validation command, and freshness gate, including sponsored runtime policy/evidence families when sponsored resource grants are implemented.

#### Scenario: Registry classifies sponsorship contract ownership [r[typed-nickel-contracts.registry.classifies-sponsorship-ownership]]

- GIVEN sponsored runtime contract families such as provider offers, sponsor policies, resource class catalogs, admission profiles, resource grants, quota ledgers, and usage receipts
- WHEN a maintainer inspects the registry
- THEN the registry MUST classify provider offers, sponsor policies, resource class catalogs, and admission profiles as `nickel-authored`
- AND it MUST classify runtime-emitted resource grant, quota ledger, and usage receipt DTO contracts as `rust-derived`
- AND it MUST mark raw secrets, payment credentials, payment rail internals, exchange rates, authorization proof verification, metering behavior, and Raft state transitions as non-candidates for Nickel ownership
