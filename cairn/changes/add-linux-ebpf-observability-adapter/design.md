## Context

The accepted `fabric-observability` capability already separates canonical observation and health logic in `molten-core` from live exporter, tracing, filesystem, and operator effects in the root runtime. It bounds descriptors, labels, events, queues, snapshots, scans, findings, and diagnostics; preserves freshness and claim scope; and denies telemetry as authority.

Linux eBPF can provide useful host-level facts that application instrumentation cannot see directly, but it introduces privileged lifecycle, kernel-cohort, loss, ordering, and confidentiality hazards. OnixOS is the system owner for loading admitted BPF Packs and can expose a generation-bound read-only observation endpoint. Molten should consume that endpoint rather than becoming a second loader.

## Decisions

### 1. Consume a host endpoint; do not load BPF

**Choice:** The live adapter opens only an explicitly supplied capability-rooted read endpoint produced by the OnixOS BPF Pack runtime adapter. It cannot issue BPF load/attach/link-update/detach operations, write maps, traverse ambient bpffs paths, inherit loader credentials, or discover arbitrary programs. Missing endpoint capability yields unavailable, not an ambient fallback.

**Rationale:** Privileged BPF lifecycle belongs to machine orchestration, while Molten owns observation semantics.

### 2. Bind every source to an exact generation and cohort

**Choice:** The profile and endpoint handshake bind machine, Onix generation, BPF Pack/object/program, kernel-build/BTF, endpoint schema, adapter, cgroup/resource scope, and loss-counter identities. Drift, replacement, stale generation, or scope mismatch closes the current observation epoch and requires fresh admission.

**Rationale:** Records from different programs, kernels, or generations must not merge into one apparently continuous series.

### 3. Author mappings and limits in typed Nickel

**Choice:** Extend the fabric observability Nickel contract with a closed Linux eBPF adapter profile covering admitted signal families, field-to-descriptor mappings, cgroup/resource scope, window and freshness policy, event detail policy, queue/record/byte/series/cardinality bounds, loss behavior, redaction, required/optional status, and non-claims. Runtime Rust consumes the checked deterministic export.

**Rationale:** Kernel observations need reviewable finite mappings and named bounds rather than implicit field discovery.

### 4. Start with bounded local operational signals

**Choice:** The initial profile vocabulary covers cgroup-scoped process lifecycle aggregates, scheduler runnable/latency aggregates, block-I/O operations/latency aggregates, network byte/packet/error aggregates, pressure or saturation indicators supplied by the admitted program, and adapter/loss health. Raw command lines, payloads, packet contents, private paths, socket addresses, peer identities, and unconstrained kernel identifiers are not admitted fields.

**Rationale:** These signals have high operational value while avoiding a general-purpose host surveillance interface.

### 5. Keep validation and projection pure

**Choice:** `molten-core` receives already-read endpoint metadata, bounded records, supplied observation/freshness ticks, and the checked profile. Pure functions validate schema/cohort/scope/sequence, enforce mappings and bounds, project canonical descriptors/samples/events/status, and classify epoch completion. The shell only opens, polls, bounds, copies, closes, and reports I/O outcomes.

**Rationale:** Record semantics and failure decisions must be testable without Linux, eBPF, clocks, or an OnixOS host.

### 6. Do not invent a deterministic cross-CPU total order

**Choice:** Records retain source CPU/stream identity and sequence facts where present. Delivery order and kernel monotonic timestamps are observation metadata, not deterministic semantic order. The adapter emits canonical aggregates over complete declared windows and may emit bounded events only when their source-local order and scope are explicit. Concurrent cross-source events remain a partial order; sorting by timestamp cannot upgrade them to deterministic evidence.

**Rationale:** eBPF delivery can interleave across CPUs and scheduler contexts even when application behavior is equivalent.

### 7. Make loss and discontinuity first-class

**Choice:** Each epoch reconciles producer attempts/submissions/drops, endpoint sequence gaps, userspace malformed/unknown/over-bound drops, queue pressure, and adapter cancellation where the endpoint supplies those facts. Non-zero or unavailable required accounting marks the affected window partial or unavailable. No health/readiness policy may treat a partial required window as complete.

**Rationale:** Silent loss turns plausible metrics into false evidence.

### 8. Reuse fabric confidentiality and cardinality enforcement

**Choice:** eBPF-derived descriptors, labels, events, and snapshots pass through the existing canonical validation and redaction policy. The adapter exposes only admitted finite label vocabularies and canonical resource refs. Unmapped fields, raw identifiers, secret-like material, payloads, and bound excess are denied or represented by approved redacted markers and typed adapter outcomes.

**Rationale:** Kernel access must not weaken the fabric's existing data-minimization boundary.

### 9. Supervise epochs without semantic side effects

**Choice:** Start, poll, generation change, endpoint failure, cancellation, close, and restart produce bounded adapter status. Shutdown closes the read endpoint and records final accounting; it does not detach programs or delete host state. OnixOS remains responsible for links, pins, maps, replacement, rollback, and cleanup.

**Rationale:** Reader cleanup and privileged program cleanup are different ownership domains.

### 10. Preserve evidence and health scope

**Choice:** Canonical snapshots and adapter status link exact profile, endpoint, generation, cohort, scope, window, accounting, and supporting Onix receipts with domain-separated BLAKE3 refs. They remain local observation evidence and cannot grant capability, authorize repair, prove complete telemetry or application correctness, establish cluster readiness, or satisfy production/release claims.

**Rationale:** Better host visibility must not silently strengthen Molten's authority or readiness claims.

## Risks / Trade-offs

- The adapter depends on a compatible OnixOS endpoint and cannot provide the same live source on generic hosts. Explicit unavailable status and deterministic simulation fixtures preserve portability.
- Window aggregation sacrifices raw event detail. This is intentional to control ordering ambiguity, cardinality, and confidentiality.
- Producer-side loss counters themselves can be unavailable or wrong. Admission binds their schema and identity, while missing accounting prevents completeness claims rather than pretending certainty.
- Kernel and program schema evolution requires coordinated profile updates. Exact cohort binding prevents silent cross-version merges.

## Non-Goals

- Loading, attaching, pinning, replacing, rolling back, detaching, or cleaning BPF programs.
- Defining Onix BPF Pack semantics, target authorization, or kernel compatibility policy.
- Exposing raw packet payloads, command lines, private paths, credentials, mutable maps, or arbitrary host tracing to extensions.
- Replacing application tracing, deterministic simulation observations, Prometheus/OpenTelemetry export, or read-only integrity scans.
- Proving service correctness, complete telemetry, causal total order, cluster truth, security, or release eligibility.
