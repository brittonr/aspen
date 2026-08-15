# Artifact Auth Operational Receipt Specification

## Purpose

Defines the `artifact-auth-operational-receipt` capability.

## Requirements

### Requirement: Canonical operational receipt identity
r[molten.artifact_auth_operational_receipt.identity] Molten MUST construct a bounded deterministic operational receipt that binds the exact standalone carrier, file-adapter handle generation and purpose, currentness evidence, shell outcome, authority flags, non-claims, and BLAKE3 receipt identity.

#### Scenario: Passing shell evidence maps deterministically
GIVEN exact standalone verification through the production file adapter
WHEN Molten constructs the operational receipt
THEN repeated construction yields identical receipt bytes and BLAKE3 identity

#### Scenario: Receipt drift is rejected
GIVEN changed carrier, handle, outcome, authority flag, non-claim, or self-hash material
WHEN Molten validates the receipt
THEN validation fails closed

### Requirement: Capability-rooted persistence
r[molten.artifact_auth_operational_receipt.persistence] Molten MUST write and read operational receipts only through an explicit `Receipts` node-state namespace and a validated relative locator, and MUST reject wrong namespaces, non-regular leaves, oversized content, and malformed encodings.

#### Scenario: Receipt survives node-state reopen
GIVEN a passing receipt written through the receipts namespace
WHEN the node-state root and namespace are reopened
THEN Molten reads the same validated receipt identity

#### Scenario: Wrong namespace is denied
GIVEN a secrets, identity, ledger, or other namespace
WHEN receipt publication is requested
THEN publication fails before writing

### Requirement: Adapter-state replay
r[molten.artifact_auth_operational_receipt.replay] Molten MUST re-derive signer generation and currentness from the reopened capability-file adapter before replaying exact standalone verification, and MUST reject rotation, revocation, missing state, carrier drift, or outcome drift.

#### Scenario: Current key replay passes
GIVEN unchanged persisted key and receipt state
WHEN Molten reopens state and replays the receipt
THEN exact standalone evidence matches while legacy authority remains active

#### Scenario: Rotation or revocation blocks replay
GIVEN a receipt signed by a retired or revoked handle
WHEN Molten reopens state and replays it
THEN replay fails closed before standalone evidence is admitted

### Requirement: Runtime authority remains unchanged
r[molten.artifact_auth_operational_receipt.authority] Molten MUST keep `legacy_authoritative = true`, `standalone_authority_admitted = false`, and `rollback_available = true`; receipt replay MUST NOT grant membership, capability, federation, transport, storage, lifecycle, signing-policy, or release authority.

#### Scenario: Passing receipt remains observational
GIVEN successful persisted replay
WHEN the report is consumed
THEN Molten's existing verification and admission decisions remain authoritative
