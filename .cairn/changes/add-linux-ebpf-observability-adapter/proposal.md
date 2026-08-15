## Why

Molten already defines canonical bounded observations, pure aggregation and health decisions, and versioned adapter contracts. Its live Linux runtime still lacks an eBPF-backed adapter for cgroup-scoped process, scheduler, block-I/O, network, pressure, event-loss, and adapter-health facts. Adding ad hoc loaders or backend-specific objects inside extensions would bypass the accepted fabric boundary and duplicate privileged lifecycle ownership that belongs to OnixOS.

Molten should add a Linux observation adapter that consumes an explicitly exported, generation-bound, read-only host endpoint from the OnixOS BPF Pack runtime adapter. It must translate bounded host records into existing canonical observations without loading programs, attaching hooks, granting authority, or claiming that telemetry proves service correctness.

## What Changes

- Add a typed Linux eBPF observation profile binding the Molten source/scope, exact Onix generation and BPF Pack cohort, endpoint schema, admitted signal families, descriptor mapping, windows, freshness, redaction, and named bounds. r[linux_ebpf_observability.profile]
- Admit only a current read-only endpoint whose machine, generation, pack, program, kernel/BTF cohort, observation schema, and declared scope match the profile. r[linux_ebpf_observability.admission]
- Keep BPF load, attach, replace, rollback, detach, pin, and cleanup outside Molten; the adapter receives observation capability only. r[linux_ebpf_observability.boundary]
- Implement pure validation and deterministic projection from bounded host records into existing canonical metric samples, events, adapter status, and snapshots. r[linux_ebpf_observability.projection]
- Treat cross-CPU records as a bounded partial-order source, prefer canonical window aggregates, and never infer deterministic total order from host timestamps or delivery order. r[linux_ebpf_observability.ordering]
- Make drops, sequence gaps, queue pressure, stale generations, unavailable endpoints, unsupported signals, malformed records, cancellation, and shutdown/cleanup failures explicit. r[linux_ebpf_observability.failure]
- Enforce existing cardinality, confidentiality, redaction, freshness, health-scope, and non-claim rules before records enter fabric observability. r[linux_ebpf_observability.confidentiality]
- Add positive and negative pure, fixture, simulation-parity, host-endpoint, restart, loss, redaction, and cleanup conformance. r[linux_ebpf_observability.verification]

## Impact

- **Pure core**: extends `crates/molten-core/src/fabric_observability/` with Linux eBPF record/profile validation and canonical projection only.
- **Imperative shell**: extends `src/fabric_observability/` with a capability-rooted bounded reader and supervision wiring; no BPF loader or mutable map/link handle is introduced.
- **Configuration/docs**: extends `docs/fabric-observability/` Nickel contracts, profiles, fixtures, generated runtime input, and operator documentation.
- **OnixOS dependency**: consumes the read-only observation handoff from `realize-linux-bpf-pack-adapter`; OnixOS remains privileged lifecycle owner.
- **Compatibility**: non-Linux deployments and deployments without the endpoint remain supported with the adapter disabled or explicitly unavailable.
- **Claims**: observations describe one exact local machine/cgroup/generation/cohort and freshness window. They do not grant capabilities, prove service semantics, establish cluster truth, prove complete telemetry, or satisfy release eligibility.
