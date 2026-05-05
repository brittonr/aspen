## MODIFIED Requirements

### Requirement: Native CI Run Receipts [r[dogfood-evidence.ci.run-receipt]]

Aspen MUST expose a native CI run receipt that operators can query by pipeline run ID without using the dogfood receipt wrapper, and the canonical Rust receipt type MUST generate a typed Nickel contract that validates exported receipt JSON.

#### Scenario: Operator queries a CI run receipt [r[dogfood-evidence.ci.run-receipt.query]]

- GIVEN Aspen has a persisted CI pipeline run in Raft-backed KV
- WHEN an operator requests the CI run receipt by run ID
- THEN Aspen MUST return a schema-versioned receipt with schema `aspen.ci.run-receipt.v1`
- AND the receipt MUST include run ID, repository ID, ref, commit hash, pipeline name, status, created/completed timestamps, stages, and jobs
- AND jobs MUST include their job IDs when available so operators can use log/output commands as follow-up handles

#### Scenario: Receipt has a generated Nickel contract [r[dogfood-evidence.ci.run-receipt.generated-nickel-contract]]

- GIVEN the Rust type that owns native CI run receipt serialization changes
- WHEN the receipt contract freshness check runs
- THEN the generated Nickel contract for `aspen.ci.run-receipt.v1` MUST be updated or the check MUST fail

### Requirement: Dogfood Run Receipt Schema [r[dogfood-evidence.receipt-schema]]

The dogfood orchestrator MUST define a versioned run receipt schema that records run identity, command identity, selected mode flags, project directory, cluster directory, and ordered stage receipts, and the Rust receipt type that owns canonical serialization MUST generate a typed Nickel contract for validating saved and published receipt JSON.

#### Scenario: Receipt includes run identity [r[dogfood-evidence.receipt-schema.run-identity]]

- **GIVEN** a dogfood run receipt
- **WHEN** it is serialized
- **THEN** the JSON includes a schema name, run id, command, created timestamp, mode flags, project directory, cluster directory, and stages array

#### Scenario: Generated contract validates local and published receipts [r[dogfood-evidence.receipt-schema.generated-nickel-contract]]

- GIVEN a dogfood receipt is saved locally or published into Aspen KV
- WHEN the generated Nickel contract validates the receipt JSON
- THEN valid receipts MUST pass and malformed, stale-schema, missing-stage, unbounded-artifact, or invalid-status receipts MUST fail
