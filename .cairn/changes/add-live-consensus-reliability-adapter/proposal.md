## Why

Molten has deterministic whole-system simulation, a NixOS multi-node VM rail, and an active ChaosControl consensus-conformance change. Those paths do not provide an independent live black-box history through public coordination endpoints.

OnixOS is adding a generic live reliability rail. Molten must own the product adapter, selected consistency model, public workload, recovery observations, and evidence import.

The first profile can use the service-registry coordination primitive as a single register. Writes update one endpoint ref through the admitted control plane. Reads use the public linearizable-read path.

## What Changes

- Define a live service-registry register profile and seeded operation generator over one admitted coordination key.
- Expose public setup, write, read, recovery, final-read, and teardown operations for the OnixOS adapter.
- Preserve generator identity, seed, choices, logical operation identity, retries, and uncertain client results.
- Emit ChaosControl semantic-history v2 events without reading consensus internals.
- Deploy exact Molten artifacts through the OnixOS native-service and live-reliability contracts.
- Run no-fault, process-restart, temporary-partition, heal, and recovery profiles on disposable clusters.
- Validate register linearizability with a pinned native checker and optional Jepsen-compatible reference checker.
- Import live run evidence into canonical Molten receipts without transferring authority or release claims.
- Compare simulation, ChaosControl KVM, NixOS VM, and live evidence as separate profiles.

## Impact

- **Files**: coordination generator and client surfaces, service-registry fixtures, OnixOS service integration, history adapter, evidence importer, docs, and Nix checks.
- **Dependencies**: implementation depends on archived ChaosControl semantic-history, OnixOS live-reliability, and Molten native-service contracts.
- **Testing**: pure adapter tests, public client fixtures, no-fault and fault runs, final recovery reads, checker disagreement, malformed evidence, and overclaim denial.
- **Claims**: passing evidence covers only the exact Molten, service-registry key, cluster, profile, fault, recovery, checker, and run cohort.

## Non-goals

- Do not replace the active ChaosControl SMR chain conformance package.
- Do not use internal Raft state as a black-box semantic oracle.
- Do not claim transaction serializability, queue semantics, Byzantine tolerance, production SLOs, or universal consensus correctness.
- Do not let live reliability evidence grant authority, policy, resources, provenance, deployment, retention, or release eligibility.
