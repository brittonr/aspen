# Linux eBPF Observability Adapter Specification Delta

## ADDED Requirements

### Requirement: Linux eBPF observation profiles are typed and bounded

r[linux_ebpf_observability.profile] Molten MUST define a typed Linux eBPF observation profile binding exact source/scope and Onix generation/cohort identities, endpoint schema, admitted signal families, canonical descriptor mappings, window/freshness policy, event detail, redaction, named queue/record/byte/cardinality bounds, loss behavior, required status, and non-claims.

#### Scenario: Complete bounded profile is admitted

- GIVEN a profile declares one exact local source and cohort, closed mappings, finite bounds, loss behavior, and non-claims
- WHEN profile validation runs
- THEN it MAY proceed to endpoint admission
- AND later observations MUST remain bound to that profile identity.

#### Scenario: Profile permits ambient or unbounded collection

- GIVEN a profile omits cohort/scope identity, uses open-ended fields or labels, lacks finite bounds or loss policy, or permits arbitrary host tracing
- WHEN profile validation runs
- THEN it MUST be denied before the endpoint is opened.

### Requirement: Endpoint admission binds exact generation and cohort

r[linux_ebpf_observability.admission] Molten MUST admit only an explicitly supplied read-only endpoint whose machine, Onix generation, BPF Pack/object/program, kernel-build/BTF, endpoint schema, adapter, cgroup/resource scope, and loss-counter identities match the checked profile and remain current.

#### Scenario: Exact current endpoint is admitted

- GIVEN every endpoint handshake identity and declared scope matches the profile
- WHEN admission runs
- THEN the adapter MAY begin a new bounded observation epoch
- AND emitted observations MUST retain those identities through evidence refs.

#### Scenario: Endpoint is stale or mismatched

- GIVEN generation, pack, program, kernel/BTF, schema, scope, adapter, or loss accounting differs or changes during collection
- WHEN admission or epoch validation runs
- THEN the current epoch MUST close as partial, stale, unavailable, or denied
- AND records from different cohorts MUST NOT merge into one continuous series.

### Requirement: Molten receives observation capability only

r[linux_ebpf_observability.boundary] The Linux adapter MUST be limited to capability-rooted bounded endpoint read/poll/close effects and MUST NOT load, attach, replace, roll back, detach, pin, clean, or discover BPF programs; write maps; traverse ambient bpffs state; or expose privileged handles and credentials to extensions.

#### Scenario: Read-only collection proceeds

- GIVEN an admitted endpoint read capability is supplied
- WHEN the shell polls within its declared bounds
- THEN it MAY copy records into the pure projection core
- AND OnixOS MUST remain owner of program lifecycle and host cleanup.

#### Scenario: Loader or writable handle is requested

- GIVEN adapter or extension code requests BPF mutation, loader access, a writable map/link handle, ambient discovery, or privileged credentials
- WHEN boundary conformance runs
- THEN activation MUST be denied
- AND no ambient fallback MAY be used.

### Requirement: Record validation and canonical projection are pure

r[linux_ebpf_observability.projection] Molten MUST validate endpoint metadata and bounded records, map admitted fields, aggregate complete windows, classify epochs, and project canonical metric samples, events, adapter status, and snapshots through pure functions over in-memory profile, record, accounting, and supplied freshness facts.

#### Scenario: Complete admitted window projects canonically

- GIVEN valid bounded process, scheduler, block-I/O, network, pressure, or adapter-health records complete a declared window with matching accounting
- WHEN pure projection runs
- THEN equivalent inputs MUST produce identical canonical observations independent of endpoint framing or exporter formatting.

#### Scenario: Record contains unknown behavior-affecting data

- GIVEN a record has an unknown schema, signal, field, mapping, scope, discriminant, or value shape
- WHEN pure validation runs
- THEN it MUST deny or classify the record explicitly rather than discard or reinterpret it silently.

### Requirement: Cross-source ordering remains partial

r[linux_ebpf_observability.ordering] Molten MUST preserve available source identity and source-local sequence facts, MUST treat delivery order and host timestamps as observation metadata only, and MUST NOT claim deterministic total order across CPUs or producer streams; canonical aggregates MUST use complete declared windows and deterministic keys.

#### Scenario: Equivalent concurrent records arrive differently

- GIVEN two complete windows contain equivalent per-source facts but cross-CPU delivery interleaves differently
- WHEN canonical window aggregation runs
- THEN their aggregate identities MUST match when the profile declares those facts order-insensitive
- AND no host timestamp sort may be presented as causal order.

#### Scenario: Consumer requests exact event-sequence proof

- GIVEN a multi-source epoch lacks an admitted causal or total-order relation
- WHEN a consumer requests deterministic event-by-event equivalence
- THEN the adapter MUST deny that claim or provide only source-local/aggregate evidence.

### Requirement: Loss, discontinuity, and adapter failure are explicit

r[linux_ebpf_observability.failure] The adapter MUST classify producer attempts/submissions/drops where supplied, sequence gaps, malformed/unknown/over-bound userspace drops, queue pressure, stale generation, endpoint unavailable, permission denial, timeout, unsupported signal, cancellation, restart, and close/final-accounting failure, and a required partial window MUST NOT satisfy complete health or readiness.

#### Scenario: Complete zero-loss window closes

- GIVEN required accounting reconciles, no sequence gap or adapter drop is observed, and final close succeeds within bounds
- WHEN epoch completion runs
- THEN the window MAY be marked complete for its exact local scope.

#### Scenario: Loss accounting is non-zero or unavailable

- GIVEN records were dropped, a sequence gap exists, required accounting is missing, queue bounds were exceeded, or final accounting cannot be read
- WHEN epoch completion runs
- THEN the affected result MUST be partial, degraded, unavailable, or denied
- AND it MUST NOT silently retain complete status.

### Requirement: eBPF-derived data preserves confidentiality and claim scope

r[linux_ebpf_observability.confidentiality] All eBPF-derived descriptors, labels, events, status, and snapshots MUST pass existing fabric cardinality, redaction, freshness, health-scope, evidence, and non-claim validation; raw command lines, payloads, packet contents, private paths, socket addresses, peer identities, secret material, mutable handles, and unconstrained kernel identifiers MUST be denied or transformed only by an approved finite rule.

#### Scenario: Bounded aggregate is safe to expose

- GIVEN a canonical aggregate uses admitted finite labels, redacted resource refs, current freshness, and local claim scope
- WHEN fabric validation runs
- THEN it MAY enter existing export and health adapters.

#### Scenario: Telemetry is used as authority or broader readiness

- GIVEN an observation is presented as capability, repair or BPF lifecycle authority, application correctness, cluster truth, complete telemetry, security proof, or production/release readiness
- WHEN downstream admission runs
- THEN the scope promotion MUST be denied.

### Requirement: Linux eBPF observation has positive and negative conformance

r[linux_ebpf_observability.verification] Molten MUST include positive and negative profile, endpoint/cohort, pure projection, ordering, loss, cardinality, redaction, live/simulation parity, supervision, restart, close, evidence-role, and non-Linux/unavailable conformance.

#### Scenario: Conforming host adapter passes

- GIVEN a supported exact Onix endpoint emits bounded admitted records and complete accounting
- WHEN shared conformance runs
- THEN canonical projection, status, restart, close, redaction, and claim-scope checks MUST pass for the declared local cohort.

#### Scenario: Host endpoint is unavailable

- GIVEN the platform is non-Linux, the Onix endpoint is absent, or required capability delivery is unavailable
- WHEN adapter startup runs
- THEN it MUST remain disabled or report unavailable according to profile
- AND deterministic fixtures or simulation success MUST NOT count as live eBPF evidence.
