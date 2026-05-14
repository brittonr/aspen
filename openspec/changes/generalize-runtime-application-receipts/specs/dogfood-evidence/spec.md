## ADDED Requirements

### Requirement: Runtime Application Receipt Contract [r[dogfood-evidence.runtime-application-receipt]]
Aspen MUST provide a generalized, secret-safe receipt contract for Aspen-started runtime applications, jobs, and service units without invalidating existing dogfood or CI receipt schemas.

#### Scenario: Receipt identifies runtime application attempt [r[dogfood-evidence.runtime-application-receipt.identity]]
- GIVEN Aspen starts, submits, reconciles, or records a runtime application, job, or service unit
- WHEN a runtime application receipt is serialized
- THEN the receipt SHALL include schema version, receipt id, source operation, unit or service identity, host kind when applicable, runner identity when applicable, artifact identity or provenance handles, lifecycle status, timestamps or durations, and parent run/job/service ids when available

#### Scenario: Receipt output is bounded and redacted [r[dogfood-evidence.runtime-application-receipt.secret-safe-output]]
- GIVEN the runtime application emits logs, artifacts, diagnostics, capability handles, or failure details
- WHEN receipt data is persisted or rendered
- THEN unbounded logs SHALL be replaced with bounded summaries or artifact handles
- AND raw tokens, tickets, private keys, cluster cookies, connection strings, and secret values SHALL NOT be serialized or displayed

#### Scenario: Receipt can be read back without log scraping [r[dogfood-evidence.runtime-application-receipt.readback]]
- GIVEN a runtime application receipt exists in a local receipt store, Raft-backed KV, job record, or service evidence store
- WHEN an operator uses the supported CLI or API readback path
- THEN Aspen SHALL validate the receipt schema before rendering it
- AND it SHALL show identity, status, timing, artifact/output handles, and redacted failure diagnostics without requiring chat logs or raw process logs

#### Scenario: Existing receipts remain compatible [r[dogfood-evidence.runtime-application-receipt.compatibility]]
- GIVEN existing dogfood or CI receipts use their current schema versions
- WHEN generalized runtime application receipt support is introduced
- THEN existing receipts SHALL remain valid under their schema-specific readers
- AND shared rendering or adapters SHALL preserve their non-secret identity, stage/job, status, timing, and artifact metadata
