# Implementation review: peer-query recovery and voting admission

## Scope and decision

Self-review of the Molten consensus-recovery core, live Raft transitions, executor, Redb replay, and existing three-process harness. The implementation replaces the deferred local-admission readback with actual peer exchange and durable repair. Execution results and source-gate receipts are recorded in [verification](evidence/2026-10-29-peer-recovery-verification.md); this review is not independent certification or release authority.

## Findings addressed

- **Unsafe absence evidence:** reporting peers must intersect commit quorums without counting the suspect participant. Complete contiguous held tails, including entries above reported commitment, participate in reconciliation. Missing or faulty local entries do not establish absence. Committed conflicts suppress repair effects.
- **Delayed acknowledgements and lost votes:** absence-based truncation and admission require reports behind a persisted higher-term barrier. Higher-term traffic invalidates cached observations. Lost hard state cannot receive a bounded vote fence until every other voter's term is observed; peer vote records are never copied.
- **Recovery availability:** one complete committed source can repair data without granting voting or truncation authority. Partial pages survive timers. Cached complete observations continue multi-batch repair even when another peer remains unavailable. A changing corpus restarts pagination, so progress is explicitly stable-window scoped.
- **Uncertainty across clean restart:** every persistence/application batch durably holds the recovery-required marker until completion, including ordinary healthy-voter batches. Failure stops the scoped replica; clean close/open cannot erase the marker. Successful completion clears it with the exact planned term and vote. Additional fence writes are admitted against the effect budget before effects run.
- **Transport loss:** failed sends retain typed bounded failure observations and timers instead of stopping a healthy replica. Storage/application failures and invalid success evidence still fail closed.
- **Snapshot replay and unresolved suffixes:** the latest snapshot is a self-contained replay base. Equal coverage selects the highest Raft boundary; conflicting equal boundaries deny. Peer repair retains unresolved above-boundary evidence, and durable suffix-disposition records preserve that decision across restart. Committed suffixes cannot be silently discarded.
- **Regression validity:** power-loss-image checks explicitly reconcile repaired storage, then replay the actual durable records independently of the transient repair flag. This prevents quarantine itself from making the durable-intent assertion pass. Clean-restart checks separately exercise the live startup admission wrapper.

## Ownership and remaining boundaries

Molten's fabric-consistency maintainers own the core/shell integration and regressions. Existing live replica services and `RedbReplicaPort` consume the capability directly; no new external adoption step or compatibility shim is required.

The live backend remains a three-voter, non-production-admitted cohort. Larger-membership pure-model regressions do not establish a larger live backend. Malformed durable histories remain denied rather than automatically reset. The retained ChaosControl campaigns are model observations, not guest, Raft, or Molten execution proof. Neither source gates nor the process harness establish arbitrary hardware durability or whole-cluster correctness.
