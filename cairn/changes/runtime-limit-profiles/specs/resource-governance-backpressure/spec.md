## ADDED Requirements

### Requirement: Runtime limit profiles select budgets under hard caps
r[molten.resources.limit_profiles.bounded_selection] Molten SHOULD support reviewed runtime limit profiles that select effective operational budgets under compiled, named hard caps. Profile-selected values MUST fail closed when they exceed hard caps, are non-positive where positive values are required, overflow checked arithmetic, or contradict subsystem coherence rules.

#### Scenario: Valid limit profile admits effective budgets
- GIVEN a runtime limit profile whose control-loop, live-send, frame, chunk, retention, and harness values are within compiled hard caps
- WHEN limit admission evaluates the profile
- THEN it returns effective budgets with pass diagnostics
- AND receipts can bind the admitted limit profile ref.

#### Scenario: One-past-hard-cap denies
- GIVEN a runtime limit profile selects a value one greater than a compiled hard cap
- WHEN limit admission evaluates the profile
- THEN admission denies before the value is used by a runtime shell
- AND diagnostics name the cap and selected value.

### Requirement: Limit profiles declare units and coherence
r[molten.resources.limit_profiles.units_coherence] Runtime limit profiles MUST declare units or named domains for reviewed numeric values and SHOULD validate relationships between related values, including retry attempts and timeout envelopes, frame size and session limits, queue depth and service-loop budgets, and retention scan bounds.

#### Scenario: Coherent timing envelope admits
- GIVEN a profile whose live-send join timeout, listener timeout, max attempts, and service tick bounds preserve the reviewed timing envelope
- WHEN limit admission evaluates the timing values
- THEN the timing block is admitted and bound to effective config.

#### Scenario: Contradictory timing envelope denies
- GIVEN a profile whose retry/timeout values contradict the reviewed timing envelope or would make receipt emission ambiguous
- WHEN limit admission evaluates the timing values
- THEN admission denies before live transport or service loops use the profile.

### Requirement: Limit profile admission is pure-core
r[molten.resources.limit_profiles.pure_core] Limit profile admission MUST be computed by a deterministic pure core over explicit profile values, hard-cap descriptors, and override inputs. Shells MUST own filesystem reads, environment lookup, CLI parsing, live timers, service loops, and receipt writing.

#### Scenario: In-memory limit admission succeeds
- GIVEN in-memory hard-cap descriptors and a valid profile value
- WHEN the admission core runs in a unit test
- THEN it returns effective limits without reading files, inspecting environment, using clocks, or starting services.

#### Scenario: Shell cannot bypass admission
- GIVEN a CLI command receives a profile-selected budget
- WHEN the command starts a service loop or live transport operation
- THEN it uses only limits returned by the admission core
- AND denied values are not applied as fallback defaults.

### Requirement: Effective runtime receipts bind admitted limits
r[molten.resources.limit_profiles.receipt_binding] Runtime receipts for configurable bounded operations SHOULD bind the admitted effective limit profile ref or the effective limit values sufficient for replay and operator review.

#### Scenario: Service receipt binds service limits
- GIVEN a node service loop runs under an admitted limit profile
- WHEN it emits a service or startup receipt
- THEN the receipt records the profile ref or effective tick/request/event limits used by the loop.

#### Scenario: Default budget caveat is visible
- GIVEN a command uses built-in local defaults because no limit profile was supplied
- WHEN the command emits a receipt
- THEN the receipt or effective-config readback records a default-budget caveat rather than implying a reviewed operator profile was used.
