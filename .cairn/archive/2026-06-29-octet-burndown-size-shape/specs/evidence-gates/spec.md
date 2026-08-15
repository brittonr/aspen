## ADDED Requirements

### Requirement: Octet size-shape burn-down is isolated
r[molten.octet_burndown.size_shape] Molten MUST track `excessive_file_length` and `function_length` burn-down as a dedicated Cairn change category, using functional-core / imperative-shell decomposition for non-trivial logic and preserving public behavior until refreshed no-disabled evidence proves the category is clean or explicitly scoped.

#### Scenario: Size-shape slice preserves behavior
- GIVEN a refactor whose primary purpose is reducing file-length or function-length warnings
- WHEN focused validation and the no-disabled Octet probe run
- THEN the evidence records before/after size-shape warning counts
- AND public command syntax, receipt labels, canonical output values, and fail-closed behavior remain stable

#### Scenario: Logic extraction is tested
- GIVEN a size-shape slice extracts or changes deterministic core logic
- WHEN the slice is counted as burn-down progress
- THEN positive and negative tests cover the changed behavior before the task is marked complete
