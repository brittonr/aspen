# System extension runtime deltas

## ADDED Requirements

### Requirement: Native extension execution validates requests against independent current authority and rejects invalid outputs before terminal commit
r[molten.system_extension.independent_authority_and_output_gate] `Process::execution_request` MUST validate callback requests against independent current authority and active supervisor generation without copying `request` fields into `authority`. `IntentRecordingEffectPort::route` MUST reject invalid materialized outputs before persisting `NativeOperationState::Terminal` and MUST classify post-effect journal failures as `Uncertain`.

#### Scenario: Stale callback generation is rejected
- GIVEN a callback envelope carrying a generation older than the active supervisor generation
- WHEN `Process::execution_request` evaluates admission
- THEN `admit_request` MUST deny execution rather than passing `request.generation` as `active_generation`.

#### Scenario: Invalid effect output is not committed as Terminal
- GIVEN `admit_materialized_effect_output` rejects a delegated effect output
- WHEN `IntentRecordingEffectPort::route` records the result
- THEN it MUST NOT persist `NativeOperationState::Terminal` with the invalid output ref.
