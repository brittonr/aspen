## Context

Retention drains individual events at 4,096 entries, but `subscribe` compares only the cursor revision with `first_retained_revision`. A retained revision may begin after ordinal zero, so revision equality does not prove complete history.

## Decisions

### Decision: Derive the retained ordinal from the first event

**Choice:** Keep the existing revision cutoff and additionally reject a cursor in the oldest retained revision when its ordinal precedes the first retained event. The retained event vector already owns this exact boundary, so no new state or public model field is necessary.

**Rationale:** This is constant-time, allocates nothing, preserves event identities and all subscription bounds, and avoids a duplicate retention watermark that could drift. An untrimmed stream begins at ordinal zero and retains the existing revision-zero start behavior.

### Decision: Compose the pending cursor refinements

This change and `add-durable-batch-progress` modify the same accepted `committed-change-stream` cursor requirement. Their final cursor blocks include both the progress snapshot/state-only rules and the partial-revision retention boundaries, with the same native merge block hash. Either sync order must preserve the other change; neither a duplicate capability nor relabelling an existing requirement as `ADDED` is an acceptable merge workaround.


## Risks / Trade-offs

- Service and epoch checks retain their precedence over retention classification.
- A cursor exactly at the oldest retained event is inclusive; later cursors paginate normally. Empty streams and after-last watermarks remain empty successful reads.
- Existing conservative cross-revision gap behavior remains unchanged. This fix does not claim cross-store crash atomicity, content retention or reading authority.

## Recorded paired execution

The repository-owned `evidence/paired-regression-2026-11-14.json` records the exact test, command, toolchain, source identities and both process outcomes. An isolated snapshot with the original stream blob `33a4fb23ac8a04f9ffc2b4c349c3ea87f5e9ad18` from `7c854d24d143ce788458e73e38500cfd31153896` failed the three-event/4,095-event regression: it returned ordinal 2 instead of the required retention gap. Replacing only that implementation with the fixed source made the same test pass (one selected test in each run).

The pre-fix archive BLAKE3 is `aaeeb4f83aa67648bbf34c77e29ef04fc8f47cce225994068ebf3cc20144bac1`; the fixed archive is `14c99dd4010a31a95ed552a8265b4043fcb6846267887bd065ea6372b123a729`. Complete source membership, archives, environment and logs remain under `target/source-gate-20261113/cursor-regression/`. This is native unit-test evidence, not full Nix acceptance, publication or release authority.

