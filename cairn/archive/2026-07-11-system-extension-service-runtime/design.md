## Context

The plugin ABI is intentionally narrow and receipt-first. It can validate artifacts and model hostcalls, but it has no executable service object, callback dispatcher, persistent instance supervisor, protocol listener integration, or recovery loop. Widening ordinary plugins in place would collapse trust tiers and make all plugin artifacts candidates for durable state and network authority.

This change introduces a sibling system-extension runtime. It reuses canonical identity, policy, provenance, resources, receipts, and supervision while keeping the existing plugin profile bounded.

## Decisions

### 1. System extensions use a separate manifest and admission profile

**Choice:** A system-extension manifest declares service identity, implementation artifact, callback surface, required and optional fabric ports, capability refs, resource envelope, execution profile, state schema, upgrade compatibility, and non-claims.

**Rationale:** Strong service authority must be explicit and reviewable rather than inferred from plugin metadata or operation names.

### 2. The host executes an explicit callback state machine

**Choice:** The runtime exposes initialize, start, request, message, stream-open, stream-event, timer, health, checkpoint, recover, drain, and shutdown callbacks. The pure core validates legal transitions and produces effects; the shell invokes callbacks and resolves effects through admitted ports.

**Rationale:** Long-running services need more than one-shot hostcalls, while an explicit state machine keeps lifecycle behavior deterministic and testable.

### 3. Service instances are generation-scoped and supervised

**Choice:** Each active instance is bound to extension id, service id, node id, generation, configuration digest, capability-set digest, port-binding digest, and resource envelope. Restart creates a new attempt within the same admitted generation; upgrade creates a new generation.

**Rationale:** Generation fencing prevents stale callbacks, timers, streams, and recovery work from mutating a replacement instance.

### 4. Concurrency and backpressure are host-enforced

**Choice:** The manifest selects reviewed limits for callback concurrency, queue depth, in-flight bytes, stream count, timer count, and shutdown grace. The host rejects or delays work according to explicit policy and propagates cancellation and deadlines.

**Rationale:** Extensions must not turn the node into an unbounded executor or hide overload in extension-specific queues.

### 5. Effects cross typed fabric ports only

**Choice:** Callback code receives canonical event values and returns state transitions, outputs, and typed effect requests. Native and sandboxed execution profiles may differ internally, but neither receives ambient filesystem, network, clock, process, environment, membership, or storage access.

**Rationale:** This is the key capability and simulation boundary.

### 6. Lifecycle evidence is coarse-grained and canonical

**Choice:** Emit evidence for admission, activation, generation changes, checkpoint/recovery, drain, failure, and final shutdown. Per-message evidence is optional and profile-bounded; it is not the default hot path.

**Rationale:** Service operation must remain observable without making callback throughput receipt-bound.

## Functional core / imperative shell split

- Pure core: manifest validation, lifecycle transition legality, callback routing, generation checks, resource/accounting decisions, effect validation, restart policy, and evidence payload construction.
- Shell: load or instantiate code, receive runtime events, invoke callbacks, perform admitted effects, enforce process or Wasm limits, persist checkpoints/evidence, and notify supervisors.

## Risks / Trade-offs

- Native extensions can compromise the process if isolation is weak. Keep native execution separately admitted and retain a sandboxed profile for less-trusted code.
- A large callback surface can become unstable. Canonicalize events, version callback groups, and deny unsupported optional callbacks at activation.
- Recovery and upgrade semantics differ by service. The host provides bounded hooks and fencing; extension-owned state compatibility remains an extension claim.
