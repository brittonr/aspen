## Why

Molten has deterministic job topological ordering, artifact dependency closure, missing-ref planning, and local loopback copy behavior. It lacks one generic bounded protocol for discovering and synchronizing DAG metadata and referenced content across peers without embedding job, Forge, commit, or transport semantics in the traversal core.

A DAG synchronization system extension should compose pure traversal primitives with fabric content and transport adapters.

## What Changes

- Add generic canonical DAG node, edge, root, traversal-bound, sync-request, sync-plan, response, and receipt records.
- Keep cycle detection, deterministic traversal, visited-set handling, topological ordering, missing-set calculation, and strategy planning pure and adapter-neutral.
- Add a receiver-driven extension protocol for stem-first, leaf-only, resumable, and deterministic peer-partitioned fetch strategies.
- Fetch node metadata and payload refs through admitted transport and content-store ports and reject unsolicited, stale, cyclic, corrupt, or over-bound responses.
- Permit job DAGs, artifact closures, and future extension-owned DAGs to use the protocol without importing their domain semantics into the fabric.
- Add same-core simulation/live conformance, restart/resume, cancellation, and first-divergence evidence.

## Impact

- **Files**: generic DAG primitives, a DAG-sync system extension, content/transport bindings, job and artifact integration adapters, operator readback, fixtures, and a new `dag-sync` accepted spec.
- **Testing**: deterministic traversal and strategy properties, live/sim protocol parity, partial resume, cycles, depth/node exhaustion, manifest drift, unexpected nodes, corruption, cancellation, and authorization denial.
- **Safety**: synchronization proves only verified receipt of requested content refs; it does not grant install, execution, merge, membership, provenance, or application-level conflict authority.
- **Licensing**: Aspen `main` DAG algorithms and tests are design references; direct implementation reuse requires a compatible source license.
