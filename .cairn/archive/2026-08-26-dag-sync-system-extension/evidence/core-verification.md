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
- generation, epoch, strategy, policy, root, schema, peer-assignment, and progress fencing;
- unsolicited, wrong-peer, stale, corrupt, unauthorized, and over-bound response denial;
- canonical Preserves records and domain-separated record identities.

The imperative shell now composes explicit authority, resource, transport, content verification, progress persistence, observation, and receipt ports. It requests only planned objects, verifies before progress mutation, stores each accepted step, and publishes a bounded receipt last.

Job DAGs now project output-root closure through reversed dependency edges. Artifact dependency closures project exact content identities and reject incomplete closures. These adapters do not grant install, execution, publication, or merge authority.

Bounded status readback reports roots, strategy, epoch, requested, verified, missing, peers, resources, failures, evidence references, and non-claims.

Eight core tests and ten shell tests passed. The tests cover bounded traversal, input-order properties, reference spelling, complete transfer, durable progress, restart, receipt-last ordering, cancellation, partition, corruption, authority denial, peer reassignment, domain projections, and status output.

The same core passed with the existing deterministic transport adapter and the existing live Iroh loopback adapter. DAG code does not import Iroh backend types.

The focused Octet workspace passed with zero findings, warnings, and errors. Core and root-crate Clippy passed for all targets and features with warnings denied.

The full `molten` and `molten-core` all-target, all-feature test command passed. It included 1,305 root-library tests and 207 core tests.

The all-target, all-feature Clippy command passed with warnings denied. `nix flake check --no-build --builders ''` also passed.

Strict Cairn validation and the proposal, design, and tasks gates passed. The focused Octet command passed after the final Rust changes.

The Nix evaluation first found a stale `cairn/archive` path. The path now points to the existing `.cairn/archive` directory.

This stage makes no multiprocess, global convergence, domain activation, installation, execution, publication, or merge claim.
