## Context

The accepted lifecycle cleanup-gate requirement preserves owner, finalizer, pin, retention, and authority blockers. Resource metadata already caps owner refs and finalizers at 64 and evidence refs at 256, while annotation values are capped at 4096 bytes. `DeletionGateInput` is public and bypasses metadata validation; its seven collections and each contained string lacked any admission limit before the producer formatted diagnostics and cloned the selected diagnostic vector. The six Octet positions in this function are one producer, not six unrelated owners.

## Decisions

### Reuse resource metadata ceilings before diagnostic construction

**Choice:** Examine all seven collection lengths first: 64 owners/finalizers, 256 cleanup receipts/live owners/pins/retention policies/authority refs. Then inspect the resource ref and every nested fact for a 4096-byte maximum. Only after both preflights succeed can the existing decision evaluation format, accumulate, or clone its outputs.

**Rationale:** Individual byte and count bounds together bound diagnostic memory and processing. An exact-bound fact remains admissible; a single oversized fact must not be formatted before rejection. The existing owner/finalizer/evidence limits avoid a second arbitrary limit policy.

### Preserve admitted decisions and fail before giving an oversized one

**Choice:** Keep `deletion-ready` when no blocker remains, `blocked` when any pin, live owner, finalizer, retention hold, or missing authority remains, and do not turn a validation error into either decision.

**Rationale:** Admission is not deletion authorization. Previously accepted in-bound cases keep their diagnostics and ordering; no gate receipt or new authority claim is introduced.

## Risks / Trade-offs

Inputs exceeding a reused resource limit, including formerly accepted gigantic strings or redundant refs, now deny before decision construction. This does not claim the Octet static analyzer infers the guards or that a warning-only probe proves release eligibility. The original catalog baseline and unrelated hardening producers remain unchanged.
