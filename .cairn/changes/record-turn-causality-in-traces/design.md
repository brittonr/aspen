# Design: Record turn causality in traces

## Context

Turn records are canonical Preserves values built in `src/runtime/turn/chronicle.rs` and hashed for trace evidence. The
record shape is stable and replay compares canonical refs. The reference harness already converts adopted Syndicate
observations into trace evidence with a replayability status, so a cause field extends that record rather than
introducing a second trace format.

## Approach

Add a cause to the committed turn record and validate it during trace construction:

- `prior-turn`: the turn that released this work, by turn ref.
- `cleanup`: the automatic retraction of an owner's outstanding assertions at the end of its life.
- `linked-task-release`: a task identified by ref with a reason of `cancelled` or `normal`.
- `periodic-activation`: the declared period.
- `delay`: the causing turn ref and the amount that expired.
- `external`: a bounded, redacted description for inputs outside the runtime model.

The vocabulary is closed. A record with an absent or unknown cause fails validation, and a truncated trace fails
closed rather than emitting partial evidence. Cause values derive from the transition input, so replay reproduces them
exactly.

Action kinds add `spawn`, `link`, `facet-start`, and `facet-stop`. Facet kinds consume the facet identity introduced by
`admit-facet-owner-scopes`; this package stays implementable without facet records by recording the facet path only.

## Decisions

### Decision: One closed cause vocabulary, validated on write

**Choice:** Validate the cause when the trace record is built, and refuse an unknown cause instead of recording an
opaque string.

**Rationale:** Causal data is only useful when a reader can group by cause. An open vocabulary degrades into free text
that no query can compare.

### Decision: Cause is part of canonical identity

**Choice:** Include the cause in the trace record that is hashed.

**Rationale:** Replay compares canonical refs. If the cause stayed outside the hash, a trace whose causality changed
would compare equal, and the drift would be invisible.

## Risks / Trade-offs

- Existing recorded traces lack a cause. They are re-recorded, and the change lists which fixtures moved. Historical
  receipts are never rewritten.
- `external` is the escape hatch, and it can absorb cases that deserve a real cause. Bounded descriptions plus a
  review note keep the vocabulary honest.
- Facet action kinds depend on the facet change. Until it lands, facet actions are recorded with a path and no facet
  identity, which this design states explicitly rather than guessing an identity.
