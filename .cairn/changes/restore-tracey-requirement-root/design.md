# Design: Restore the Tracey requirement root

## Context

The inherited-debt guard compares accepted requirement definitions with evidence references and with a reviewed
baseline of inherited uncovered ids. It denies dangling references and baseline growth. The classifier maps each
baseline id to its specification. Nix checks bind the baseline, classification, and repair manifests by BLAKE3.

## Decisions

### Decision: Fail closed on a missing or empty requirement root

**Choice:** The guard returns an error when `.cairn/specs` is missing or yields no requirements. The classifier
returns an error when the root is missing.

**Rationale:** A root that silently vanished produced `requirements=0`, and the guard's failures then pointed at
references instead of the root. An explicit root error names the cause. The accepted tree always contains
requirements, so an empty set means misconfiguration.

### Decision: Add markers only where evidence exists

**Choice:** Add `impl` or `verify` evidence marker comments only where code, a test, a document section, or a Nix check
implements or verifies the requirement text. Leave the three ids without implementation uncovered and failing.

**Rationale:** Growth denial forbids adding new requirements to the baseline, and a marker without evidence would be
a false coverage claim.

### Decision: Repair CAS markers through a MODIFIED delta

**Choice:** Restate the four CAS requirements with standalone `r[...]` lines. Titles, text, and scenarios stay the
same.

**Rationale:** This is the marker-repair path the accepted debt requirement names. It takes effect when the change
syncs.

## Risks / Trade-offs

- The guard still fails on this branch. Thirty-three dangling ids resolve only through this change's sync (4) and the
  owning changes' syncs (28, one of them undefined), and three requirements stay uncovered pending implementation.
  Verification records exact counts instead of claiming a pass.
- Moving traceability roots in the vendored policy is a documented local port. A future upstream refresh must keep
  it.
