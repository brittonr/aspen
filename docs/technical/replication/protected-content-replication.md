# Protected Content Replication

Protected replication moves verified content without treating availability as permission to reveal it. This article assumes content-addressed storage, explicit placement epochs, and the pure-core/effectful-shell boundary. The [content replication contract](../../content-replication.md) governs the extension; the discussion below explains its existing planner and transfer validation rather than prescribing a cryptosystem. Return to the [Technical companion](../README.md) for the surrounding architecture.

## Separate three questions

A replication decision needs distinct answers to three questions: which bytes represent the content, which replicas satisfy current placement, and which effects are admitted? Content stores retain byte identity, verification, transforms, protection, and local availability. The optional replication extension owns replica policy, placement, repair, handoff, and convergence status. Ordinary storage access does not activate that policy.

`ReconcileInput` supplies the manifest, inventory, peers, prior operations, and observed tick to a deterministic planner. This is a complete input model for planning, not a claim that inventory can never become stale. The [planner](../../../crates/molten-core/src/content_replication/planner.rs) compares each candidate current replica against the active generation, membership epoch, placement epoch, presence, identity verification, manifest reference, and protection flag. Matching a content reference alone is insufficient.

Source selection is intentionally a different predicate. `verified_source` requires matching content and manifest references, presence, identity verification, and protected form, but does not impose the current generation and epoch checks used for counting replicas. An old placement can therefore supply verified bytes without satisfying the new placement. Confusing source eligibility with current replica credit would make convergence appear complete before current placement had been established.

## Protection is preserved across the boundary

When no compatible source exists, the planner distinguishes a protected-form mismatch from a general lack of verified source. It does not plan an implicit decrypt-and-reencrypt repair. The transfer action carries the required protected form, and [envelope validation](../../../src/content_replication/service/validation.rs) checks it alongside operation identity, content and manifest references, source, receiver, generation, epochs, and encoded size.

These comparisons bind the received object to a receiver-owned operation. A valid-looking content reference in an unrelated envelope does not satisfy the requested operation. After envelope validation, the shell asks the content port to verify the result and checks identity admission, authorization admission, target peer, current generation and epochs, presence, and protected form on the verification observation.

This division is narrower than claiming that replication proves confidentiality. The replication layer preserves the declared protected representation and delegates content verification to its port. Key handling, transform correctness, and the soundness of the underlying protection mechanism remain content-layer concerns. Replication status contains references and bounded operational facts, not plaintext or credentials.

## Retention precedes transfer

The [execution shell](../../../src/content_replication/service/execution.rs) first acquires and validates a retention pin for a transfer, repair, or handoff. Only then does it request transport, validate a received envelope, and invoke content verification. A terminal operation is stored before its canonical operation observation is published. Status follows action execution, and the aggregate receipt comes last according to the governing contract.

The pin prevents cleanup from being interpreted as independent of an active transfer obligation. Cleanup is a separate action path: the retention port supplies matching cleanup admission, then the content port performs cleanup. The governing contract also requires content-rule authority and matching clearance, and says an active pin blocks cleanup. A transfer receipt does not double as deletion permission.

Transport outcomes remain differentiated. A received and successfully verified object produces a verified operation; cancellation, uncertainty, unavailability, and timeout are not collapsed into success. In particular, uncertainty is represented as an uncertain operation. The action history is operational evidence, not a warrant to assert that every attempted destination holds verified bytes.

## Placement and bounded repair

Receiver candidates must be available, match membership and placement epochs, have enough capacity, and not already occupy the excluded peer set. Sorting considers existing fault domains before domain and peer names. The planner also checks the selected domain coverage against the minimum required by policy. This is deterministic selection over declared facts; it is not proof that nominal fault domains are physically independent.

Resource admission is also bounded in the planner. Concurrent transfers, queue depth, and total planned encoded bytes constrain further actions. Attempt limits can defer repair. Prior verified operation identity may support reuse, while conflicting history is not silently replaced. These limits make insufficient capacity and exhausted repair visible rather than authorizing unbounded convergence work.

## Worked protected-handoff scenario

Suppose, illustratively, protected object `P` needs two current replicas across two fault domains after a placement-epoch change. Peer `old-a` has verified protected bytes from the previous epoch. Two available current peers, `new-b` and `new-c`, occupy distinct domains and have sufficient capacity. The old replica can be a source, but its existence does not reduce the required current replica count.

A transfer envelope for `new-b` then arrives with the correct content reference but `protected` set differently from the planned form. The shell rejects it before treating the content as a verified replica. An operator should not “repair” this by adjusting status or counting the source: the operation and representation did not match. If a later transfer succeeds but publication fails after operation storage, durable history and observation publication are separate reconciliation facts; failure to publish does not itself prove the transfer never happened.

## Verification and non-claims

Suggested review should compare stale-source eligibility with current-replica counting, alter the envelope protection flag, cross source and receiver identities, and exhaust byte and attempt budgets. Trace retention admission before transport and operation storage before observation publication. These are review suggestions, not checks executed for this article.

Replica counts do not prove permanent durability or global availability. Protected replication does not reveal content, authorize execution, establish publication rights, or justify cleanup by itself. Neither operation reuse nor a successful transfer establishes exactly-once external effects or production readiness.

## Sources

- [Bounded content replication](../../content-replication.md)
- [Bounded DAG synchronization](../../dag-sync.md)
- [Replica selection and bounded repair planner](../../../crates/molten-core/src/content_replication/planner.rs)
- [Transfer and cleanup execution](../../../src/content_replication/service/execution.rs)
- [Envelope, pin, and verification validation](../../../src/content_replication/service/validation.rs)
- [Technical companion](../README.md)
