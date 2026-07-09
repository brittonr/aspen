# Resource Governance Backpressure Specification

## Purpose

Defines the `resource-governance-backpressure` capability.

## Requirements

### Requirement: System MUST Define canonical resource grant and consumption records with scope, kind, amount, rate/window, expiry, parent pool, policy refs, and evidence refs
r[molten.resources.grant_model] The system MUST Define canonical resource grant and consumption records with scope, kind, amount, rate/window, expiry, parent pool, policy refs, and evidence refs.

### Requirement: System MUST Define initial resource kinds for turns, CPU/fuel, memory, mailbox slots, assertions, blob bytes, storage bytes, network messages/bytes, effect calls, and trace bytes
r[molten.resources.kinds] The system MUST Define initial resource kinds for turns, CPU/fuel, memory, mailbox slots, assertions, blob bytes, storage bytes, network messages/bytes, effect calls, and trace bytes.

### Requirement: System MUST Document that resource grants do not imply data access or capability authority
r[molten.resources.no_data_authority] The system MUST Document that resource grants do not imply data access or capability authority.

### Requirement: System MUST Emit receipts for grant, consume, throttle, deny, renew, revoke, and cleanup decisions
r[molten.resources.receipts] The system MUST Emit receipts for grant, consume, throttle, deny, renew, revoke, and cleanup decisions.

### Requirement: System MUST Enforce deterministic mailbox bounds and overflow behavior
r[molten.resources.mailbox_bounds] The system MUST Enforce deterministic mailbox bounds and overflow behavior.

### Requirement: System MUST Enforce actor turn budgets and deterministic cancellation/yield points
r[molten.resources.turn_budgets] The system MUST Enforce actor turn budgets and deterministic cancellation/yield points.

### Requirement: System MUST Enforce dataspace assertion and subscription count limits
r[molten.resources.assertion_bounds] The system MUST Enforce dataspace assertion and subscription count limits.

### Requirement: System MUST Add deterministic scheduler fairness/backpressure policy independent of OS thread timing
r[molten.resources.scheduler_fairness] The system MUST Add deterministic scheduler fairness/backpressure policy independent of OS thread timing.

### Requirement: System MUST Wire Wasmtime execution to admitted fuel/epoch/deadline budgets
r[molten.resources.wasmtime_fuel] The system MUST Wire Wasmtime execution to admitted fuel/epoch/deadline budgets.

### Requirement: System MUST Add cooperative budget checkpoints for Steel and native actors
r[molten.resources.steel_native_budgets] The system MUST Add cooperative budget checkpoints for Steel and native actors.

### Requirement: System MUST Enforce blob, storage, network, remote sync, and trace-volume budgets in adapters
r[molten.resources.blob_storage_network] The system MUST Enforce blob, storage, network, remote sync, and trace-volume budgets in adapters.

### Requirement: System MUST Feed resource budgets into distributed job DAG placement and fusion decisions
r[molten.resources.job_dag_planning] The system MUST Feed resource budgets into distributed job DAG placement and fusion decisions.

### Requirement: System MUST Add tests for deterministic queue overflow, throttling, denial, and supervisor signaling
r[molten.resources.backpressure_tests] The system MUST Add tests for deterministic queue overflow, throttling, denial, and supervisor signaling.

### Requirement: System MUST Add replay tests proving budget decisions reproduce under the same profile and seed/log
r[molten.resources.replay_tests] The system MUST Add replay tests proving budget decisions reproduce under the same profile and seed/log.

### Requirement: System MUST Add tests that revoked/expired budgets deny future work and clean up dependent state
r[molten.resources.revocation_tests] The system MUST Add tests that revoked/expired budgets deny future work and clean up dependent state.

### Requirement: System MUST Add Hegel property tests for budget monotonicity, queue bounds, and no-silent-drop invariants
r[molten.resources.property_tests] The system MUST Add Hegel property tests for budget monotonicity, queue bounds, and no-silent-drop invariants.

### Requirement: Syndicate flow-control observations become Molten resource evidence
r[molten.syndicate_dataspace.flow_control_receipts] Molten SHOULD record Syndicate account, debt, loaned-item, fanout, and repayment observations as canonical Molten resource or backpressure evidence where the Syndicate reference harness uses them. Decisions MUST be derived from explicit bounds and recorded observations, not host scheduler timing.

#### Scenario: Fanout debt is bounded deterministically
- GIVEN a reference harness input that routes one incoming assertion to multiple observers
- WHEN the fanout would exceed the declared resource budget
- THEN Molten emits throttle or deny resource evidence with account/debt observations
- AND committed dataspace state follows the deterministic resource decision.

#### Scenario: Scheduler timing cannot change resource decision
- GIVEN the same canonical harness input, budget, account observations, and repayment sequence
- WHEN host thread scheduling differs between runs
- THEN the Molten resource decision and receipt refs remain unchanged
- OR the evidence is marked diagnostic-only because required observations were not recorded.

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
