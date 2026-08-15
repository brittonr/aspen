## ADDED Requirements

### Requirement: Preserves safety checks inspect structure, not rendered text
r[molten.preserves_value_inspection.structural_scan] Molten MUST base semantic safety checks over Preserves structure rather than over pretty-printed text output.

#### Scenario: Rendered-looking string is not a structural marker
- GIVEN a Preserves string containing characters that look like a rendered sensitive record
- WHEN a structure-only marker check evaluates the value
- THEN the check does not treat that string as a sensitive record
- AND diagnostic rendering remains non-normative.

### Requirement: Sensitive marker detection is structural
r[molten.preserves_value_inspection.marker_detection] Molten MUST detect secret, confidential, credential, private, and encrypted-ref markers by structural record or symbol identity at safety boundaries.

#### Scenario: Nested sensitive marker denies
- GIVEN a service record containing a nested structural secret marker
- WHEN the service safety check inspects the record
- THEN the check detects the marker
- AND the boundary emits the configured deny or redaction decision.

### Requirement: Ambient job tokens are denied structurally
r[molten.preserves_value_inspection.ambient_token_denial] Molten MUST deny mobile-code, host-path, process-command, environment, source-text, and similar ambient tokens by structural identity before worker admission.

#### Scenario: Worker request contains host path record
- GIVEN a worker request containing a structural host-path token
- WHEN job worker admission validates the request
- THEN admission is `deny`
- AND no worker execution starts.

### Requirement: Ref-retention checks traverse Preserves values structurally
r[molten.preserves_value_inspection.ref_retention] Molten MUST locate retained content refs through structural traversal when cleanup, upgrade, or retention logic proves a ref is no longer referenced.

#### Scenario: Cleanup finds nested retained ref
- GIVEN a ledger artifact with a nested structural content ref to the cleanup target
- WHEN upgrade cleanup checks retained references
- THEN cleanup is `deny`
- AND diagnostics identify the retaining artifact and structural path.
