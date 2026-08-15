## ADDED Requirements

### Requirement: Remaining Octet path-shape warnings are reduced without public rename churn
r[molten.octet_burndown.path_shape_finish] Molten MUST burn down remaining no-disabled `path_segment_repetition` warnings through private path-shape cleanup while preserving public Rust paths, CLI syntax, receipt schemas, canonical labels, and fail-closed behavior unless a separate compatibility change admits a public rename.

#### Scenario: Private path-shape cleanup preserves public behavior
- GIVEN a path-repetition hotspot is refactored with private aliases, helper renames, or module ownership changes
- WHEN focused validation and no-disabled Octet evidence run
- THEN public command syntax, receipt labels, and canonical Preserves values remain stable
- AND before/after path-shape counts are recorded.

#### Scenario: Public repetition is documented instead of hidden
- GIVEN a repeated path segment is part of a public command, schema, receipt label, or compatibility boundary
- WHEN remediation evidence is reported
- THEN Molten documents the preserved repetition as an active or explicitly scoped caveat instead of hiding it with a suppression.
