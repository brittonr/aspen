# Crash-restart conformance

A process disappearing is an interruption observation, not a transaction outcome. Molten's world-fault rail makes that distinction explicit by comparing owner decisions with durable read-back at semantic phases. This article assumes the [world crash and concurrency conformance guide](../../world-fault-conformance.md) and explains the shell/core boundary and the conservative interpretation of restart evidence. The [Technical companion](../README.md) links related storage and world-state discussions.

## Mutation-specific linearization

The mutation inventory binds each operation family to an owner, linearization point, durable record, and recovery entry. Capture, head updates, promotion, witness records, outbox attempts, replication availability, imports, retention roots, and garbage-collection plans are not interchangeable writes. For example, committing a garbage-collection plan does not mean executing every deletion, and recording an outbox attempt is not evidence that an external effect completed.

The governing guide defines eight semantic phases: uninterrupted, before submit, after possible submit, after durable write, before response, lost response, process restart, and recovery read-back. These names identify meanings at adapter hooks, not source-line positions or elapsed wall-clock intervals. A refactor can move a hook physically while preserving its semantic phase; conversely, an unchanged line number can become the wrong hook if submission behavior changes.

The [reviewed local profile](../../../config/world-faults/profiles/local-deterministic.ncl) makes expectations explicit. Before submission expects `safe-to-retry`; after possible submission, lost response, and process restart expect `uncertain`; recovery read-back expects `already-complete` for its supported standard cases. These are expectations for the reviewed cases, not a universal classifier that can infer durable outcome from phase alone. Witness cases remain separately unsupported and use their conservative profile treatment.

## Shell sequencing without decision appropriation

The [shell service](../../../src/world_faults/service.rs) validates inventory and profile before iterating supported cases. It calls the fault-control port, invokes restart for process-restart and recovery-read-back phases subject to the restart bound, obtains durable observations, and passes those observations plus the interruption to the owner-decision port. The resulting observation records the case and operation identities, phase, submission and response observations, durable read-back, owner decision, rollback flag, and cleanup flag.

This orchestration establishes call ordering, not a concrete operating-system restart implementation. The [port contract](../../../src/world_faults/ports.rs) delegates restart, durable read-back, and owner classification to distinct application-owned interfaces. The governing requirement is that the shell reopen local node state before supplying durable read-back. Reviewing the generic service alone cannot demonstrate that a particular adapter actually reopens a store; that requires adapter and execution evidence.

After case and schedule evaluation, the shell canonicalizes and publishes the receipt. It rejects a publication port returning a different record identity. Artifact publication is therefore tied to the expected canonical record rather than accepted merely because some file or log was produced.

## Conservative observation rules

The [comparison implementation](../../../crates/molten-core/src/world_faults/conformance/comparison.rs) checks operation identity, phase, and expected owner decision, then applies independent conservative constraints. `AlreadyComplete` requires applied durable status and both state and record refs. Missing durable state cannot become success. `PossiblySubmitted` combined with `SafeToRetry` is rejected. Corrupt read-back requires corrupt or manual-review classification; contradictory read-back requires conflict or manual review.

For whole-store rollback without an independent witness, only uncertain or manual-review classification survives the conservative rule. This matters because restoring a local image can roll back both a head and its generation. Agreement among fields from that same image is not independent evidence of freshness. The local profile's unsupported witness row remains visible precisely because local consistency cannot establish non-rollback.

Cleanup is another separate boundary. The comparison rejects an asserted cleanup authorization without complete applied state and an independent witness. Passing this predicate is not a new cleanup capability: the governing guide explicitly says a focused conformance receipt never grants cleanup authority.

## Worked lost-response case

Consider an illustrative promotion that may have submitted its transaction before the process loses its response. Retrying immediately because no success message arrived would confuse an observation about communication with a fact about persistence. A `PossiblySubmitted` observation and `SafeToRetry` owner decision fail the conservative comparison even if the reviewed test expected a favorable outcome elsewhere.

A later recovery case can read exact applied state and its record, after which the owner may classify the operation as already complete under its own rules. If read-back is contradictory, the rail cannot “repair” the verdict into completion to make the case pass. The useful evidence is the transition from uncertainty to an owner-supported classification with exact facts, not a claim that retry made execution exactly once.

Concurrent schedules add a related constraint. The [schedule evaluator](../../../crates/molten-core/src/world_faults/schedule.rs) checks observation bindings and missing or duplicate operations, rejects multiple applied outcomes, and limits total effect release for promotion and outbox schedules. These bounded checks do not model every possible concurrent history.

## Verification and non-claims

Suggested review is to trace each phase from profile through adapter hook, restart action, reopened read-back, owner decision, and canonical receipt. Negative cases should expose unsafe retry, missing durable records, contradictory state, and local rollback overclaims. No crash or VM experiment was executed for this article.

A passing receipt establishes bounded agreement for its exercised cohort. It does not establish physical power-loss safety, general storage correctness, universal concurrency correctness, release eligibility, or independent rollback detection. The layered evidence distinction in [distributed testing](../../distributed-testing.md) remains applicable even when the conformance decision is favorable.

## Sources

- [World crash and concurrency conformance](../../world-fault-conformance.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [Reviewed local fault profile](../../../config/world-faults/profiles/local-deterministic.ncl)
- [Shell conformance orchestration](../../../src/world_faults/service.rs)
- [Shell port contracts](../../../src/world_faults/ports.rs)
- [Conservative observation comparison](../../../crates/molten-core/src/world_faults/conformance/comparison.rs)
- [Concurrent schedule evaluation](../../../crates/molten-core/src/world_faults/schedule.rs)
- [Technical companion](../README.md)
