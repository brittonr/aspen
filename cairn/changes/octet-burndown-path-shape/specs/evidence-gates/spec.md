## ADDED Requirements

### Requirement: Octet path-shape burn-down is isolated
r[molten.octet_burndown.path_shape] Molten MUST track `path_segment_repetition` burn-down as a dedicated Cairn change category, preserving public Rust paths, command behavior, receipt schemas, canonical Preserves values, and source-gate caveats until refreshed no-disabled evidence proves the category is clean or explicitly scoped.

#### Scenario: Path-shape slice preserves behavior
- GIVEN a refactor whose primary purpose is reducing `path_segment_repetition`
- WHEN focused validation and the no-disabled Octet probe run
- THEN the evidence records the before/after path-shape warning count
- AND public command syntax, receipt labels, and canonical output values remain stable

#### Scenario: Path-shape caveat remains visible until clean
- GIVEN `path_segment_repetition` remains disabled or warning-only in no-disabled evidence
- WHEN Octet remediation evidence is reported
- THEN Molten labels path shape as an active burn-down category rather than source-remediated zero
