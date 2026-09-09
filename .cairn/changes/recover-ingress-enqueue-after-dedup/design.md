## Context and evidence

Finding F03 applies to `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

`src/node/parts/daemon/p026/body.rs:133-173` commits dedup before submission and suppresses duplicate queue-read errors. `src/delivery/parts/idempotency/p001/body.rs:255-302` persists the first decision before it authorizes the caller effect. `src/node/parts/daemon/p018/body.rs` writes the inbox before its queue receipt.

Trigger: submit one admitted envelope with an unused sequence and exact request reference. Commit its dedup identity, then stop before inbox publication. Retry the same envelope. Current code suppresses enqueue, returns no queue reference, and supplies no diagnostic. Expected behavior reports recovery or uncertainty, not successful delivery without queue evidence.

A second required case stops after inbox publication but before its receipt. A third case permits dispatch before recovery observes the inbox. These cases prevent a repair based only on receipt absence.

The original evidence is static only. No crash or storage-fault injection ran. The audit recorded 359 passing core tests and eight failed regression assertions in other findings. Those results do not establish F03 recovery behavior.

## Pure reconciliation model

The core receives exact delivery identity, sequence-window facts, durable intent, queue observations, dispatch observations, and storage health. It classifies fresh publication, known incomplete publication, existing publication, completed dispatch, conflict, or outcome unknown.

Each decision returns a typed effect plan or a diagnostic result. The core does not open Redb, inspect files, or infer that an error proves absence. Operation and payload identity must agree across every observation.

The shell persists recoverable intent before effects and records observed publication afterward. An atomic store boundary or a recoverable journal can implement that contract. The implementation review selects the smallest existing mechanism that preserves the required observations. This plan mandates no new dependency.

Recovery publishes only after authoritative reconciliation establishes that publication and dispatch did not occur. Existing inbox content is reused after exact binding checks. Existing dispatch evidence suppresses reexecution. Missing queue receipts require reconciliation, not blind replay.

A commit or synchronization error preserves an unknown outcome. Conflicting bytes, unavailable observations, or quarantined storage block enqueue, dispatch, and successful acknowledgements. Rejection preserves prior dedup, inbox, and dispatch state apart from explicitly separate diagnostic evidence.

## Shell and adapter boundary

Existing application-owned delivery and node-state capabilities supply persistence observations. The shell owns transactions, bounded reads, reopen, and effect order. Node-host adapters retain capability-rooted filesystem access. Vendor types stay outside the reconciliation core.

Tests inject boundaries before dedup, after dedup, after inbox publication, after dispatch, and before receipt publication. They reopen persisted state rather than relying only on in-memory flags. Positive cases cover fresh enqueue, safe recovery, and exact duplicate suppression. Negative cases cover identity conflict, missing receipts, read errors, ambiguous commit, and quarantined storage.

## Compatibility, replay, and receipts

Preserve delivery identity and sequence-window semantics. A legacy dedup entry without publication evidence is unresolved until exact local observations establish its outcome. It is neither proof of successful enqueue nor permission to replay.

Queue and ingress receipts distinguish successful observed publication, duplicate suppression, incomplete recovery, and unknown outcomes. Review whether existing receipt schemas can express these facts or need a versioned extension. Historical receipts retain their original provenance.

Recovery must not execute the underlying control operation automatically. Reconstructed queue evidence records observations, not invented original receipts. Dispatch remains subject to normal operation admission and its own duplicate rules.

## Overlap and ownership

Molten node-runtime owns ingress meaning. Delivery-idempotency owns dedup identity and sequence rules. Node-host owns local filesystem mechanics.

`recover-from-storage-faults` owns general uncertain persistence, quarantine, and explicit reopen policy. Agree on those result types before adapter integration. F03 can define and test local ingress decisions independently. It does not add consensus repair, voting rules, or peer validation.

F01 owns shutdown admission after dispatch. F14 owns historical shutdown result interpretation. F02 owns startup gate evidence used by fixtures. These packages share tests and observations, not circular blocking dependencies.

## Validation and claim boundary

Baseline `control_ingress_enqueues_once_and_preserves_provenance_gate` and `control_ingress_denies_missing_authority_before_enqueue` before core edits. Repeat them after edits with normal repository recovery regressions.

Run focused Octet and Clippy error gates, workspace tests, relevant Nix delivery and node-state checks, and required Cairn gates. Keep failed and blocked runs explicit. Local fault tests do not prove power-loss durability, exactly-once external effects, consensus safety, or release readiness.
