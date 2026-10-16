## Context

The existing standalone guard has two independent obligations: resolve source reference identities and enforce an exact inherited uncovered set. It currently uses the accepted set for both, so a meaningful marker belonging to an incomplete active package causes a pre-sync gate failure. Accepted coverage and lifecycle completion must not be weakened to resolve that ordering conflict.

## Decisions

### Decision: Separate identity resolution from accepted coverage

**Choice:** Load accepted identifiers and active-delta identifiers separately. Compute missing requirements and accepted reference counts from accepted identifiers only. Classify references that resolve only through an active delta as pending; compute dangling references against both sets. Retain the existing reviewed baseline unchanged.

**Rationale:** A proposed requirement can identify ongoing work without becoming an accepted requirement or coverage claim. Overlapping accepted and proposed identifiers remain accepted identities; active-only identifiers never enter inherited debt.

### Decision: Keep native lifecycle ownership explicit

**Choice:** Only native `.cairn/changes/<change>/specs/**/spec.md` delta specifications can contribute pending identifiers. Recognize ADDED and MODIFIED requirement sections, not REMOVED requirements or RENAMED labels. Proposal/design/tasks, archives, generated evidence, and other Markdown are not identity sources. Reuse the guard's existing identifier grammar. Native Cairn structural validation remains a separate required gate; discovery by this guard does not claim to perform semantic Cairn validation.

**Rationale:** Reusing the existing scanner and native delta layout avoids introducing a second lifecycle parser or a manually maintained allowlist. A malformed or incomplete package still cannot sync/archive under unchanged native gates.

### Decision: Preserve denial and expose non-claims

**Choice:** Unknown identifiers still deny normal execution and baseline-write mode. Continue rejecting new accepted gaps, stale baseline entries, malformed baselines, missing accepted roots, duplicates, and ordering errors. Report pending references separately and state that they establish identity resolution only.

**Rationale:** Pending-reference admission must not hide debt growth, become a baseline exception, or authorize acceptance.

## Risks / Trade-offs

- The guard establishes bounded source identity facts, not marker truth, runtime behavior, native package validity, or lifecycle acceptance. Cairn validation and review remain mandatory.
- A proposed identifier removed from all active deltas becomes dangling unless already accepted; negative transition cases must catch this regression.
- Source gate clearance can coexist with unfinished scheduler/consensus behavior. Existing packages and evidence must retain their honest incomplete status.
- The separate unavailable nominal-domain policy blocker is not resolved or exempted by this change.
