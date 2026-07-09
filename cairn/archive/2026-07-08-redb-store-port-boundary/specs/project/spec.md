# Project Delta: Redb Store Port Boundary

### Requirement: Local persistence uses explicit store ports
r[molten.modularity.store_ports.explicit_port] Repository-owned domain cores that require local indexes or durable metadata SHOULD express persistence needs through explicit store ports, deterministic plans, or typed query/result records rather than direct Redb access.

#### Scenario: Domain core returns store plan
- GIVEN a domain operation needs to read or update local persistent indexes
- WHEN the pure core evaluates admitted in-memory inputs
- THEN it returns a structured store query or mutation plan without opening Redb or beginning a transaction

#### Scenario: Direct Redb access is contained
- GIVEN a module imports Redb types or opens Redb transactions
- WHEN reviewers inspect the module after migration
- THEN the code is inside an approved store adapter or records a staged-migration exemption

### Requirement: Redb adapter owns database mechanics
r[molten.modularity.store_ports.redb_adapter] Redb table definitions, database open/create, transaction lifetimes, migration checks, and low-level Redb error mapping MUST be owned by the Redb adapter shell, not by pure domain cores.

#### Scenario: Adapter maps Redb result
- GIVEN a Redb read or write operation completes
- WHEN the adapter returns to the domain shell
- THEN the result is expressed as typed store data, canonical diagnostics, or structured adapter error

### Requirement: Admission precedes store writes
r[molten.modularity.store_ports.admission_before_write] Store mutation plans MUST be produced only after domain admission succeeds, and denied requests MUST NOT begin Redb write transactions or mutate local indexes.

#### Scenario: Denied mutation has empty plan
- GIVEN missing authority, stale evidence, malformed refs, resource denial, or unsupported store profile
- WHEN the domain planner evaluates the request
- THEN it returns a deny result with no write transaction or mutation plan

### Requirement: Store port extraction has positive and negative tests
r[molten.modularity.store_ports.tests] Store port refactors SHOULD include positive tests for admitted plans and negative tests for denied, malformed, stale, unavailable, or conflicting inputs.

#### Scenario: Store tests cover denial
- GIVEN a store port boundary is introduced
- WHEN reviewers inspect the tests
- THEN valid admitted inputs and denied inputs are both covered, including proof that denied inputs do not request writes
