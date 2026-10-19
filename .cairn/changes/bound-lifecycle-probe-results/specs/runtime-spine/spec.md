# Lifecycle probe status-reference intake bound

## ADDED Requirements

### Requirement: Lifecycle probe validation bounds returned status references

r[molten.runtime_spine.lifecycle_probe_result_bound] Public lifecycle probe-result validation MUST reject a caller-supplied probe collection larger than the existing 1,024-reference lifecycle receipt ceiling before cloning any status-condition reference into the result. Collections of at most 1,024 probes MUST retain their existing evidence/generation validation, order, and optional status-reference behavior. This boundary does not grant restart authority or establish an accepted runtime-spine spec.

#### Scenario: Exact limit retains ordered status references
- GIVEN 1,024 valid lifecycle probe results, each with a status-condition reference
- WHEN a caller validates the probe collection
- THEN the returned collection contains all 1,024 status references in input order.

#### Scenario: One extra probe denies before collection growth
- GIVEN 1,025 caller-supplied lifecycle probe results
- WHEN a caller validates the probe collection
- THEN validation rejects the oversized collection before copying a status-condition reference.

#### Scenario: Admitted probes retain ordinary validation
- GIVEN at most 1,024 probes including an invalid evidence reference or future generation
- WHEN a caller validates the probe collection
- THEN ordinary probe validation still rejects the malformed admitted input.
