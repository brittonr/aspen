# Complete blob/castore/cache readiness evidence Delta

## ADDED Requirements

### Requirement: Readiness promotion requires complete evidence [r[blob-castore-cache-extraction.promotion-requires-complete-evidence]]
The blob/castore/cache family MUST NOT be raised above `workspace-internal` until downstream fixtures, negative dependency checks, checker policy, and representative compatibility evidence are all captured.

#### Scenario: Readiness promotion requires complete evidence evidence [r[blob-castore-cache-extraction.promotion-requires-complete-evidence.evidence]]
- GIVEN the family readiness state is evaluated
- WHEN the reviewer proposes `extraction-ready-in-workspace`
- THEN the evidence SHALL include direct downstream fixture builds, cargo metadata, negative app-shell dependency checks, readiness checker output, and representative Aspen consumer checks.

### Requirement: Adapter paths remain explicit [r[blob-castore-cache-extraction.adapter-paths-explicit]]
Blob, castore, and cache Aspen runtime integrations MUST remain behind explicit features, adapter crates, or compatibility shells rather than leaking into reusable defaults.

#### Scenario: Adapter paths remain explicit evidence [r[blob-castore-cache-extraction.adapter-paths-explicit.evidence]]
- GIVEN a reusable default graph and an Aspen integration graph are both checked
- WHEN cargo tree evidence is captured
- THEN reusable defaults SHALL exclude node/bootstrap/handler/app shells and integration graphs SHALL name the feature or adapter that owns each runtime dependency.
