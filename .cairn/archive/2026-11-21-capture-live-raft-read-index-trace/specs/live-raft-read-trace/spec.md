# Bounded live Raft read trace

## ADDED Requirements

### Requirement: Trace capture is explicitly selected and bounded

r[molten.testing.live_raft_trace.capture] An explicitly selected local diagnostic MUST admit only its fixed three-voter fixture and bounded step/byte profile; record ordered, correlated execution and read outcomes from the real Raft service; and refuse missing observations instead of substituting plan or send facts. Ordinary runtime and test selections MUST NOT enable trace retention.

#### Scenario: The live service crosses the read barrier
- GIVEN an explicitly selected fixed three-voter fixture with a retained prior write acknowledgement
- WHEN a new leader receives a read before and after its commit barrier
- THEN the capture contains the actual retry and current quorum read with distinct request identities and ordered observed facts.

### Requirement: The model binds the observed read-index window

r[molten.testing.live_raft_trace.model] The diagnostic MUST freeze the Aspen model and configuration and assign each modeled variable from a recorded boundary or a documented derivation of the recorded facts. The baseline MUST represent the current-term commit rule and the committed-log exception. An absent boundary or variable MUST be unsupported, never filled by guessing.

#### Scenario: An incomplete read observation is not a model state
- GIVEN a capture missing a required acknowledgement or read field
- WHEN a complete state sequence is requested
- THEN the missing fact prevents a supported trace verdict.

### Requirement: A trace verdict remains selected diagnostic evidence

r[molten.testing.live_raft_trace.checking] The evidence MUST bind the exact source, bounds, events, model/configuration, mapping and checker identities. A clean and negative trace MUST be compared against the same baseline. A failed, incomplete or unsupported arm MUST stay distinct from valid conformance, and no bounded result confers production or release authority or replaces the existing ConsensusFastPath or Quint rails.

#### Scenario: Checker cannot decide a trace
- GIVEN a missing dependency, unmapped action or unsupported checker outcome
- WHEN evidence is retained
- THEN the exact blocker is recorded without asserting implementation correctness, replay, or release qualification.
