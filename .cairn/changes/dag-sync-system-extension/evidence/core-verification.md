# DAG-sync core verification

Recorded on 2026-08-26.

The declared fabric dependencies are archived, including `fabric-whole-system-simulation` at `.cairn/archive/2026-08-01-fabric-whole-system-simulation`.

Implemented pure core coverage:

- nominal BLAKE3 node, root, schema, content, plan, epoch, policy, and receipt references;
- domain-neutral node, edge, root, bounds, inventory, request, response, progress, plan, and receipt DTOs;
- duplicate root, node, edge, and inventory rejection;
- unknown root and edge rejection;
- deterministic reachable-set and topological ordering;
- cycle, node, edge, root, depth, byte, peer, and step bounds;
- full, stem-first, leaf-only, resumable, and deterministic peer-partitioned strategies;
- generation, epoch, strategy, policy, and progress fencing;
- unsolicited, wrong-peer, stale, corrupt, unauthorized, and over-bound response denial;
- canonical Preserves records and domain-separated record identities.

The imperative shell now composes explicit authority, resource, transport, content verification, progress persistence, observation, and receipt ports. It requests only planned objects, verifies before progress mutation, stores each accepted step, and publishes a bounded receipt last.

Job DAGs now project output-root closure through reversed dependency edges. Artifact dependency closures project exact content identities and reject incomplete closures. These adapters do not grant install, execution, publication, or merge authority.

Bounded status readback reports roots, strategy, epoch, requested, verified, missing, peers, resources, failures, evidence references, and non-claims.

Four core tests and five shell tests passed. The shell tests cover complete receiver-driven transfer, durable per-object progress, receipt-last ordering, deferral, corruption denial, domain projections, and status output. Core and root-crate Clippy passed for all targets and features with warnings denied.

No concrete live transport, Redb adapter, system-extension lifecycle, live loopback, multiprocess, or domain activation claim is made by this stage.
