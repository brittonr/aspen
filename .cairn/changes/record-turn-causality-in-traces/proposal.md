# Proposal: Record turn causality in traces

## Why

`RuntimeEvent` (`src/runtime/turn/chronicle.rs`) records what a turn did: messages delivered, assertions committed and
retracted, observations, effect requests and responses, admission decisions, and rollbacks. It never records why the
turn ran. The harness `TraceEvidence` binds event refs and a replayability status, so a recorded trace cannot answer
"why did this turn run" or "which earlier turn released this work".

The manual makes the reason part of the record: a turn carries a `cause` that is a prior turn, `cleanup`,
`linkedTaskRelease`, `periodicActivation`, `delay`, or `external`, and each activation records one entry
(`42-protocols__syndicate__trace.md → Turn causes`). The accepted requirement
`molten.runtime_spine.interaction_tracing` is listed as implementation-unestablished in the tracey baseline.

## What Changes

- Every committed turn record MUST carry a cause from the reviewed vocabulary, and a trace record without a valid
  cause MUST fail validation. r[molten.runtime_spine.turn_causality]
- Add the cause vocabulary in Molten terms: prior turn by ref, cleanup, linked-task release with reason, periodic
  activation, delay with the causing turn and amount, and external with a bounded description.
- Add the action kinds the vocabulary needs to stay coherent, aligned with the facet owner scope change
  (`admit-facet-owner-scopes`): spawn, link, facet start, and facet stop, alongside the existing enqueue and dequeue
  events.
- Keep the cause deterministic and replayable: it comes from transition inputs, never from a wall clock, and it
  participates in the canonical record identity.
- Record the trace-completeness non-claim: a trace proves recorded causal order only.

## Impact

- **Files**: `src/runtime/turn/chronicle.rs`, `src/runtime/dataspace/state.rs`, `src/runtime/dataspace/tests.rs`,
  `tools/tracey/` trace proofs where turn records are validated, `docs/architecture.md` tracing section.
- **Testing**: positive cases for a dependency-driven turn and a cleanup turn; negative cases for a record without a
  cause, an unknown cause, a truncated trace, and a replay comparison that changes only the cause; `cargo test -p
  molten` before and after; focused Clippy.
- **Non-goals**: no claim that a trace is complete, no sampled or aggregated trace, no wall-clock cause, and no change
  to authority or receipt semantics.
