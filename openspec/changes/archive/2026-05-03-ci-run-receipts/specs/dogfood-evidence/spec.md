## ADDED Requirements

### Requirement: Native CI Run Receipts [r[dogfood-evidence.ci.run-receipt]]

Aspen MUST expose a native CI run receipt that operators can query by pipeline run ID without using the dogfood receipt wrapper.

#### Scenario: Operator queries a CI run receipt [r[dogfood-evidence.ci.run-receipt.query]]

- GIVEN Aspen has a persisted CI pipeline run in Raft-backed KV
- WHEN an operator requests the CI run receipt by run ID
- THEN Aspen MUST return a schema-versioned receipt with schema `aspen.ci.run-receipt.v1`
- AND the receipt MUST include run ID, repository ID, ref, commit hash, pipeline name, status, created/completed timestamps, stages, and jobs
- AND jobs MUST include their job IDs when available so operators can use log/output commands as follow-up handles

#### Scenario: Missing CI run receipt is explicit [r[dogfood-evidence.ci.run-receipt.missing]]

- GIVEN Aspen has no CI pipeline run for the requested run ID
- WHEN an operator requests the CI run receipt
- THEN the command MUST fail explicitly rather than printing fabricated evidence

#### Scenario: Receipt output is deterministic [r[dogfood-evidence.ci.run-receipt.deterministic]]

- GIVEN a CI run contains multiple jobs in a stage
- WHEN Aspen renders the CI run receipt
- THEN jobs MUST be ordered deterministically by job name
- AND JSON output MUST be machine parseable without log prefixes on stdout
