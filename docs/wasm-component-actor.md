# Ordinary Component Model actors

`molten::wasm_component::consumer::Actor` is the bounded library product
root for ordinary actors. It is distinct from the legacy core-module actor
harness and from addressable-actor sleep/wake/restore ownership. Each invocation
runs a fresh component instance using the retained admitted shared engine.
Installation does not imply component correctness or release eligibility.

## Installation and current inputs

Load a producer-published bundle with `VerifiedBundle::load`, using authorized
source/staging directory capabilities and a fresh admitted staging child. Keep
the actual Structural+Bytes reports. Build `ActorInstallInput` from:

- the exact admitted `RuntimeProfile`;
- `Arc<VerifiedBundle>`, not raw Wasm or a locally fabricated `MantleBundle`;
- its separate `AdmissionEnvelope`, bound to that genuine bundle reference;
- artifact facts measured from the actual component;
- the exact declared import manifest and complete current import grants.

Load the closed effect profile with `wasm_component::hosted()`. The installed
actor retains `wasm_component::Engine`, whose engine-specific epoch worker has
a capacity-one stop channel and wakes on disconnect. `wasm_component::source_cohort()`
reports the linked source identity; it does not grant execution authority.

`Actor::install` verifies production materialization, profile selection,
artifact kind, actual fixed resource declarations, observed import/export
surface, facts, and the complete admission plan before building the engine. The
installed profile, snapshot, envelope, facts and import manifest are private and
cannot subsequently be swapped. `materialization()`, `profile()` and `facts()`
provide read-only inspection.
The resource facts describe the product inspector's conservative declaration
counts, not the checker's independent allocation counts: an imported table
alias can contribute another declaration without allocating another table.
Derive facts with that product counting rule from the selected bytes; preserve
the checker's allocation observations separately rather than substituting them.

Each `ActorInvocationInput` contains the actual canonical actor `IOValue`, an
explicit requested execution profile, and the current import grants. Invocation
passes all of these through the existing `ExecutionRequest` admission boundary;
there is no fallback from a component request to a legacy core module.

- `invoke` runs a pure turn; it does not supply an ambient effect host.
- `invoke_live(input, live_input)` takes the canonical `ActorInvocationInput`
  and `consumer::host::LiveInput<C, V>`. The latter owns `scope`, `generation`,
  `incarnation`, `current: C` implementing `CurrentContext`, and
  `values: V` implementing `NativeCallbackValuePort`.
- `replay` takes the same canonical actor input, selected scope/lifecycle, and
  exact recorded effects. It accepts neither a live value port nor authority
  services.

All return an `ActorInvocation` containing the canonical `ExecutionOutcome` and
an inspectable `consumer::host::Recording` (live/replay setup can also return `Denial`).
No process-global actor registry, lifecycle ownership, mailbox, or durable
checkpoint protocol is implied by this bounded root.

## Owned effect bridge

`consumer::host::CurrentContext::with_current` constructs the existing
`host::CurrentInput` only within its closure. This lets an application own its
policy JSON, UCAN token, holder signature, nonce and enforcement-service
implementations while constructing the borrowed Basalt service boundaries on
the stack. The generation, incarnation, installed effect manifest, handler
profile, measured grant, capability set, limits and authority all come from the
product, never from guest assertions.

Before dispatch, the hosted linker decodes the bounded canonical request and
requires its entire grant to equal the exact outer admitted `ExecutionRequest`
grant. A guest/current-host agreement cannot substitute for that outer binding.
The existing Session still independently rechecks lifecycle and authority.

`consumer::host::Live<C, V>` always calls the existing `host::Session::invoke`. The
session performs current lifecycle/manifest/policy/UCAN/holder admission and
uses the real value port, publishing canonical admission, intent, request,
response and completion objects. `SharedValuePort(SharedCallbackValuePort)`
permits the application and its host to retain access to the same product store
without copying it or fabricating responses.

The engine owns the host for the invocation. Its paired cloneable
`consumer::host::Recording` survives that ownership transfer. `snapshot()` returns
`consumer::host::Records` containing completed `RecordedEffect` values (canonical completion
record, exact response bytes and the existing `EffectLogEntry` references), every
live `consumer::host::Publication` attempt's request/value identity and byte count, original
accepted receipt (including invalid receipts), structured `ValuePortFailure`,
first host denial, and replay's remaining boundary count. An owning value-port
wrapper retains publication certainty before Session converts failures into a
denial. Partial publications remain in the real store; a failed invocation is
not permission to retry an uncertain effect.
Live hosts reserve the complete 1,024-boundary recording capacity and five
publication slots per Session boundary at construction. Each call/attempt checks
its remaining slot before authority or port dispatch. Replay reserves only its
selected finite boundary count and has no live publication adapter.
No recording-vector allocation or capacity expansion occurs after dispatch; an
unavailable recording lock after a completed effect requires reconciliation.
Pure invocations use the explicit empty recording constructor.

`consumer::host::Replay` calls the actual `host::Replay::consume` for every boundary,
checks the accompanying effect-log sequence/scope/request/response references,
and stops after divergence. Missing, modified or reordered boundaries deny.
`Actor::replay` also turns a successful guest execution with trailing
unconsumed boundaries into a canonical denied execution. Completion denial
appends a terminal denial to the observed inspection/instantiation/execution
chain, retaining actual fuel, input, grants and source/configuration identities;
it clears output instead of pretending execution never occurred. Applications
composing the bridge directly must call `consumer::host::Recording::ensure_replay_complete()`
before accepting replay success.

## Manifest-installed component callbacks

`ComponentCallbackHostFactory::create` returns `system_extension::Binding<Host>`,
not a bare host. `Binding::live(host)` obtains the current grant
from that live host's `CurrentContext` and retains that same host's recording.
Installation grants are checked only as installation inputs; each callback uses
the binding's current grants, including refreshed lifecycle-bound grants after
restart. Private binding fields prevent substituting another host's completion
handle. `Binding::replay(host, recorded_grant)` explicitly
selects offline replay and its recorded authority, with no live fallback.
Successful replay execution must consume the complete recording before callback
output admission or a pending outcome exists; trailing effects produce canonical
denied execution receipts and cannot publish or commit.

Callback observations retain the live/replay `host_records` snapshot even when
guest execution denies, plus each callback-outcome publication's value identity,
byte count, returned receipt and structured failure. Hostcall and outcome
`UnknownAfterAcceptance` or invalid/mismatched accepted receipts set the same
persistent in-memory executor fence: all subsequent invocations, including new
incarnations,
deny before materialization or guest execution. Dispatcher uses the executor's
`failure_class()` and quarantines these non-retryable failures rather than
restarting them. Definitely rejected publications remain ordinary retryable
errors. No automatic reconciliation, fence reset, durable fence journal, or
process-replacement recovery protocol is supplied by this bounded executor.

## Product smoke procedure

Use a throwaway Rust example in the integrated checkout; do not convert these
steps into fabricated evidence or bypass the published bundle verifier.

1. Generate and materialize the real effect-world guest using the pinned
   producer's `prepare_effect_fixture` example and full Mantle pipeline. Select
   the unchanged effect profile and exact WIT. Load the actual published bundle
   with `VerifiedBundle::load`; derive `ArtifactFacts` from its measured bytes
   using the product resource-declaration counting rules, not profile maxima,
   independent allocation counts or legacy fixture facts. Retain the separate
   checker receipt.
2. Construct the separate real admission envelope, exact declared import
   manifest, measured grants, effect manifest and handler profile. Construct a
   host scope and genuine product-owned authority services/holder credential.
   Seed the actual value port with one canonical blob and retain its shared
   handle. Install the actor from that verified snapshot.
3. For the producer's generic guest, the actor input is a canonical Preserves
   ByteString containing the command bytes: `0 || canonical_payload` is pure;
   `1 || canonical_host_request` calls the host; `2` traps; `3` returns malformed
   output; `4` performs unbounded compute; `5 || count_u64_le` performs bounded
   compute. The entire command must be packed as canonical Preserves, not sent
   as raw Wasm argument bytes. Encode the effect request with
   `encode_host_request`, binding the exact scope/lifecycle/current grant.
   The producer's strict framing is tag `b2`, minimal unsigned LEB128 byte
   length, then the command, with no trailing bytes and at most 65,536 command
   bytes. Exact smoke inputs in hexadecimal: pure Boolean true
   `b2 02 00 81`; trap `b2 01 02`; malformed `b2 01 03`; infinite compute
   `b2 01 04`; finite zero iterations `b2 09 05 00 00 00 00 00 00 00 00`.
   Finite compute returns a canonical eight-byte ByteString (`b2 08` followed
   by its u64 little-endian state), not raw diagnostic bytes.
4. Invoke live with mode 1 and valid current authority. Decode the canonical
   guest output with `decode_host_response` against that request; require a
   passing response and the actual stored blob value. Inspect the completed
   record and response references through `host_recording.snapshot()` and
   materialize those objects from the retained product value port.
5. Invoke with a stale generation or denied current authority and a separately
   valid current request binding. Require a canonical denied host response and
   no blob materialization; inspect the published denial/intent/completion.
   Never treat the guest's successful return of a denial response as an
   admitted effect.
6. Replay the admitted invocation with the exact recorded scope/lifecycle,
   input, grants and `RecordedEffect` vector. Supply no authority or live port.
   Require exact output and canonical execution receipt agreement. Then change
   one response byte, request/lifecycle binding, entry reference, record order,
   or remove/append a boundary; each replay must deny without live fallback.
7. Exercise modes 2/3/4 and require canonical trap/payload/fuel-or-deadline
   denials. Separately mutate the install profile, facts, declared imports,
   bundle-linked envelope or requested execution profile and require denial
   before a live value-port read.

The [2026-10-29 operated consumer record](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-28-hosted-consumers/2026-10-29-operated-consumers.json)
retains an executed instance of this procedure through immutable Mantle
`dd2bebab894688d07e7a29c75c1437f4ecc120df`, including the disposable consumer
source closure and canonical receipts. Its installed-extension companion also
completed checkpoint, failure/restart/recovery and refreshed-authority requests.
Both kept the unchanged 100,000-fuel profile; the actor separately observed an
operated deadline after a completed synchronous host effect, not interruption
of a blocked host call. Molten component-runtime maintainers own this composition
and its repeatability evidence.

Only an actual run of those steps is execution evidence. This document and the
public types do not claim a completed adoption gate or authorize archiving the
active Cairn change.
