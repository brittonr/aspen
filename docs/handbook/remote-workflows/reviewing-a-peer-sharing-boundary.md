# Reviewing a peer-sharing boundary

Mode: Review checklist

Use this checklist before approving an integration that advertises, fetches, imports, or applies peer-supplied content. Review a concrete call path and its captured inputs, not a subsystem name. Each accepted answer should point to code and evidence; mark unobserved behavior as unverified rather than replacing it with a plausible architectural story.

This document is source-checked, not a runtime verification report. No commands were executed for this batch. Return to the [Handbook](../README.md). The [assertion lifetime companion](../../technical/dataspaces/assertion-lifetimes-and-cleanup.md) and [DAG closure companion](../../technical/replication/bounded-dag-closure-exchange.md) explain the underlying models without granting implementation authority.

## Identify the actual boundary

- [ ] Is the reviewed path reproduction-bundle exchange, chain-segment exchange, gossip delivery, inventory pull, locator admission, or DAG synchronization? Attach its entry function and caller. Reject a review that silently substitutes the local exchange fixture for a live deployment.
- [ ] Are pure validation, local filesystem effects, and live transport effects labeled separately? The [local bundle helper](../../../src/iroh/parts/exchange/p000/body.rs) performs storage effects even though no live peer participates.
- [ ] Does the integration name who owns destination paths, expected references, trust context, and receiver policy? A peer's announcement must not choose these implicitly for the receiver.
- [ ] Are claimed outcomes scoped to availability, admission, or application separately? A receipt is evidence, not execution, installation, publication, membership, or merge authority.

## Require identity evidence at each conversion

- [ ] Does the receiver retain an independently selected expected reference rather than accepting a ticket's advertised identity as its entire policy?
- [ ] Where are canonical Preserves bytes produced or parsed, and where is BLAKE3 checked? Identify the exact comparison before import. Rust memory layout, debug text, and transport endpoint identity are not replacements.
- [ ] Are text output files distinguished from canonical stored bytes? A text rendering can represent the same value without having the canonical byte hash.
- [ ] For federation resources, does actual artifact kind match the advertised resource type as well as the canonical hash? The [pull implementation](../../../src/federation/parts/mod/p002/body.rs) checks both.
- [ ] Does the security description accurately characterize the inspected federation signature helper? It hashes payload and supplied context/key material; do not claim a public-key verification protocol from its function name.

## Verify receiver selection and boundedness

- [ ] Are roots, strategy, schemas, policy, epoch, generation, and peers preserved with DAG progress? Require a changed-context rejection example, not only successful resume with identical inputs.
- [ ] Is peer reassignment represented by a new epoch rather than edited old progress? Ask how verified bytes enter inventory without laundering old assignment evidence.
- [ ] Are resource and import limits explicit in the selected federation policy? Empty allowed-type lists do not mean deny-all, and `allow_all()` is not a safe deployment policy simply because it is available.
- [ ] Is response order compared with the receiver's fetch order? The [remote traversal helper](../../../src/remote/parts/dataspace/p005/body.rs) can reject a response with the right set in the wrong sequence.
- [ ] Does the reviewer distinguish the older reference-list traversal helper from full graph validation? Its selection helper filters supplied roots by visited references; naming a kind `artifact-closure` does not itself demonstrate recursive closure discovery.

## Check receiving-session lifetimes

- [ ] Does the caller choose an open session whose receiver and topic match the delivery context? The [session admission function](../../../src/remote/parts/dataspace/p006/body.rs) checks declared-owner existence/state, but does not itself compare those context fields with the envelope.
- [ ] Are evidence lists backed by resolved, substantively admitted artifacts? Nonempty canonical references and membership checks do not prove every referenced capability or policy is valid now.
- [ ] Does an admitted assertion identify its receiving owner in both evidence and runtime state? Use the [session fixture](../../../src/remote/parts/dataspace/tests/m000/p003/body.rs) as the concrete evidence shape.
- [ ] Is actual disconnect handling connected to closure? The live gossip event helper ignores neighbor notifications as deliveries; that alone neither invokes nor proves cleanup.
- [ ] Do closure evidence, observer events, and retained state agree? Confirm late delivery to a closed owner leaves state unchanged, and reconnect uses a new generation rather than resurrecting historical facts.

## Review failure effects, not just final decisions

- [ ] Can the reviewer identify all writes before a returned error? Explicit bundle output precedes optional ledger imports; federation imports happen resource by resource.
- [ ] Does recovery reconcile imported, skipped, and denied resources before another effectful attempt? Reject unconditional retries, deletion of state, or relaxed admission as repair techniques.
- [ ] Are locator reachability and sampled possession claims kept diagnostic? [Locator admission](../../../src/federation/parts/mod/p004/body.rs) requires additional evidence categories; a reachable peer alone cannot authorize import.
- [ ] Are replay results separated from current liveness and exactly-once claims? Unknown or closed session replay is diagnostic-only in the session-aware helper.

## Worked review decision

Consider an integration that imports two permitted inventory resources, denies a third, then treats the overall failure as proof that the destination is unchanged. Reject that recovery design. The pull loop can already have imported the first two before constructing a failed receipt. Required evidence is the original inventory and policy, returned resource partitions when available, and destination reconciliation before any authorized continuation.

Likewise, do not approve a readiness dashboard that replays a closed session's old assertion to restore “healthy” status. Require a fresh admitted session and peer reassertion. Historical evidence can explain what happened; it cannot manufacture a live owner.

## Record unresolved source-review limits

List missing integration evidence explicitly. Reference-list validators do not prove full authority resolution; session APIs do not prove live disconnect wiring; fixture success does not prove production support. These are source-review limits, not reproduced incidents. Approval should state precisely which boundary has evidence and which claims remain excluded.

## Sources

- [Handbook](../README.md)
- [Governing architecture](../../architecture.md)
- [DAG synchronization contract](../../dag-sync.md)
- [Assertion lifetime companion](../../technical/dataspaces/assertion-lifetimes-and-cleanup.md)
- [DAG closure companion](../../technical/replication/bounded-dag-closure-exchange.md)
- [Federation policy fields](../../../src/federation/parts/mod/p000/body.rs)
- [Federation signature implementation](../../../src/federation/parts/mod/p003/body.rs)
- [Traversal reference selection](../../../src/remote/parts/dataspace/p009/body.rs)
- [Session closure and replay](../../../src/remote/parts/dataspace/p007/body.rs)
