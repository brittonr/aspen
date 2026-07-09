## Context

The codebase already uses named hard caps for many safety-sensitive paths. Those caps are important and should remain compiled review boundaries. The configurability gap is below that layer: operators need to choose smaller or environment-specific budgets without editing Rust constants or repeating CLI defaults.

## Decisions

### Hard caps stay compiled and named

**Choice:** Runtime limit profiles may select effective values only within compiled hard caps. Raising a hard cap remains a code/review change.

**Rationale:** Profiles should tune deployments, not silently enlarge the safety envelope.

### Limit admission is a pure core

**Choice:** Add a deterministic admission core that takes hard-cap descriptors, profile-selected values, profile tier, and optional CLI overrides, then returns effective limits or denial diagnostics.

**Rationale:** Bound checks and coherence rules are pure and should be testable without running services or reading files.

### Profiles carry units and coherence rules

**Choice:** Limit profiles include units and relationships such as timeout envelopes, attempts versus retry receipts, max frame bytes versus session bytes, and queue depths versus service loop budgets.

**Rationale:** Reviewers should see what a number means and when two numbers contradict each other.

### Receipts bind effective limits

**Choice:** Subsystems using configurable limits bind the admitted effective limit profile ref and key effective values into receipts or readback artifacts.

**Rationale:** Replay/review should know which budget constrained an operation.

## Validation strategy

- Positive tests for accepted limits below hard caps.
- Negative tests for one-past-cap values, zero/overflow values, contradictory timeouts, and denied production overrides.
- Representative integration tests for node serve/live-send, chunking, retention GC, and harness budgets.
- Traceability updates for each subsystem that consumes admitted limits.

## Non-claims

Runtime limit profiles do not prove liveness, fairness, availability, adapter health, policy authority, or release readiness. They only bound selected resource and time budgets.
