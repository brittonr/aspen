# Admitted native effect runtime composition

## Authority and ownership

This is a separate public-host prerequisite, not a consequence of source admission,
callback success, queue acceptance, or service-name matching. It composes the
existing native supervisor effect journal with an explicitly installed
collaboration-scope consistency replica and committed application. It does not
provide a generic plugin/effect framework or grant production readiness.

The pure system-extension core continues to own exact manifest/port/generation
admission and canonical effect/completion identities. `Supervisor` owns native
intent-before-effect recording and completion delivery. The public node loader
owns explicit admitted runtime composition and retains its concrete scope router,
replica/application and capability roots for the serving-loop lifetime.
`collaboration_scope::Application` owns current collaboration authorization and committed state;
`CommitPort` owns the exact staged consistency proposal boundary. A raw
sender, token, reference, source-gate receipt or callback output cannot install a
router or grant resource/effect authority.

## Required admission

Before creating effect resources, configuration must bind the exact native
service, admitted manifest/port binding, scope/group, active generation, policy,
resource and installation references. The operator's actual current
holder-signed UCAN/Basalt operation must cover this exact configuration and bundle.
Scope mutations additionally pass the owning scope application's current policy,
revocation, consent, replay and state/currentness admission. Native operation
admission is not scope mutation admission, and source trust grants neither.
Unconfigured services/ports/operations remain fail-closed.

## Effect order and uncertainty

The native callback's approved effect is not a successful scope commit. Before
routing it, the native journal persists its exact effect intent. Only the exact
staged, currently admitted command may enter the actual bounded active Raft
replica. A `proposal-queued` receipt explicitly means queued, not committed and
not protected-resource authority. Actual application of a committed Raft entry
must be observed in the retained `Application` before publishing a commit
completion. Completion is delivered through the native supervisor's existing
completion callback path, preserving exact operation/binding/generation checks.

Three references are distinct: the native Effect terminal ref names provider output;
the immutable cache address names the `system-extension-effect-completion-v2` wrapper
carrying that output inline; `NativeEffectDisposition::Completed` returns the Message
callback receipt, which is also the work/readiness completion boundary. Neither the
terminal output ref nor that callback receipt is the wrapper's cache address.
Durable readback must retain the actual cached wrapper, decode its inline bytes,
verify the output content identity, and correlate the Request ingress, terminal
effect, consumed operation and completion-bound native readiness. Fresh scope
verification also produces a new authorized proposal: its command ref need not equal
the historical intent's empty-decision command ref. Evidence must retain the actual
proposal, exact mutation, current allowed decisions and committed position rather
than pinning the intent ref.

Callback-cache values are opaque content-addressed bytes, not a Preserves-only
store. Readback classifies canonical completion candidates without following
non-regular entries; every selected wrapper still needs its exact immutable address,
schema and native operation links. An unrelated opaque value is not malformed
completion evidence, and absence of a matching wrapper is not completion.

Definite pre-effect denial is distinct from queue acceptance, unknown outcome,
application/storage failure and completion-callback failure. Once an effect may
have occurred, failure leaves durable unknown/reconciliation-required state.
There is no automatic retry, model-only success, port echo, fabricated commit or
implicit activation. A durable store closed uncleanly is quarantined on reopen and
serves only explicit reconciliation, so process loss never becomes an implicit
restart or retry. Process loss must not requeue an uncertain proposal merely
because a new process has the same configuration. A completion callback emitting
new unconfigured effects cannot be treated as completed work.
Unknown work can retain an actual Request execution, failure or acknowledgement
receipt. Its presence is not a successful effect completion: the work disposition,
exact native effect/completion linkage and callback outcome remain independent.
For Unrouted work, the retained receipt identifies the Request ingress, not a
completed provider effect.

## Public installation and producer contract

The explicit operator-owned selector is `service-host-effects`, containing
canonical packed `service-host-effects-v1` configuration with at most 32 exact
collaboration bindings and 1 MiB. `node_service_host::effects::config` exposes
the bounded `Configuration`/`Binding` producer codec. Each binding names service,
manifest, canonical port binding/key/operation, scope group, active generation,
policy, resource and installation refs, plus an owned relative runtime artifact
locator and its exact content ref. Duplicate services, unknown protocols,
traversal, malformed refs and altered bytes deny rather than select a fallback.

`effects::authority::{install_request,route_request}` projects the exact current
operator signing requests. `install-effects` binds bundle, complete configuration,
source-validation observation and logical tick; it is verified before scope
resource creation. `route-effect` additionally binds the selected installation
and every actual typed effect/binding field, including schemas, request ref,
generation, incarnation and byte accounting. Existing independent issuer/holder
trust, Basalt policy, credential files and durable nonce admission supply genuine
current verification. Source validation appears only as installation evidence,
never as the effect grant. Scope mutation authority remains a separate check.

Both public one-shot operations and the retained daemon owner install these
routers only after real native supervisors are reconstructed. The loader checks
the actual Running service, admitted manifest binding, generation, native policy
and executable resource; the scope factory's pure admission must produce the
exact configured installation/group/policy/resource before any scope worker or
storage is opened. Fresh install without an active service cannot implicitly
activate that service through effect configuration. A later generation must
receive a separately admitted matching scope configuration.

Native bootstrap `install` runs its explicit authorized Initialize/Start/checkpoint
without trying to construct a Running-only scope installation beforehand.
Non-delivery one-shot operations and request previews do not open an unused scope
replica. Actual direct `deliver` composes the admitted scope runtime before its
native callback; retained `installed::Owner` construction composes it after native
state is already Running. Thus effect configuration neither blocks the native
bootstrap through a circular precondition nor silently activates a service.

For a fresh node, authorize and execute `node service-host install` before starting
`node serve` with a selected effect runtime. Publish the selector only after that
explicit activation, or leave it absent until activation has completed. A selected
runtime with a non-Running member fails readiness; selection does not defer or
implicitly activate the member. The runtime producer must measure the seven exact
Raft infrastructure port bindings, not the first seven entries of a sorted manifest.

A direct `deliver` opens a new runtime installation. It needs its own exact
`install-effects` grant for that command's source-validation reference and logical
tick, in addition to the load, delivery, route-effect and independent scope grants.
A startup install grant cannot authorize installation at a later tick. A denial
after persisted effect intent must not trigger a speculative retry at another tick.


The retained public owner exposes `preview(&Operation, tick)`,
`execute(Operation, tick)` and bounded `execute_packed(command, bytes, tick)`.
These use the same actual Host operation/admission path as the public CLI and
operate on the retained native/scope runtime instead of reopening its locked
stores or constructing a test-owned supervisor. All existing lifecycle/work
operations remain exact-current-authority operations. The opaque source token
still has to come from actual executing-ELF admission; this API has no synthetic
test constructor or source-gate fallback.

The concrete router refuses unresolved predecessor native effects and binds each
route to the current actual callback receipt and native recovery group. Its
current operator grant is checked immediately before delegating to the retained
actual scope runtime. The native host routes only approved effects through its
existing intent recorder and delivers verified completions through the existing
materialized completion callback. Root work becomes complete only when the actual
completion callbacks succeed without further unhandled effects; queue receipts
and cleared effect lists are not completion evidence.

## Opt-in retained local control transport

`node serve --live-iroh --service-host-control-stdio --state-root ROOT` exposes
the selected retained owner to an explicitly owned local process session. The
flag requires `--live-iroh`; flag-free serving is unchanged. Source admission,
exact startup grants and selected runtime/worker startup still precede readiness.
The ordinary endpoint-bound line and native ready record are published only after
the actual Iroh listener binds. They may coexist with protocol replies on stdout.

Send one newline-delimited Preserves text record per request:

```text
<node-service-host-control-v1 "preview" "status" 1 <node-service-host-status-v1>>
<node-service-host-control-v1 "execute" "status" 1 <node-service-host-status-v1>>
```

The four fields are mode (`"preview"` or `"execute"`), a closed service-host command,
unsigned 64-bit logical tick, and its exact operation value. Existing host commands
and the metadata-only `"scope-observe"` command are accepted; schemas cannot be
substituted between commands. Frames are limited to 16 MiB excluding the newline; canonical packed
operations retain the existing 1 MiB limit. Requests enter a capacity-one channel
and execute serially on the retained owner, while the bound live listener remains
active. No operation reopens locked stores, reloads source, reinstalls a runtime,
or gains authority from the transport. Every execute still checks the existing
exact current signed operation authority. Preview returns the existing signing
request projector and does not grant authority.

Successful replies have four fields:
`<node-service-host-control-result-v1 MODE COMMAND TICK VALUE>`. Preview's VALUE is
the actual `node-service-host-operation-request-v1`; execute's VALUE is the existing
five-field `node-service-host-receipt-v1`. Failures have three fields:
`<node-service-host-control-error-v1 COMMAND TICK DETAIL>`. Malformed requests use
`"invalid"` and tick zero; oversized or unterminated framing ends the session.
Error details are fixed bounded diagnostics: exact safe authority/uncertainty
messages may be retained, while arbitrary backend diagnostics are withheld to
avoid disclosing payloads or credential material. An error is never a successful
receipt and never authorizes a retry; inspect durable uncertainty before retrying.

EOF explicitly ends this owned session, shuts down its actual listener and
releases its retained owner. It does not fabricate a successful listener/service
receipt. Listener completion under its ordinary event/timeout bounds also ends
the session. The descriptor-polling reader is cancellable and joined, including
when stdin stays open or a bounded send is pending. The caller must continuously
drain stdout while submitting requests.

The scope worker has a separate owned stop notification, not a stop command that
can be lost behind a full diagnostic queue. It finishes any already accepted
adapter operation before stopping; queued diagnostic clients then observe closure.
Keeping a diagnostic handle cannot keep the owning replica or its stores alive.

The retained Raft ingress pump distinguishes an idle pre-session accept deadline
from a failed session. Idle waits keep the admitted listener alive and do not
consume its finite successful-delivery budget. Handshake, framing, protocol and
acknowledgement failures remain errors; delivery-budget exhaustion remains
terminal. This does not retry an accepted message or protected operation.

## Currentness observation before signing

The retained owner additionally exposes `preview_scope(service, &read, tick)` and
`observe_scope(service, &read, tick)`. The opt-in local operation is
`<node-service-host-scope-observe-v1 SERVICE READ_REF>`, selected only by the closed
`"scope-observe"` command. Preview uses the normal signing request projector.
Execute requires a **separate exact current operator grant** with ability
`scope-observe` on the host bundle resource (the resource
producer is `node_service_host::authority::resource`). Source/load/install-effects,
previous reads and route-effect grants do not authorize this operation.

The request binds the current actual Host snapshot, full selected configuration
reference, exact selected binding, actual native instance identity, generation and
incarnation, service, tick and read reference. The Host observes the existing
durable native journal and denies terminal/pending recovery, non-Running, changed
generation or unresolved native operations. No replay store or scope runtime is
reopened. Observer clients are collected only for the retained owner; direct CLI
installation supplies no collector and creates no unused observer clients.

Execute returns the normal five-field `node-service-host-receipt-v1`, whose value
is `<node-service-host-scope-observed-v1 SERVICE READ CONFIG_REF ADMISSION_REF OBSERVATION>`.
`CONFIG_REF` is `effects::config::reference` of the entire selected configuration;
`ADMISSION_REF` identifies the actual current host authorization receipt, not the
bundle. `OBSERVATION` is the data-only canonical
`<collaboration-scope-observation-v1 READ STATE CURRENTNESS UNIX_SECONDS>` from an
actual quorum read in the retained scope worker. Its decoder cannot construct
`CurrentState`, and diagnostic observation does not consume the later protected
read. Protected mutation staging still performs its mandatory fresh probe and all
independent scope authority/state/currentness checks.

The independent producer obtains this observation **before** preparing/signing a
mutation, using its state/currentness/read and a scope credential interval covering
its Unix time. Publish scope operator trust at the actual applied logical index
before the final diagnostic read, then freeze that trust while signing/delivering:
rewriting trust changes its revocation identity and stales the request. Only
bounded metadata observation attempts may be retried with fresh reads and exact
grants; an uncertain protected delivery must never be automatically repeated.

## Acceptance and evidence

The installed-host fixture must use a genuine materialized native callback and
actual test authority, submit through the public host, observe its real Raft
proposal and committed scope-state transition, and observe the native completion
callback. Negative cases include missing/wrong configuration, stale generation,
wrong port/group/policy/resource, denied current scope authority, uncommitted
proposal and failure after possible effect. Real process restart retains the
same journal/application uncertainty boundary. Replacement proof also requires
the actual verifier-persisted admission for its fresh exact installation request,
unchanged semantic Scope replay reservation keys/digests, and no new protected
proposal or completion. The replacement keeps the same unresolved predecessor
effect, consumed-completion inventory and generation/incarnation fence; its own
activation may re-observe the instance, which is not a retry of that effect.
Install admission is proven by the exact credential request digest and invocation
nonce plus the replay key; the persisted replay value is basalt's canonical
holder-invocation digest, not the producer request reference.
Reopen comparisons cover append/truncate/commit records directly, not election
hard state, flush markers or snapshots. Replay readback uses the durable ordered
state, not its incidental journal layout. No acceptance row is complete without the
parent-run integrated source cohort and real observed proof.
