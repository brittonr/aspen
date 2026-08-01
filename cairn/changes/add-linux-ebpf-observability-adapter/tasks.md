## Phase 1: Profile and pure core

- [ ] [depends:onixos.realize-linux-bpf-pack-adapter] Add exact positive and negative read-only endpoint handoff fixtures binding machine, generation, BPF Pack/program, kernel/BTF, schema, scope, and loss accounting. r[linux_ebpf_observability.admission] r[linux_ebpf_observability.boundary]
- [ ] [serial] Extend the typed fabric observability Nickel contracts with Linux eBPF signal mappings, windows, freshness, event detail, queue/record/byte/cardinality bounds, loss policy, required status, redaction, and non-claims. r[linux_ebpf_observability.profile]
- [ ] [serial] Define bounded endpoint metadata, raw record, accounting, epoch, and projection DTOs in `molten-core`. r[linux_ebpf_observability.profile] r[linux_ebpf_observability.projection]
- [ ] [serial] Implement pure endpoint/cohort/scope admission, record validation, descriptor mapping, window aggregation, partial-order handling, loss reconciliation, epoch completion, and canonical projection. r[linux_ebpf_observability.admission] r[linux_ebpf_observability.projection] r[linux_ebpf_observability.ordering] r[linux_ebpf_observability.failure]
- [ ] [parallel] Add positive process/scheduler/block/network/pressure/adapter-health aggregate fixtures and deterministic canonicalization assertions. r[linux_ebpf_observability.verification]
- [ ] [parallel] Add negative unknown schema/field/signal, cohort/scope/generation mismatch, duplicate/sequence gap, cross-CPU total-order claim, overflow, stale window, missing accounting, and over-bound fixtures. r[linux_ebpf_observability.verification]

## Phase 2: Linux read adapter

- [ ] [serial] Implement a thin capability-rooted Linux endpoint reader that performs bounded open/poll/copy/close effects and delegates every semantic decision to `molten-core`. r[linux_ebpf_observability.boundary] r[linux_ebpf_observability.failure]
- [ ] [serial] Wire adapter supervision for start, endpoint handshake, poll, generation rollover, cancellation, final accounting, close, and restart without BPF mutation operations. r[linux_ebpf_observability.boundary] r[linux_ebpf_observability.failure]
- [ ] [parallel] Add structural guards preventing loader libraries, BPF syscalls, writable map/link handles, ambient bpffs traversal, and privileged credentials from entering Molten or extension APIs. r[linux_ebpf_observability.boundary]
- [ ] [parallel] Add positive host-endpoint and deterministic recording-adapter parity tests plus negative unavailable, permission, timeout, backpressure, malformed record, cancellation, close failure, and generation-change tests. r[linux_ebpf_observability.verification]

## Phase 3: Fabric integration and confidentiality

- [ ] [serial] Project admitted windows into existing canonical metric samples, bounded events, adapter status, snapshots, health inputs, and evidence refs without backend-specific identities. r[linux_ebpf_observability.projection]
- [ ] [serial] Enforce existing descriptor, label, series, event, queue, snapshot, freshness, scope, and diagnostics bounds for every eBPF-derived observation. r[linux_ebpf_observability.confidentiality]
- [ ] [parallel] Add redaction and denial fixtures for command lines, payloads, packet contents, private paths, socket/peer identifiers, secret-like text, raw kernel identifiers, and unmapped fields. r[linux_ebpf_observability.confidentiality] r[linux_ebpf_observability.verification]
- [ ] [parallel] Add health/readiness assertions proving partial, stale, lost, or unavailable required windows cannot satisfy complete local readiness and local observations cannot satisfy cluster or release scope. r[linux_ebpf_observability.failure] r[linux_ebpf_observability.confidentiality]

## Phase 4: Wiring and closeout

- [ ] [serial] Wire checked generated profile input, adapter enablement, capability delivery, status composition, and optional/required service policy into the Molten node shell. r[linux_ebpf_observability.profile] r[linux_ebpf_observability.boundary]
- [ ] [parallel] Document OnixOS ownership, supported signal/cohort scope, endpoint setup, aggregation/order limits, event loss, privacy, debugging, cleanup ownership, and non-claims. r[linux_ebpf_observability.confidentiality]
- [ ] [parallel] Add evidence-role guards proving telemetry cannot grant capability, authorize repair, detach programs, establish application correctness or cluster truth, or satisfy production/release gates. r[linux_ebpf_observability.confidentiality]
- [ ] [serial] Run focused `molten-core`, Nickel fixture/export, live adapter, simulation-parity, redaction, supervision, and integration positive/negative checks. r[linux_ebpf_observability.verification]
- [ ] [serial] Run Cairn validation and proposal/design/tasks gates; sync and archive only with current bounded host-endpoint evidence including explicit loss and cleanup outcomes. r[linux_ebpf_observability.verification]
