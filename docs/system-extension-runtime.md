# System-extension service runtime

Molten system extensions are optional, manifest-installed, long-running distributed-service implementations. They are a separate executable tier from ordinary sandboxed plugins and application workloads. The tier exists for services that need lifecycle, recovery, supervision, typed fabric ports, and durable operator evidence; it does not broaden plugin authority.

The normative split is:

| Tier | Purpose | Authority boundary |
| --- | --- | --- |
| Sandboxed plugin | Short-lived or callback-style untrusted extension logic | Narrow plugin hostcalls only |
| System extension | Optional long-running service or protocol implementation | Explicit system-tier admission plus exact fabric-port bindings |
| Application workload | Consumer of admitted services | Service-use authority only; no inherited adapter or extension authority |

Plugin manifests, plugin permission receipts, artifact possession, service names, and implementation refs cannot substitute for a system-extension manifest or system-tier admission.

## Functional core and executable shell

`molten-core::system_extension` is the pure deterministic core. It owns:

- canonical manifest admission inputs;
- exact callback, execution-profile, state-schema, and fabric-port requirements;
- lifecycle and generation transition laws;
- deadline, cancellation, resource, overload, and backpressure decisions;
- typed fabric-effect validation;
- restart-versus-quarantine decisions;
- state-schema migration compatibility;
- executable-conformance checks.

The core performs no filesystem, network, storage, process, environment, clock, entropy, telemetry, or code-loading effects.

`molten::system_extension::Dispatcher` is the imperative shell. It invokes an admitted `SystemExtensionExecutor`, validates its result before releasing any typed effect request, accounts resources, applies the pure lifecycle transitions, and emits canonical Preserves/BLAKE3 evidence. The host does not expose ambient filesystem, network, clock, randomness, process, environment, or backend handles in callback context.

## Canonical manifest

`<system-extension-manifest-v1 "molten.system-extension.manifest.v1" ...>` binds:

- extension and service identifiers;
- implementation content ref;
- declared callback groups;
- exact required and optional fabric-port binding refs;
- capability, policy, provenance, and evidence-profile refs;
- finite resource envelope and overload policy;
- separately admitted execution profile;
- current and compatible state schemas;
- initial generation;
- explicit non-claims;
- the system-tier admission ref.

Admission rejects unknown callbacks, duplicate fields represented by the typed input, malformed refs, absent lifecycle callbacks, incompatible or silently substituted ports, port authority outside the admitted tier, unadmitted execution profiles, incomplete state compatibility, missing policy/provenance/capability evidence, and missing non-claims. Optional missing ports remain absent; an available but incompatible optional port does not silently fall back.

## Lifecycle and generation fencing

The pure lifecycle includes:

```text
absent -> installed -> admitted -> initializing -> initialized
       -> starting -> running
       -> checkpointing -> running
       -> recovering -> running
       -> draining -> drained -> shutting-down -> stopped -> removed
       -> failed -> restarting -> starting/recovering
       -> upgrading / rolling-back
       -> quarantined
```

Every event and callback names the active generation. Upgrade and rollback create exactly the next generation. Old-generation callbacks, effects, completions, timers, and messages are denied before executor invocation.

### Incarnation fencing

Generation names the implementation; it does not name the running instance. A restart or recovery replaces the
instance while the generation stays the same, so the lifecycle also carries a monotonic `incarnation` that advances on
every restart-entry transition. Every runtime-local timer, callback completion, and failure notification binds the
incarnation that produced it, and dispatch denies work whose incarnation is not the executing one, before executor
invocation and before any effect is planned. A delayed timer, a pending callback completion, and a failure notification
from the previous instance are therefore all rejected against the replacement instance, and the lifecycle receipt binds
the previous and next incarnation so the advance is visible in evidence.

Durable logical work is not fenced out by the incarnation. It is re-admitted deliberately: the shell presents durable
work with a fresh delivery claim bound to the current generation and incarnation, and the claim records the current
incarnation. Runtime-local work presented as durable work is denied, and durable work presented without a current claim
is denied. The native callback protocol carries the invocation incarnation in the envelope and the effect record, and
the native host binds the materialized effect to the invocation it answers; the native callback envelope, outcome, effect,
instance-state records, and the native ALPN move to version 3, so a native extension built against version 2 is not
admitted by a version 3 host. Drain and shutdown completion require zero tracked callbacks, queued events, in-flight bytes, streams, timers, and effect requests. Retryable failures may restart only within the admitted budget; fatal, policy, resource, or generation failures quarantine rather than loop indefinitely.

## Callback contract

Supported callback groups are:

- `initialize`, `start`, `drain`, and `shutdown` for the required lifecycle;
- `request` and `message` for unary and asynchronous service traffic;
- `stream-open` and `stream-event` for bounded streams;
- `timer` for logical timer delivery;
- `health` for bounded health evaluation;
- `checkpoint` and `recover` for explicit state continuity.

A callback receives canonical event/payload refs, generation, sequence, logical tick, and an explicit logical deadline. Cancellation and expired or overlong deadlines deny before invocation. Callback outputs are bounded content refs, state/checkpoint refs, health, and typed fabric-port effect requests. Callback code cannot mint authority by returning a receipt.

## Typed facades

The facade layer is an opt-in library over the same interface. A facade is admitted as a manifest profile that binds
the event, state, and reply schemas, the extension manifest it composes with, and the profile's own knobs: a
state-machine phase table with per-phase legal and postponable events, a postponement bound, and a state timeout with
its fencing declaration; a task completion schema; and a concurrency limit. Admission rejects a phase table whose
events are not declared, unbounded postponement, a state timeout without incarnation-and-phase fencing, a task profile
without a completion schema, and a concurrency limit below one.

A facade turn is `current state plus admitted event` lowered into `next state, replies, effect intents, and timer
intents`. The facade owns the structure, not the business logic:

- an event outside the phase's legal set is rejected with a typed phase issue and the state does not change;
- a postponable event is deferred while the phase's bound holds, and dead-letters through the existing delivery path
  once the bound is exhausted;
- a state timeout is scheduled fenced by its phase and the current incarnation, and a timer that fires from an earlier
  phase or an earlier incarnation is rejected as stale;
- effects bind the executing generation and incarnation and must name a bound port;
- a parallel event for the same logical service that presents a stale turn sequence is rejected, so two events cannot
  interleave inside one state transition;
- a completed task maps to the existing stopped transition and consumes no restart budget.

The facade lowers into the canonical envelopes the interface already defines, so a facade-lowered turn and a
hand-written turn produce identical envelopes, admission results, and evidence. Facades add no runtime, no I/O route,
and no state store, and they change no failure-isolation, authority, or durability guarantee.

## Fabric effects

A system extension can request an external effect only through an exact admitted `(port-id, version)` binding. Each request binds:

- operation class;
- input and output schema refs;
- request content ref;
- active generation;
- active incarnation;
- accounted bytes.

The host rejects ambient filesystem, network, clock, randomness, process, and environment targets. It also rejects unbound ports, unsupported operations, schema mismatch, stale generations, stale incarnations, malformed refs, duplicate requests, and resource excess. Successful validation returns effect requests to an adapter shell; it does not execute them inside the pure core and does not grant new authority.

## Execution profiles

Execution profiles are explicit and have no fallback:

- `in-process-native` is trusted native Rust execution. It has process authority risk and must be admitted only for reviewed code.
- `native-process` runs one admitted executable per callback through the bounded execution fabric port. Its v2 local pilot supplies identity-checked payload, state, completion, effect, output, and checkpoint bytes without reference-only fallback.
- `sandboxed-component` is exercised by the conformance fixture through a fuel-bounded Wasmtime module with no imports or WASI. A production component adapter must separately define its ABI, memory/fuel limits, hostcalls, process isolation, and evidence.

An executor whose advertised profile differs from the manifest is rejected before activation. Native execution is not a sandbox. Wasmtime execution does not itself grant capabilities or prove extension semantics.

## Resources, supervision, and recovery

The manifest resource envelope bounds callback concurrency, queued events, in-flight bytes, streams, timers, effect requests, callback deadline ticks, shutdown grace ticks, and restart attempts. Overload behavior is explicit: reject, bounded delay, or upstream backpressure. There is no silent drop policy.

Checkpoint refs are canonical and state-schema compatibility is explicit. Recovery consumes a named checkpoint ref under the active generation. Upgrade and rollback require sequential generations and an admitted state-migration plan. The first host slice exercises checkpoint, a retryable callback failure, bounded restart, recovery, post-recovery request handling, drain, and shutdown.

## Evidence and operator readback

The runtime emits:

- canonical admitted manifest and exact fabric-port binding artifacts;
- lifecycle transition receipts;
- callback receipts bound to real executor invocation and execution profile;
- typed effect refs only after outcome validation;
- failure, restart, recovery, drain, and shutdown evidence;
- bounded status artifacts exposing extension/service identity, manifest ref, generation, phase, profile, port bindings, resource envelope/usage, health, restart count, checkpoint ref, last lifecycle ref, and invocation count.

Status artifacts exclude private keys, bearer tokens, raw capability material, environment values, and backend handles. CLI readback parses only the fixed status schema and prints selected safe fields.

Run the deterministic executable fixture and inspect its final status with:

```sh
cargo run -- system-extension run-fixture \
  --profile sandboxed-component \
  --out target/system-extension-fixture
cargo run -- system-extension show \
  --status target/system-extension-fixture/status.preserves
```

The deterministic fixture also supports `--profile in-process-native`. The native process pilot uses the separate lifecycle test and profile in [`native-system-extension-host.md`](native-system-extension-host.md).

## Non-claims

Installation is not activation. Artifact possession is not authority. A callback success or receipt is not proof of:

- consensus;
- durable persistence;
- protocol compatibility;
- extension semantic correctness;
- production readiness;
- correct external effect execution.

OpenRaft is not selected, adapted, or used. A future consensus implementation remains an optional system extension composed from pure laws and admitted adapters.

Aspen `main` is an AGPL-3.0-or-later architecture and behavior reference. Molten's system-extension requirements, implementation, and tests use the repository's `AGPL-3.0-or-later` license.
