# Coordination Delta

## MODIFIED Requirements

### Requirement: Semaphore and rate-limit primitives are bounded
r[molten.coordination.rate_limit_semaphore] Molten MUST implement semaphore and rate-limit coordination operations with manifest-defined bounds, deterministic pass/deny receipts, and resource-governance evidence refs. The `rate-limit` service MUST be a per-key acquisition quota for the lifetime of the coordination state; until coordination state carries an admitted logical time source, it MUST NOT be described or relied on as a time-window rate.

#### Scenario: Semaphore capacity denial is deterministic
- GIVEN all semaphore permits are held
- WHEN another acquire request arrives
- THEN Molten emits a deny receipt
- AND the semaphore state is unchanged.

#### Scenario: Rate-limit quota does not reset
- GIVEN a key has used its manifest-defined `rate-limit` quota
- WHEN later operations commit, including acquisitions on another key
- THEN a further acquisition on the exhausted key emits a deny receipt naming the quota and leaves state unchanged
- AND another key still receives its own full quota.
