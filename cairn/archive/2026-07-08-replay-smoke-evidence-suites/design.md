## Context

Accepted requirements already state that evidence-bearing runs declare replay status and that non-replayable exploratory runs cannot satisfy gates. The improvement is to make that invariant mechanically exercised for every eligible suite.

## Design

A replay smoke helper takes an in-memory suite descriptor and expected replay eligibility. For deterministic suites it performs:

- fresh run from declared fixtures;
- replay using recorded effect log;
- second fresh run from the same identity;
- comparison of report refs, final-state refs, effect-log refs, and relevant trace or receipt refs.

For non-replayable suites, the helper asserts explicit exclusion from deterministic evidence and checks the diagnostic or caveat.

The pure core should classify replay eligibility and compare canonical refs. Shell code owns CLI invocation, filesystem fixtures, and optional report writing.

## Validation

Start with representative harness, CLI-generated report, distributed simulation, and dogfood-diagnostic cases. Positive tests cover deterministic replay stability. Negative tests cover missing effect log, changed effect response, ambient-state marker, and non-replayable evidence presented as deterministic pass evidence.
