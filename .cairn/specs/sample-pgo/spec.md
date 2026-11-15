# Sample Pgo Specification

## Purpose

Defines the `sample-pgo` capability.

## Requirements

### Requirement: Exact optional cohort

r[molten.sample_pgo.cohort] The experiment MUST use immutable compiler, converter, source, dependency, build, binary, dataset, and raw/converted profile identities; MUST NOT replace default/Octet compilers or enable profiler features implicitly.

#### Scenario: Source or binary differs
- GIVEN a captured profile is associated with a different source, compiler, flags, or baseline binary
- WHEN candidate construction is requested
- THEN construction MUST fail before a valid comparison is emitted.

### Requirement: Bounded external capture

r[molten.sample_pgo.capture] Sampling MUST be externally bounded and require the declared hardware event and LBR. Missing permission, missing samples, wrong build ID, or converter failure MUST block the experiment without substituting `.fxt` or instrumentation profiles.

#### Scenario: Hardware sampling is unavailable
- GIVEN perf cannot record the declared event and branch stack
- WHEN capture runs
- THEN the failed command and diagnostics MUST be retained and the result MUST be blocked, not measured stop or success.

### Requirement: Held-out repeated comparison

r[molten.sample_pgo.measurement] A fixed disjoint held-out workload, raw paired repetitions, explicit resource/warmup limits, practical threshold, regression checks, and uncertainty MUST precede a keep decision. Unsupported metrics MUST be identified, not invented.

#### Scenario: Training improves but held-out evidence does not
- GIVEN only training performance improves or uncertainty crosses the threshold
- WHEN a decision is calculated
- THEN the experiment MUST NOT recommend a maintained profile.

### Requirement: Canonical semantics before speed

r[molten.sample_pgo.replay] Successful, denied, exhausted, and error paths MUST retain and compare real canonical harness artifacts across both variants. Supported reports MUST replay through both variants with their recorded effects. Semantic divergence MUST invalidate performance eligibility.

#### Scenario: Candidate changes a deny or failure artifact
- GIVEN the same held-out input produces different canonical reports or failures
- WHEN equivalence is evaluated
- THEN the experiment MUST be ineligible regardless of timing.

### Requirement: Observations do not grant authority

r[molten.sample_pgo.non_claim] Results MUST remain machine-local development observations, MUST preserve existing `.fxt` role restrictions, and MUST NOT enable a default build profile, release admission, or universal performance claim.

#### Scenario: Measured experiment recommends keep
- GIVEN thresholds and replay gates pass
- WHEN a maintained profile is considered
- THEN a separate reviewed Cairn change MUST authorize that profile.
