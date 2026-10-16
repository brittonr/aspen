# Runtime spine: caveat attenuation boundary delta

## ADDED Requirements

### Requirement: Caveat attenuation follows one recorded authority decision
r[molten.runtime_spine.caveat_authority_boundary] Molten MUST NOT add a Molten-owned caveat, rewrite, or payload-filter authority format until a recorded decision names whether the stack's UCAN and Basalt authority model already covers the reviewed attenuation needs, and any admitted form MUST reject unknown caveats, MUST use one documented evaluation order, and MUST NOT widen a grant.

#### Scenario: No second authority format without a decision
- GIVEN a proposed attenuation filter with no recorded boundary decision
- WHEN the proposal reaches review
- THEN it is rejected or returned for the decision note before any implementation lands.

#### Scenario: Unknown caveat rejects
- GIVEN an admitted caveat chain that contains a caveat the evaluator does not recognize
- WHEN the chain is evaluated against a payload
- THEN the payload is rejected and nothing is delivered.

#### Scenario: Attenuation cannot widen
- GIVEN a narrowed capability that carries an attenuation chain
- WHEN the holder attempts an action outside the narrowed scope
- THEN the action denies and the denial evidence names the attenuation restriction.
