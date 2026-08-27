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

Four core tests and two canonical shell-record tests passed. Core and root-crate Clippy passed for all targets and features with warnings denied.

No transport, storage, progress persistence, extension lifecycle, live loopback, multiprocess, or domain activation claim is made by this stage.
