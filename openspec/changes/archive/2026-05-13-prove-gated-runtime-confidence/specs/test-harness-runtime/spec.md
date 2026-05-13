## ADDED Requirements

### Requirement: Gated runtime confidence proof sweep [r[test-harness-runtime.gated-runtime-confidence-sweep]]

Aspen MUST support a staged operator proof campaign for confidence boundaries that the quick-confidence rail intentionally skips.

#### Scenario: Sweep begins from a clean baseline [r[test-harness-runtime.gated-runtime-confidence-sweep.clean-baseline]]

- GIVEN an operator is about to run gated runtime confidence proofs
- WHEN the sweep starts
- THEN it SHALL record the git branch/status and host prerequisites for KVM, TUN, virtualization CPU flags, and user permissions
- AND it SHALL stop or classify host blockers before interpreting gated VM failures as product failures

#### Scenario: Sweep runs from cheap product paths toward expensive gated proofs [r[test-harness-runtime.gated-runtime-confidence-sweep.staged-order]]

- GIVEN the quick-confidence rail has passed
- WHEN the gated proof sweep is executed
- THEN it SHALL run cheap runtime-host product-path checks before broader VM, nested-KVM, Hermit/uHyve, Hyperlight, dogfood, or full-flake proofs
- AND each later tier SHALL preserve the highest proof boundary reached before moving to a more expensive or more privileged tier

#### Scenario: Sweep classifies every result by proof boundary [r[test-harness-runtime.gated-runtime-confidence-sweep.boundary-classification]]

- GIVEN a proof command passes, fails, is cached, or is skipped
- WHEN evidence is reported
- THEN the report SHALL classify the result as one of static readiness, product-path marker, VM boot, microVM/VM product assertion, runtime-host receipt, dogfood/self-hosting acceptance, full-flake confidence, host capability blocker, build-input drift, or product behavior failure
- AND the report SHALL NOT claim a stronger boundary than the command actually reached

#### Scenario: Sweep preserves redacted evidence [r[test-harness-runtime.gated-runtime-confidence-sweep.redacted-evidence]]

- GIVEN proof commands produce logs or summaries
- WHEN evidence is retained for review or committed to OpenSpec artifacts
- THEN raw tickets, cluster cookies, credentials, connection strings, private keys, and equivalent secret material SHALL be redacted or omitted
- AND retained evidence SHALL prefer compact summaries, command lines, exit status, proof markers, derivation paths, and failure stages over full noisy logs

#### Scenario: Product failures route to follow-up changes [r[test-harness-runtime.gated-runtime-confidence-sweep.follow-up-routing]]

- GIVEN a gated proof exposes a real product behavior failure or a repair that spans multiple components
- WHEN the failure is classified
- THEN Aspen SHALL create or update an OpenSpec for that repair before implementation
- AND narrow build-input drift MAY be repaired directly only when the fix is local, low-risk, and verified by the failed proof command
