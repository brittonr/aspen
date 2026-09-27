# Bounded DAG Closure Exchange

This article explains how a receiver describes, bounds, and resumes an exchange of graph metadata and referenced content. It assumes familiarity with content references and the separation between Molten's deterministic core and effectful shell. The governing [bounded DAG synchronization contract](../../dag-sync.md) remains authoritative; this article is a reasoning companion, not a new protocol. Return to the [Technical companion](../README.md) for related topics.

## Closure is relative to roots and strategy

A graph closure is not an unqualified request to copy everything a peer knows. `plan_dag_sync` first validates the supplied graph and request, derives the nodes reachable from selected roots, and constructs a deterministic topological order. It then projects that order into objects according to the explicitly selected strategy. Inventory and compatible verified progress remove objects from the fetch set. These stages are visible in the [planner](../../../crates/molten-core/src/dag_sync/planner.rs).

The distinction between reachable nodes and requested objects matters. `StemFirst` emits node references before payload references. `LeafOnly` selects payloads attached to nodes without outgoing edges. `Full`, `Resumable`, and `PeerPartitioned` walk the topological order and include each node and its associated payload when present. Repeated objects are removed while retaining first occurrence. Consequently, completion under a leaf-only request is not evidence that all interior graph metadata was transferred. The strategy is part of the statement being completed, not an optimization that can be silently substituted. See the [strategy projection](../../../crates/molten-core/src/dag_sync/validation.rs).

Bounds apply to different stages rather than to one interchangeable counter. Shape validation checks root, node, edge, and peer counts. Reachability consumes traversal steps, including popped references that may already have been visited. Topological processing checks aggregate encoded node bytes and maximum depth, and rejects cycles when its output cannot cover the reachable set. Missing-object requests have a further step-bound check. A small number of distinct nodes therefore does not alone establish that a request fits every budget.

## Resume is a fenced statement

`DagSyncProgress` repeats epoch, generation, strategy, policy, roots, schemas, and peers. `validate_progress` compares these facts with the new request and derived context, verifies that remembered objects belong to the strategy's object set, and rejects duplicate progress objects. Resume is thus reuse of evidence within a matching traversal context, not permission to reinterpret previous work under a different assignment.

The [shell](../../../src/dag_sync/service.rs) loads progress by traversal epoch. If both caller-supplied and stored progress exist but differ, it returns an error rather than choosing the more optimistic value. If the caller supplies none, the stored value becomes the planner input. This makes disagreement visible at the boundary before any new transfer is requested.

For each received response, `admit_dag_response` checks epoch and generation, finds the requested object, compares the assigned peer, rejects already-verified objects, and requires identity and authorization admission with a bounded nonzero encoded size. Successful admission produces a new progress value; it does not itself write storage or send messages.

## Effects and evidence have an order

For a received object, the shell validates the transport envelope, asks the content port to verify it, and applies the pure response transition. It publishes the canonical response observation before storing new progress, then publishes the canonical progress observation. The completion receipt is published after the transfer loop. Cancellation or deferral ends the loop with a partial result when objects remain; validation or port errors can return without a completion receipt. It would be misleading to describe every unsuccessful invocation as a recorded partial completion.

One source discrepancy limits the admission claim. The governing DAG document describes obtaining authority and resource observations before each transfer. The inspected `run_dag_sync` implementation obtains and validates them once before iterating over `plan.requests`; content verification subsequently receives the authority reference. This companion does not claim that the loop refreshes authority and resources for every object, nor does it redefine the governing contract. Per-transfer refresh remains a review discrepancy between those sources.

## Worked reasoning example

Consider an illustrative root whose node `A` references `B` and `C`, with both branches referencing `D`. Each node has an associated content reference. A full request includes the reachable metadata and payload objects, with shared objects deduplicated. Suppose two payload references already appear in inventory and one node reference appears in compatible progress. Those three objects are omitted from the new missing set; the graph still participates in reachability and bound validation.

Now suppose the transport returns the next requested object from a different assigned peer. Byte identity alone is insufficient: response admission rejects the assignment mismatch. Reassigning peers is a new traversal context, not a way to continue updating the old progress record. Under the governing contract, peer reassignment requires a new epoch. Verified local content can inform a later inventory, but old progress cannot simply be relabeled.

Finally, imagine a crash after publishing a response observation but before progress storage succeeds. The observation is evidence that one stage occurred, not proof that the durable resume record advanced. Recovery uses stored progress rather than inferring persistence from the diagnostic stream.

## Verification and limits

Suggested review should separately exercise graph rejection, strategy projection, progress compatibility, response admission, and shell publication ordering. Include a shared-descendant graph, a cycle, an over-budget traversal, a changed schema set, a duplicate response, and cancellation after admitted progress. These are proposed checks, not execution results from writing this article.

A complete receipt establishes verified availability for its requested references and context. It does not establish global convergence, trusted peer membership, permanent durability, application merge semantics, installation, execution, or publication authority. The planner operates on supplied facts without ambient network or clock effects; the truth and currentness of those facts remain boundary responsibilities.

## Sources

- [Bounded DAG synchronization](../../dag-sync.md)
- [Bounded content replication](../../content-replication.md)
- [DAG planning and response admission](../../../crates/molten-core/src/dag_sync/planner.rs)
- [Graph, strategy, and progress validation](../../../crates/molten-core/src/dag_sync/validation.rs)
- [DAG synchronization shell](../../../src/dag_sync/service.rs)
- [Technical companion](../README.md)
