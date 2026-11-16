## MODIFIED Requirements

### Requirement: Service lifecycle is explicit and generation-fenced
r[molten.system_extension.lifecycle] Aspen MUST model installation, admission, initialization, start, running, checkpoint, recovery, drain, failure, restart, upgrade, rollback, shutdown, and removal as explicit legal transitions. Every callback, timer, stream, effect, checkpoint, and recovery result MUST bind the active service generation. Stale, future, inactive, drained, or removed generations MUST deny mutation and externally visible output. System-extension transition-facing generation and incarnation MUST be distinct non-interchangeable local types retaining the complete `u64` range, including values above `2^63-1`. Conversion at numeric API and canonical Preserves boundaries MUST be explicit and MUST preserve existing packed bytes, field order and BLAKE3 content references. Type separation MUST NOT be treated as authority or as cross-wire domain fencing.

#### Scenario: Legal lifecycle reaches running state
- GIVEN an admitted service instance initializes and starts successfully
- WHEN the lifecycle core applies those results in order
- THEN the instance enters running state with a canonical generation and activation ref.

#### Scenario: Stale generation callback is rejected
- GIVEN an instance has been replaced by a newer generation
- WHEN a delayed callback result from the old generation arrives
- THEN the result is discarded or denied according to the callback contract
- AND it cannot emit effects or outputs for the active generation.

#### Scenario: Generation and incarnation are not interchangeable
- GIVEN a caller constructs a system-extension event or transition-facing field
- WHEN the caller supplies a generation where an incarnation is required, or the reverse
- THEN the program fails to compile rather than silently accepting the swapped values.

#### Scenario: Full-width values preserve existing identities
- GIVEN a valid generation or incarnation greater than `2^63-1` and within `u64`
- WHEN the lifecycle admits it or a boundary encodes and decodes it
- THEN the value remains unchanged and canonical bytes and content references match the prior raw-`u64` representation.

#### Scenario: Checked increments deny overflow
- GIVEN a lifecycle generation or incarnation is `u64::MAX`
- WHEN a transition requires that domain to advance
- THEN it denies with the existing domain-specific overflow issue and does not wrap.
