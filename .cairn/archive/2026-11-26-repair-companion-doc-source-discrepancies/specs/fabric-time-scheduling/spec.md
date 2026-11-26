# Fabric time scheduling deltas

## ADDED Requirements

### Requirement: Retry planning rejects wall-clock deadlines
r[molten.fabric_time.retry_deadline_domain] Retry planning MUST validate the deadline domain before returning a deadline. A retry based on a wall-clock time value MUST be denied at construction, not only when the deadline is later evaluated.

#### Scenario: Wall-clock retry base is denied
- GIVEN a retry request whose current time value is in the wall-clock domain
- WHEN retry planning runs
- THEN it MUST return a typed deadline-domain error and no deadline.

#### Scenario: Monotonic retry base is accepted
- GIVEN a retry request whose current time value is monotonic
- WHEN retry planning runs
- THEN it MUST return a deadline in the monotonic domain.

### Requirement: Entropy evidence binds the stream capability
r[molten.fabric_time.entropy_capability_evidence] Canonical entropy evidence MUST include the capability reference of the stream that produced it, under a versioned evidence label. Replay MUST reject entropy evidence whose capability reference differs from the admitted stream.

#### Scenario: Mismatched entropy capability is rejected on replay
- GIVEN recorded entropy evidence whose capability reference differs from the admitted stream's reference
- WHEN replay checks the evidence
- THEN replay MUST fail with a typed capability-mismatch diagnostic.
