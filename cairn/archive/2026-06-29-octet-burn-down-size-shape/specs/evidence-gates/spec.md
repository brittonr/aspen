## ADDED Requirements

### Requirement: Remaining Octet size-shape warnings are reduced by functional decomposition
r[molten.octet_burndown.size_shape_finish] Molten MUST burn down remaining no-disabled file-length and function-length warnings through behavior-preserving decomposition, using pure deterministic cores for non-trivial logic and keeping public command and receipt behavior stable.

#### Scenario: Size-shape split preserves behavior
- GIVEN a long file or function is split to reduce size-shape warnings
- WHEN focused validation and no-disabled Octet evidence are produced
- THEN public command syntax, receipt labels, canonical Preserves values, and denial behavior remain stable
- AND the evidence records before/after movement for the touched size-shape warnings.

#### Scenario: Extracted logic is tested
- GIVEN a size-shape split extracts deterministic core logic
- WHEN the split is counted as burn-down progress
- THEN positive and negative tests cover the extracted logic without requiring external services or filesystem side effects.
