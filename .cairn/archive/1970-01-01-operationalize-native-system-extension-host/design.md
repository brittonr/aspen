# Design: Native system-extension service host

## Context

Molten's system-extension core already models canonical manifests, callbacks, lifecycle phases, generations, resources, effects, checkpoints, migrations, status, and evidence.

The executable fixture provides in-process and sandboxed component witnesses. It rejects the native-process fixture profile and does not install a long-running operator service.

The missing layer is an imperative node composition that invokes an admitted native executable through the same callback contract and routes its typed effects.

## Success Contract

Completion requires one separately executed extension that can install, activate, accept a request, request effects, checkpoint, restart, recover, drain, and stop.

Every callback must bind the exact manifest, executable artifact, execution profile, instance, service, generation, sequence, deadline, input, state, and port set.

False completion includes a new fixture-only path, ambient child authority, direct process calls outside the execution port, volatile-only instance state, hidden retries, or workload-specific node branches.

## Decisions

### Decision: Use one process per callback

**Choice:** Implement `NativeProcessSystemExtensionExecutor` as a bounded one-shot callback invocation.

The host sends one canonical callback envelope on standard input. The executable returns one canonical callback outcome on standard output.

The execution port clears the environment, applies named limits, captures bounded diagnostics, and owns teardown. The callback executable does not stay resident.

**Rationale:** One-shot execution fits the reviewed Bounded Exec mechanism and avoids a hidden interactive child protocol.

### Decision: Admit executable artifacts before instance creation

**Choice:** Installation binds exact executable bytes, artifact kind, target, execution profile, provenance, source gate, policy, authority, resources, dependency closure, and materialization receipts.

The shell resolves bytes to an executable capability only after pure admission. Host paths stay diagnostic and non-canonical.

**Rationale:** Artifact possession and filesystem location do not grant code-execution authority.

### Decision: Keep extension semantics outside the node core

**Choice:** The native host understands only the canonical callback, lifecycle, state, effect, and status contracts.

A static or packaged extension supplies its own deterministic service state machine. Molten does not branch on Kiln or another service type.

**Rationale:** The fabric must remain workload-neutral.

### Decision: Persist intent before callbacks and effects

**Choice:** Persist callback intent before process start. Persist approved effect intent before effect routing.

After each definitive observation, persist the callback or effect result before delivering the next service event. Unknown outcomes remain unresolved records.

**Rationale:** Recovery needs to distinguish work that never started from work that may have executed.

### Decision: Durable instance state is explicit

**Choice:** Store instance manifest, active generation, lifecycle state, callback sequence, checkpoint ref, unresolved callbacks, unresolved effects, resource use, and last evidence refs.

Recovery validates the executable, manifest, profile, state schema, checkpoint, generation, and port bindings before invoking `recover`.

**Rationale:** Volatile host state cannot support safe restart claims.

### Decision: Route effects through exact ports

**Choice:** Validate callback output before releasing typed effects. Route each effect through the exact binding in the active manifest snapshot.

Unknown, optional-missing, incompatible, stale, disabled, or over-authorizing ports deny. No available port becomes a fallback candidate.

**Rationale:** Callback code cannot mint authority or select infrastructure.

### Decision: Separate callback completion from effect completion

**Choice:** A callback can complete with approved pending effects. Each effect completion enters through its own generation-fenced event.

The extension decides how an effect observation changes its semantic state during a later callback. The host does not infer service success.

**Rationale:** External effects can complete after the callback process exits.

### Decision: Expose a canonical service ingress

**Choice:** Register one versioned service protocol and ALPN per admitted extension service profile.

Ingress binds endpoint, peer, service, manifest, active generation, request, authority, policy, resource, framing, and acknowledgement identities. Transport acceptance does not prove callback acceptance.

**Rationale:** Applications need a stable service boundary without transport handles.

### Decision: Make recovery explicit

**Choice:** Startup inventories unresolved callbacks and effects. It classifies each as not-started, running-observed, terminal, unknown, or stale.

The host invokes `recover` only after evidence and binding admission. It never repeats an unknown side effect automatically.

**Rationale:** Process or transport failure can occur after externally visible effects.

### Decision: Start with an explicit pilot class

**Choice:** Label the first native-process service profile as local live pilot.

Promotion requires separate security, sandbox, reliability, deployment, and release evidence. Same-host separate-process evidence cannot satisfy cross-host or production claims.

**Rationale:** A real process boundary is stronger than a fixture but weaker than a production deployment.

## Required Flow

```text
operator install
  -> manifest, executable, provenance, authority, resource, and port admission
  -> durable instance record
  -> activate lifecycle
  -> persist callback intent
  -> bounded native callback process
  -> validate callback outcome
  -> persist approved effect intent
  -> route exact fabric effects
  -> persist effect observations
  -> generation-fenced completion callback
  -> checkpoint and status evidence
```

## Operator Surface

The initial shell exposes bounded operations for install, start, request, status, recover, drain, stop, and remove.

Every mutation is previewable where practical. Existing active instances, stale manifests, incompatible state, unresolved effects, or missing teardown evidence block removal.

Status reports lifecycle phase, generation, profile, checkpoint, callback counts, unresolved operation counts, resource use, health, and evidence refs. It omits secrets, raw paths, process identifiers, and backend handles.

## Test Design

Pure tests cover manifest linkage, state transitions, generation fencing, callback ordering, effect admission, restart classification, and removal blockers.

Executor tests cover canonical input and output, cleared environment, oversized bytes, timeout, cancellation, nonzero exit, malformed output, output flood, and teardown failure.

Service tests cover install, activate, request, pending effect, effect completion, checkpoint, restart, recover, drain, shutdown, and removal.

Negative tests cover stale generations, substituted executables, missing provenance, incompatible ports, missing authority, resource exhaustion, duplicate callbacks, unknown outcomes, and fallback attempts.

A separate-process parent harness observes child start and terminal state. Offline verification checks every canonical artifact and index member.

## Risks and Trade-offs

- Per-callback process startup increases latency and host load.
- Native execution is not a sandbox. Deployment policy must restrict admitted executables and host authority.
- Callback success can precede effect completion. Operators must inspect unresolved effects separately.
- Durable intent records increase write volume. They provide required recovery facts.
- An extension can request malicious effects. Port admission and capability policy remain mandatory.

## Claim Boundary

Passing evidence proves a bounded native callback and service lifecycle for the exact local pilot cohort.

It does not prove sandboxing, child correctness, callback semantic correctness, effect success, network delivery, fleet availability, or production readiness.
