## ADDED Requirements

### Requirement: Service Hash Identity [r[runtime-service-core.service-hash]]

Aspen MUST model deployed service identity as an immutable `ServiceHash` that refers to a validated service, artifact, or execution-closure manifest rather than to a mutable human name.

#### Scenario: Service hash identifies immutable target [r[runtime-service-core.service-hash.immutable-target]]

- GIVEN a runtime service deployment is admitted
- WHEN Aspen records its deployed identity
- THEN the identity MUST include a `ServiceHash` derived from immutable validated content
- AND later name changes MUST NOT alter that hash

### Requirement: Service Name Registry [r[runtime-service-core.service-name-registry]]

Aspen MUST provide a Raft-backed registry mapping stable `ServiceName` values to immutable `ServiceHash` targets with generation metadata.

#### Scenario: Assign service name [r[runtime-service-core.service-name-registry.assign]]

- GIVEN an operator or deploy action has authority to assign a service name
- WHEN it assigns `ServiceName` to a valid `ServiceHash`
- THEN the mapping MUST be committed through Raft with a monotonically increasing generation
- AND a receipt MUST record the name, next hash, generation, and redacted authorization summary

#### Scenario: Update service name [r[runtime-service-core.service-name-registry.update]]

- GIVEN a service name already points to a prior service hash
- WHEN an authorized update points the name to a new valid service hash
- THEN the registry MUST preserve previous-hash evidence in the update receipt
- AND lookup after commit MUST resolve to the new hash and generation

#### Scenario: Roll back service name [r[runtime-service-core.service-name-registry.rollback]]

- GIVEN a previous service hash remains available and valid
- WHEN an authorized rollback reassigns the service name to that hash
- THEN the registry MUST treat the rollback as a new generation pointing to the prior hash
- AND the receipt MUST distinguish rollback from initial assignment

### Requirement: Service Name Lookup [r[runtime-service-core.service-name-lookup]]

Aspen MUST expose readback that distinguishes mutable names from immutable resolved hashes.

#### Scenario: Lookup returns name and hash [r[runtime-service-core.service-name-lookup.name-and-hash]]

- GIVEN a service name exists in the registry
- WHEN an operator or runtime component resolves it
- THEN the response MUST include service name, resolved service hash, generation, and update timestamp or log index

#### Scenario: Missing name is explicit [r[runtime-service-core.service-name-lookup.missing]]

- GIVEN no mapping exists for a requested service name
- WHEN lookup runs
- THEN Aspen MUST return a typed not-found result rather than falling back to an implicit latest deployment

### Requirement: Service Registry Authorization and Redaction [r[runtime-service-core.service-name-registry.auth-redaction]]

Aspen MUST authorize service-name mutations and keep registry receipts secret-safe.

#### Scenario: Unauthorized mutation rejected [r[runtime-service-core.service-name-registry.unauthorized]]

- GIVEN a caller lacks permission to assign, update, or roll back a service name
- WHEN it attempts the mutation
- THEN Aspen MUST reject the mutation before changing Raft state

#### Scenario: Receipts redact credentials [r[runtime-service-core.service-name-registry.redacted-receipt]]

- GIVEN a service-name mutation used tickets, tokens, cookies, keys, or capability proofs
- WHEN the receipt is emitted
- THEN it MUST include only opaque handles, hashes, or redacted summaries and MUST NOT include raw secret material
