# Testing Harness Delta: redacted repro export profiles

### Requirement: Export profiles are explicit
r[molten.testing.redacted_repro_export_profiles.profile_schema] Repro export MUST require an explicit confidentiality profile whenever sensitive markers are present.

#### Scenario: Default profile remains fail-closed
- GIVEN a report containing `<secret ...>`
- WHEN repro export uses the default profile
- THEN export fails closed before writing a sealed pass bundle

#### Scenario: Redacted diagnostic profile emits transform evidence
- GIVEN a report containing sensitive markers
- WHEN repro export uses `redacted-diagnostic`
- THEN the output bundle contains deterministic redaction markers
- AND the bundle contains a redaction transform receipt bound to the source report and output bundle
- AND the bundle is marked diagnostic-only unless policy says otherwise

### Requirement: Transform receipts bind all redactions
r[molten.testing.redacted_repro_export_profiles.transform_receipt] Redaction transform receipts MUST bind the source report ref, suite ref, redaction policy ref, profile, transform manifest, and output bundle ref.

#### Scenario: Stale transform receipt is rejected
- GIVEN a redacted bundle with a transform receipt from another report
- WHEN verify, unpack, or gate checks run
- THEN the bundle fails closed with a transform binding diagnostic

#### Scenario: Missed sensitive marker is rejected
- GIVEN a redacted bundle whose transform manifest does not cover every sensitive marker
- WHEN verify, unpack, or gate checks run
- THEN the bundle fails closed before materializing private content

### Requirement: Encrypted refs require validation and reveal receipts
r[molten.testing.redacted_repro_export_profiles.encrypted_ref_validation] `<encrypted-ref ...>` values MUST remain fail-closed unless encryption metadata, recipient policy, and reveal receipts validate.

#### Scenario: Malformed encrypted ref is rejected
- GIVEN a redacted or encrypted bundle containing a malformed `<encrypted-ref ...>`
- WHEN verify or unpack runs
- THEN the bundle fails closed

#### Scenario: Authorized reveal materializes private content
- GIVEN an encrypted-private bundle and a matching reveal receipt
- WHEN unpack runs with reveal authority
- THEN only authorized private material is materialized
- AND the reveal receipt is written beside the unpacked bundle evidence
