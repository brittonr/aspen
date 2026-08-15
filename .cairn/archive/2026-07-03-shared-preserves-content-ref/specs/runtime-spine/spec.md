## ADDED Requirements

### Requirement: Content refs have one canonical internal type
r[molten.preserves_content_ref.shared_newtype] Molten SHOULD use a shared validated content-ref newtype for canonical `blake3:<lowercase-hex>` Preserves content refs across migrated runtime and storage DTOs.

#### Scenario: Valid ref remains wire-compatible
- GIVEN a valid canonical content ref string
- WHEN the ref is parsed into the shared type and rendered back to Preserves or JSON
- THEN the rendered value is identical to the original string
- AND the typed value can be compared and ordered deterministically.

### Requirement: Invalid content refs cannot be represented
r[molten.preserves_content_ref.invalid_denials] Molten MUST reject invalid content refs before constructing the shared content-ref type.

#### Scenario: Uppercase ref denies
- GIVEN a content ref string containing uppercase hex characters
- WHEN a boundary parser constructs the shared content-ref type
- THEN parsing fails
- AND no downstream DTO receives a typed content ref.

### Requirement: Runtime envelopes use the shared content-ref type
r[molten.preserves_content_ref.runtime_envelope] Molten MUST use the shared content-ref type for runtime envelope blob refs while preserving existing Preserves and JSON wire formats.

#### Scenario: Envelope hash remains stable
- GIVEN an existing valid runtime envelope fixture
- WHEN its blob refs are represented with the shared content-ref type
- THEN its canonical Preserves hash remains unchanged
- AND invalid blob refs still deny before envelope admission.

### Requirement: DTO ref migrations preserve public formats
r[molten.preserves_content_ref.dto_migration] Molten SHOULD migrate parsed DTO fields that represent canonical content refs from raw strings to the shared content-ref type without changing CLI syntax or Preserves record layout.

#### Scenario: Typed DTO rejects bad ref early
- GIVEN a parsed storage, artifact, job, eval-cache, catalog, or schema DTO with a malformed ref field
- WHEN the DTO parser runs
- THEN parsing fails before semantic admission
- AND public error diagnostics identify the ref field.
