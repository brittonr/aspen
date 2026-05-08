## ADDED Requirements

### Requirement: Operator Receipt Redaction Hardening [r[dogfood-evidence.operator-redaction-hardening]]

Aspen MUST ensure operator-visible dogfood and runtime evidence receipt output remains useful while never displaying raw secret material.

#### Scenario: Secret markers are redacted from receipt rendering [r[dogfood-evidence.operator-redaction-hardening.rendering]]

- GIVEN a receipt contains fields or failure details with tokens, tickets, cookies, private keys, connection strings, or test secret markers
- WHEN receipt list, show, diagnosis, or summary rendering is produced
- THEN the rendered output SHALL NOT contain raw secret values
- AND it SHALL retain non-secret run ids, stage names, artifact identifiers, statuses, and bounded failure categories

#### Scenario: Protected references stay opaque [r[dogfood-evidence.operator-redaction-hardening.opaque-references]]

- GIVEN a receipt references protected owner-only files, capability handles, or cluster connection material
- WHEN the receipt is serialized or displayed
- THEN the output SHALL use opaque handles, content hashes, redacted summaries, or protected-path references rather than raw credentials

#### Scenario: Redaction failure blocks evidence publication [r[dogfood-evidence.operator-redaction-hardening.fail-closed]]

- GIVEN a receipt output path cannot prove that configured secret markers are absent
- WHEN evidence is prepared for operator-facing publication or archival
- THEN the preparation SHALL fail closed or leave the publication task incomplete rather than emitting unsafe evidence
