# Capture current-head dogfood acceptance receipt Delta

## ADDED Requirements

### Requirement: Current head dogfood receipt is durable
A current-head dogfood acceptance run MUST produce a durable schema-versioned receipt tied to the exact git commit and run id.
ID: r[dogfood-evidence.current-head-receipt-durable]

#### Scenario: Current head dogfood receipt is durable evidence
The current-head dogfood evidence scenario MUST record the produced receipt and redacted diagnostics.
ID: r[dogfood-evidence.current-head-receipt-durable.evidence]
- GIVEN `main` is clean and synced before the dogfood run
- WHEN the full dogfood loop completes or fails
- THEN the receipt SHALL record git commit, schema version, run id, stage outcomes, timings, artifact identifiers, failure category when any, and redacted diagnostics.

### Requirement: Receipt readback proves operator evidence
Dogfood receipt acceptance MUST prove that operators can inspect local receipt data and any cluster-published receipt without scraping logs.
ID: r[dogfood-evidence.receipt-readback-operator-evidence]

#### Scenario: Receipt readback proves operator evidence evidence
The receipt readback evidence scenario MUST include local receipt readback and diagnostics.
ID: r[dogfood-evidence.receipt-readback-operator-evidence.evidence]
- GIVEN a dogfood receipt is produced
- WHEN `receipts show`/`diagnose` or equivalent documented commands are run
- THEN the evidence SHALL include local receipt path, cluster KV key when published, readback result, and redaction confirmation for secrets.
