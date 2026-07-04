## ADDED Requirements

### Requirement: Syndicate traces are canonicalized as Molten evidence
r[molten.syndicate_dataspace.trace_evidence] Molten SHOULD convert adopted Syndicate trace observations into canonical Molten trace, turn, or parity evidence records. The records MUST bind the triggering event ref, committed action refs, actor or facet owner refs, route refs, and replayability status. Incomplete or live-only traces MUST remain diagnostic-only.

#### Scenario: Complete trace becomes replayable evidence
- GIVEN a Syndicate reference-harness run records the triggering event, committed assertions, retractions, messages, and owner refs
- WHEN Molten canonicalizes the trace
- THEN the trace evidence binds those refs with replayability status `recorded`
- AND replay can compare the trace by canonical refs.

#### Scenario: Incomplete trace is diagnostic-only
- GIVEN a Syndicate trace observation lacks a required committed action ref or owner ref
- WHEN Molten emits trace evidence
- THEN the evidence is marked diagnostic-only
- AND it cannot satisfy pass gates that require replayable turn evidence.
