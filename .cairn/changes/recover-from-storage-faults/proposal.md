# Proposal: Recover from storage faults

## Why

Molten labels admitted Redb transitions as durable after an ordinary commit result. Snapshot payloads currently use buffered file flush before their metadata commit.

An `fsync` or commit error does not prove that no bytes reached storage. The kernel page cache can also retain bytes that differ from persistent media. A local database repair can restore structure without proving that a Raft node retained committed history.

Molten needs explicit uncertain outcomes, adapter quarantine, durable snapshot publication, and protocol-aware peer repair.

## What Changes

- Separate requested durability from observed persistence outcomes.
- Add `OutcomeUnknown` and poisoned-store states for synchronization and commit failures.
- Block normal reads, writes, acknowledgements, and voting after a storage fault.
- Reopen and reconcile storage through an explicit shell workflow.
- Publish snapshot payloads with the pinned Durable File Publication component before metadata visibility.
- Check snapshot content identities during startup and installation.
- Add protocol-aware Raft repair using term, index, commitment, and snapshot facts.
- Add participant-scoped recovery progress for local sufficiency, peer-available repair, global absence, and incomplete observation.
- Prevent a locally repaired node from voting until peer validation completes.
- Add storage-fault caveats to consensus and durability receipts.
- Exercise the behavior through pinned ChaosControl storage-fault campaigns.

## Impact

- **Core**: durability observations, recovery decisions, node admission, and repair policy.
- **Shell**: Redb transactions, synchronization, reopen, peer queries, publication, and quarantine.
- **Adapters**: Redb, capability-rooted files, Durable File Publication, and ChaosControl fixtures.
- **Evidence**: exact workload, fault, filesystem, kernel, device, candidate, and non-claim bindings.

## Dependencies

- Durable File Publication supplies one-file publication mechanics.
- Trellis supplies reviewed protocol-aware recovery decisions after publication.
- ChaosControl supplies guest flush and persistent-node fault evidence after publication.
- Transactional Reconciliation Core can supply product-neutral uncertain-persistence classification after publication.

All dependencies must use immutable published revisions. Ambient sibling paths are not product dependencies.

## Non-goals

- Do not claim that Redb, a filesystem, or hardware cannot lose data.
- Do not make direct I/O a universal requirement.
- Do not treat checksums as proof that one replica is authoritative.
- Do not treat local database repair as Raft repair.
- Do not claim whole-cluster safety, liveness, or release readiness from one campaign.
