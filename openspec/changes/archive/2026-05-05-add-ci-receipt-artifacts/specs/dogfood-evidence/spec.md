## ADDED Requirements

### Requirement: CI Run Receipts Include Artifact Metadata [r[dogfood-evidence.ci.run-receipt.artifacts]]

Native CI run receipts MUST include operator-safe artifact metadata for jobs that produced artifacts.

#### Scenario: Job artifact metadata appears in receipt [r[dogfood-evidence.ci.run-receipt.artifacts.present]]

- GIVEN Aspen has a persisted CI pipeline run with a job ID
- AND CI artifact metadata exists under the job artifact KV prefix
- WHEN an operator requests the CI run receipt by run ID
- THEN the receipt job entry MUST include the artifact name, blob hash, size, content type, creation timestamp, and metadata
- AND the receipt MUST NOT include blob download tickets or credential material

#### Scenario: Artifact metadata is deterministic and bounded [r[dogfood-evidence.ci.run-receipt.artifacts.deterministic]]

- GIVEN a CI job has multiple artifact metadata records
- WHEN Aspen renders the CI run receipt
- THEN artifact entries MUST be ordered deterministically
- AND artifact scanning MUST use a bounded per-job limit

#### Scenario: Artifact scan failures are explicit [r[dogfood-evidence.ci.run-receipt.artifacts.scan-failure]]

- GIVEN Aspen cannot scan artifact metadata for a job while constructing a CI run receipt
- WHEN an operator requests the CI run receipt
- THEN the request MUST fail explicitly rather than returning a partial receipt that appears complete
