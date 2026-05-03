## ADDED Requirements

### Requirement: Dogfood Receipt Cluster Publication [r[dogfood-evidence.receipt-cluster-publication]]

The dogfood orchestrator MUST support publishing a validated local dogfood run receipt into the running Aspen cluster so the cluster can carry its own acceptance evidence while it is alive.

#### Scenario: Publish receipt to cluster KV [r[dogfood-evidence.receipt-cluster-publication.publish]]

- **GIVEN** a running dogfood cluster and a valid local dogfood receipt selected by run id or explicit path
- **WHEN** an operator runs `aspen-dogfood receipts publish <run-id-or-path>`
- **THEN** the command writes the canonical receipt JSON to the cluster KV store under `dogfood/receipts/<run-id>.json`
- **AND** the command prints the run id and cluster key without printing credential material

#### Scenario: Reject invalid local receipt before publishing [r[dogfood-evidence.receipt-cluster-publication.reject-invalid]]

- **GIVEN** the selected local receipt is missing, malformed, or uses an unexpected schema
- **WHEN** an operator runs `aspen-dogfood receipts publish <run-id-or-path>`
- **THEN** the command fails before writing to cluster KV

### Requirement: Dogfood Receipt Cluster Show [r[dogfood-evidence.receipt-cluster-show]]

The dogfood orchestrator MUST support loading one published dogfood receipt from the running Aspen cluster and MUST validate it before rendering it to an operator.

#### Scenario: Show published receipt by run id [r[dogfood-evidence.receipt-cluster-show.run-id]]

- **GIVEN** a running dogfood cluster has a value at `dogfood/receipts/<run-id>.json`
- **WHEN** an operator runs `aspen-dogfood receipts cluster-show <run-id>`
- **THEN** the command validates the stored value as a dogfood receipt and prints the same human-readable receipt summary used for local receipt files

#### Scenario: Show published receipt as JSON [r[dogfood-evidence.receipt-cluster-show.json]]

- **GIVEN** a running dogfood cluster has a valid published receipt value
- **WHEN** an operator runs `aspen-dogfood receipts cluster-show <run-id> --json`
- **THEN** the command emits validated canonical JSON for that receipt

#### Scenario: Reject missing or invalid cluster value [r[dogfood-evidence.receipt-cluster-show.invalid]]

- **GIVEN** the running dogfood cluster has no value or an invalid value at the selected receipt key
- **WHEN** an operator runs `aspen-dogfood receipts cluster-show <run-id>`
- **THEN** the command fails with a receipt error rather than printing unvalidated evidence
