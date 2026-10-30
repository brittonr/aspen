# System-extension service runtime

Molten system extensions are optional, manifest-installed, long-running distributed-service implementations. They are a separate executable tier from ordinary sandboxed plugins and application workloads. The tier exists for services that need lifecycle, recovery, supervision, typed fabric ports, and durable operator evidence; it does not broaden plugin authority.

The normative split is:

| Tier | Purpose | Authority boundary |
| --- | --- | --- |
| Sandboxed plugin | Short-lived or callback-style untrusted extension logic | Narrow plugin hostcalls only |
| System extension | Optional long-running service or protocol implementation | Explicit system-tier admission plus exact fabric-port bindings |
| Application workload | Consumer of admitted services | Service-use authority only; no inherited adapter or extension authority |

Plugin manifests, plugin permission receipts, artifact possession, service names, and implementation refs cannot substitute for a system-extension manifest or system-tier admission.

## Public node service-host admission

`molten node service-host {install,status,supervise,submit,readmit,deliver,upgrade,reconcile}`
requires `--state-root`, `--admission`, `--logical-tick`, and `--input` on every
operation. Admission and input names are relative paths below the existing,
explicitly selected state-root capability. They contain canonical packed
Preserves, not JSON or rendered text. `--preview` projects the exact operation
request for an independent signing producer; it does not authorize or execute
that operation. Loading still requires its own current authorization.

`--preview-load` is the read-only bootstrap: it validates the actual source gate
and canonical bundle, then emits the exact load signing request before opening
hosting journals, value stores or replay reservations. It conflicts with
`--preview`. The independent producer signs load, runs operation preview, then
refreshes the load nonce and signs the projected exact operation before invoking
the actual command. A request projection is never an admission receipt.

`node_service_host::codec::{Bundle,ExtensionInput,ReplacementInput}` is the producer
interface. `encode`, `reference`, and `decode` bind all bytes, with at most 64
services, 64 exact replacements and 16 MiB per bundle. Each service supplies the
actual system-tier request, manifest, port descriptors, native host profile,
executable evidence, bounded execution profile/request/grants, materialized
executable locator, callback template, durability profile/namespace and owned
ledger value/journal namespaces. The loader invokes the owning admission APIs;
decoding alone grants no authority. Executable bytes are measured through the
selected root and callbacks use a retained file-descriptor path, not a later
ambient lookup of the bundle's executable path. Existing journals reconstruct
`Supervisor::from_recovered`; a stored replacement generation requires its exact
admission material to remain in the bundle.

Before journal/value-store effects, the public boundary validates the actual
executing ELF through the startup source-gate loader, then verifies an exact
`load` request: subject and input ref are the bundle ref, state ref is the opaque
source validation ref, and logical tick is explicit. Independent operator-owned
`service-host-trust` pins UCAN issuer/holder keys and Basalt policy. Exact
holder-signed credentials are selected by `authority::credential_path`; their
operation binding covers the bundle and every request field. Current validity,
revocation and durable no-replace nonce admission are checked on every request.
Neither bundle references nor source-gate evidence grant operation authority.
Host separately admits the exact operation before its durable intent/callback.

Operation producers use `node-service-host-{operation}-v1` records. Status has no
fields; install/reconcile select a declared group; upgrade selects a member and
exact replacement ref. Submit retains the complete work presentation and actual
payload bytes. Readmit accepts either a retained work ref or an explicit
`node-service-host-migrated-work-v1` containing work ref, migration ref and new
payload. The latter is accepted only against actual verified stored native
migration evidence. Delivery must reproduce retained work, payload and the
current claim. Canonical status/receipts omit tokens, keys and payload bytes.
This is an experimental local-live materialized-values path, not a sandbox,
distributed availability guarantee or production promotion.

Durable node hosting requires admitted `checkpoint` and `recover` callbacks in
every base/replacement manifest. Bundle admission rejects their absence before
loader effects. Initial activation checkpoints each started member so a later
real invocation can recover its Running native record without resetting state.
Admission also enforces each native profile's aggregate `max_instances` across
distinct service members, including every reachable replacement profile. Multiple
admissions for the same member/profile consume one slot; the global 64-service
codec limit never raises an admitted profile's smaller instance bound. One
profile reference cannot describe conflicting profile values.

### Retained daemon owner

A hosting node must retain in its actual long-lived node loop
`node_service_host::installed::Owner`, constructed with
`open_after_source_admission(root, &validated_source_gate)`. The optional fixed
`service-host-startup` selector is the packed record
`<node-service-host-startup-v1 "relative-bundle-locator" logical-tick>`.
Absence means hosting was not selected, not successful hosting. Construction
requires separate exact current load and status credentials, reconstructs real
native supervisors and owns their journal locks and measured execution handles
until shutdown. `ready()` exposes the canonical authorized hosting observation;
it must not be published before construction succeeds. This constructor does
not activate callbacks or replace operation authority. Retaining it inside a
one-shot startup function would not host services for the daemon lifetime.

The public `node run-loop` and both `node serve` modes own this session across
their actual control-loop/listener call. Live-Iroh serve validates source and
host admission before binding; it publishes the canonical hosting-ready record
to the current process's stdout only after the live endpoint is bound, then
retains the host through `bound.serve().await`. Output is flushed so a real
crash campaign can wait for that child's actual hosting observation before
terminating that same process. No persistent ready marker can be reused as
proof that a later or already-killed process owns the services. An absent
selector produces no hosting-ready observation. Normal completion releases
only that session's journal and measured-file capabilities; process loss leaves
the owning durable recovery/uncertainty laws intact.

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
