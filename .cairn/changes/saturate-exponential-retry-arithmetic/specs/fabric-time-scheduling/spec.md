# F12 saturating exponential retry

## ADDED Requirements

### Requirement: Exponential growth saturates without high-bit loss

r[molten.audit_f12.saturation]
Molten MUST compute exponential retry delay with checked multiplication or a pre-shift overflow check and cap it at `maximum_delay_ticks`.

#### Scenario: Ordinary growth remains exact
- GIVEN an admitted exponential policy whose mathematical delay is below its maximum
- WHEN the generic planner computes a retry
- THEN the delay equals the mathematical exponential result before admitted jitter

#### Scenario: High bits exceed the integer range
- GIVEN base delay two, attempt 63, maximum delay 128, no jitter, and an admitted attempt budget
- WHEN the generic planner computes a retry
- THEN the delay is 128 rather than zero

### Requirement: Arithmetic bounds remain explicit

r[molten.audit_f12.bounds]
Molten MUST use named arithmetic bounds and handle every admitted attempt without narrowing wrap or an attempt-sized loop.
Molten MUST preserve finite-attempt admission and checked deadline arithmetic.

#### Scenario: A large admitted attempt saturates
- GIVEN an admitted attempt exceeds the delay integer width and the target instant can represent the capped delay
- WHEN the exponential planner runs
- THEN it returns the maximum delay without narrowing-conversion failure

#### Scenario: A deadline cannot represent the delay
- GIVEN the capped delay exceeds the remaining target-instant range
- WHEN deadline construction runs
- THEN it returns the existing arithmetic error without a wrapped deadline or retry effect

### Requirement: Existing admission and fixed behavior remain intact

r[molten.audit_f12.compatibility]
Molten MUST preserve fixed-delay outputs and generation, domain, attempt, and jitter admission.
Shells MUST preserve consumer state and schedule no retry after a rejected plan.

#### Scenario: The current delivery profile remains fixed
- GIVEN the coordination-delivery fixed-delay profile and valid explicit time inputs
- WHEN it requests a retry
- THEN its delay and deadline remain unchanged by the exponential correction

#### Scenario: Retry inputs are invalid
- GIVEN exhausted attempts, invalid jitter, a stale generation, or a mismatched time domain
- WHEN retry admission runs
- THEN the declared error occurs without a timer effect or consumer-state mutation

### Requirement: Evidence distinguishes planner behavior from deployment claims

r[molten.audit_f12.validation]
Molten MUST retain positive and negative normal repository tests for arithmetic, adapters, and replay.
Evidence MUST scope F12 to the generic exponential planner and MUST NOT claim fixed-profile exposure, safe retry, or measured performance.

#### Scenario: Corrected retry evidence matches the plan
- GIVEN a valid saturated plan and matching adapter inputs
- WHEN the shell records the retry observation
- THEN its canonical delay and deadline match the corrected plan

#### Scenario: Historical replay expects wrapped delay
- GIVEN a legacy trace expects the former truncated exponential result
- WHEN corrected replay evaluates that position
- THEN it reports divergence or an explicit unsupported cohort without rewriting historical receipts
