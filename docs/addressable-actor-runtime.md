# Addressable actor runtime profile

## Scope

The addressable actor is a versioned system-extension profile. It composes existing Aspen fabric mechanisms. It does not add another mailbox, store, scheduler, placement service, transport, authority engine, or evidence engine. Existing `molten` crate, CLI and canonical schema names retain the internal codename.

The profile uses canonical actor keys. A key contains a namespace reference, an actor type, and a bounded key string. Aspen derives the actor-key identity with a domain-separated BLAKE3 projection.

## Ownership

The pure core in `crates/molten-core/src/addressable_actor/` owns these rules:

- actor-key and profile identity;
- the actor lifecycle view;
- wake and callback fencing;
- the survival matrix;
- deterministic transition and effect-intent planning;
- explicit unknown external outcomes;
- bounded read-only status projection.

The shell in `src/addressable_actor/` owns these effects:

- capability-rooted Redb state storage;
- compare-and-commit reconciliation;
- a fresh admission check before each effect;
- typed effect execution through a port;
- status publication;
- canonical Preserves receipts;
- restart and child-process test adapters.

Existing system-extension, placement, coordination-delivery, durable-state, logical-time, resource, supervision, authority, and evidence components keep their current ownership.

## Lifecycle

The actor profile uses these states:

| State | Meaning |
|---|---|
| `dormant` | Durable facts can exist, but actor runtime resources are absent. |
| `starting` | A generation-bound wake plan is active. |
| `running` | The current generation can receive admitted work. |
| `draining` | New work is blocked while bounded current work completes. |
| `stopped` | The profile does not accept another wake. |
| `degraded` | Recovery or operator action is required. |
| `recovering` | A current checkpoint restore is in progress. |

Each request binds the actor key, placement, extension generation, expected lifecycle sequence, profile, and system-extension manifest. A mismatch preserves state and emits no actor effects.

The actor state maps to the generic system-extension lifecycle. For example, `dormant` maps to `drained`, and `running` maps to `running`. The shell rejects a mismatched generic lifecycle observation.

## Wake behavior

A dormant actor can wake from an admitted message, logical timer, new connection, or operator request. A wake plan can contain these ordered intents:

1. restore the selected checkpoint, if one exists;
2. start the runtime for the current generation;
3. deliver the message or invoke the selected wake reason.

The plan is not execution authority. Before each intent, the shell reads current policy, capability, placement, generation, resource, and adapter admission facts. If any fact changed, the shell denies that intent before execution.

A connection wake creates a new runtime connection. It does not claim that a previous stream or session survived.

## Sleep and drain

Idle sleep uses admitted logical time only. The profile requires an exact idle threshold. Sleep is denied while mailbox items or unresolved effects remain.

A successful sleep records a checkpoint reference and plans checkpoint publication before runtime stop. A bounded drain must report zero remaining items before the profile enters `stopped`.

## Survival matrix

The `v1` matrix is closed:

| Class | Disposition |
|---|---|
| durable state | durable |
| admitted mailbox entries | durable |
| completed semantic events | durable |
| selected checkpoints | durable |
| processes | runtime-only |
| streams | runtime-only |
| sessions | runtime-only |
| partial callbacks | unsupported |
| in-flight deltas | unsupported |

A recovery result can name only classes marked `durable`. A checkpoint reference does not prove that any class survived. The restore adapter must provide the matching bounded evidence.

## Delivery completion

The profile consumes coordination-delivery tokens and item references. It plans an acknowledgement only after the actor supplies a durable semantic-event commit reference.

Duplicate completed-event references do not emit another acknowledgement. The delivery profile remains at-least-once. The actor profile does not claim exactly-once external effects.

## Unknown effects

If an external effect can have occurred without terminal evidence, the shell records the effect reference as unknown and moves the actor to `degraded`. It stops the remaining effect plan.

In the native counter adapter, preparation, read and identity failures before
any mutating owner call remain definite failures. Once such a call is attempted,
later preparation, publication or validation errors preserve uncertainty; they
do not authorize retry.

Unknown state blocks wake, delivery, and recovery work. An operator must supply an explicit resolution reference before checkpoint recovery can begin. The resolution does not authorize an automatic retry of the uncertain external effect.

## Storage and reconciliation

`addressable_actor::Local` stores canonical actor state in `addressable-actor.redb` under a capability-rooted storage namespace. Compare-and-commit binds the expected state reference, revision, and engine epoch.

If a commit result is unknown, the service reads the state back once. It classifies the result as applied, not applied, or unknown. It does not issue a blind retry.

## Evidence

Commit receipts bind the planned state, final state, effect-admission observations, effect outcomes, status observation, currentness, durability, and engine epoch. Receipts and status do not grant mutation, effect, retry, activation, release, or production authority.

Deterministic simulation uses the same pure transition planner and service shell with in-memory ports. The child-process fixture reopens the Redb state and proves that an old generation cannot wake after replacement.

## Public-node counter composition

`addressable_actor::native::Consumer` is the library composition over a retained source-admitted `node_service_host::installed::Owner`, actual `fabric_membership::persistence::Redb`, the existing actor `Local`, and `delivery::Bridge` backed by the existing coordination-delivery `Local`. `Consumer::open(native::Open { storage, engine_epoch, configuration, owner, placement, delivery, signing })` groups the existing explicit construction inputs; `Consumer::apply(native::Apply { fence, operation_id, operation, logical_tick, work, acknowledgement })` retains the current-operation fence. Native instance records provide current generation/incarnation, resource usage and exact manifest/profile bindings; the assignment owner supplies committed current placement. Current policy, credential expiry, revocation and holder-invocation replay are independently checked before protected operations. Profile or actor identity never grants authority.

The measured `molten-addressable-counter` guest is effect-free application code: a request changes canonical keyed counter state through admitted native callback publication and the existing durable native journal. It keeps at most 128 completed logical IDs and their original results. It rejects a changed payload under an old ID, wrong actor key, overflow and exhausted retention rather than silently discarding deduplication history. This bounded application state is not another mailbox or semantic persistence backend.

The delivery extension is a distinct admitted, always-running native member. Binding it to the sleeping actor's Drained lifecycle would prevent a durable message from waking that actor; the consumer does not bypass that guard. Enqueue/claim/acknowledgement use its real policy, clock/timer profile and durable queue. Acknowledgement additionally checks the actor's original successful callback receipt bytes and exact completed native journal operations. Those original receipts are published through the native owner's existing value port before terminal ingress, so proof reconstruction after process reopening does not invent a successful callback.

Sleep runs one checkpoint callback followed by one drain callback, not terminal shutdown. The checkpoint must equal the selected canonical counter state. Mailbox and unresolved-operation counts come from actual stores, not caller-authored zeroes. Rewake records runtime loss and recovers the exact checkpoint with a fresh incarnation and unchanged generation. A subsequent start intent observes the already Running native postcondition without invoking the guest again. Process, stream and session continuity are not restored. Quarantined dead letters remain retained but do not represent runnable mailbox work.

Public results distinguish `applied`, `denied`, `stale`, `unknown`, `duplicate-replay` and lifecycle `progress`. A persisted wake plan or successful callback alone is progress, never durable semantic completion. `Consumer::result` binds the original native event/commit/state, actor completed-event metadata, the retained successful canonical actor completion receipt and its exact acknowledgement request/token, and the actual terminal delivery row. The existing actor `Local` retains that immutable completion evidence only after the original actor completion succeeds; it does not copy native semantic state or confer authority. Repeating a completed logical request observes that original result without another callback, acknowledgement or semantic commit. Unknown state does not authorize an implicit retry.

Raw completion replay is not semantic success merely because actor metadata
already names the event. The original-result read additionally binds the exact
acknowledgement request, token and terminal mailbox row; a refused or absent
original acknowledgement remains `unknown` with reconciliation required even
if a separately authorized acknowledgement later completes that payload.
Missing native completion returns `unknown`; failed reads, malformed receipts
and inconsistent durable evidence remain errors. This read never dispatches
another callback or acknowledgement. Result-observation v2 includes the original
acknowledgement identity and canonical actor completion receipt identity.
The three required native event/commit/state references may be ordered freely
and accompanied by additional admitted evidence, matching acknowledgement
admission. The exact retained original request identity remains mandatory.
Unsupported survival claims exit nonzero; the negative driver accepts that
denial transport and still compares actual native state before and after each case.

The request command previews the existing delivery planner before claiming a
newly enqueued payload. If FIFO selects older ready work, the new request returns
queued `progress` without claiming or executing that older work under the new
request's authority. Claim admission remains fenced by the exact queue state.
An exact, exclusively ready payload already retained by the delivery owner is
not enqueued again on a later attempt. It resumes through the same current FIFO
preview and independently authorized fenced claim; altered, active, terminal
or ambiguous rows are not treated as fresh queued work.

### Pinned runnable flow

Build the reviewed immutable source producer with the repository's locked Nix inputs. This build runs the real source-admission producer; a local Cargo binary or a fabricated receipt cannot replace it:

```sh
nix build .#molten-startup-source-binding --no-update-lock-file -L
```

Have the independently authorized source authority sign/select that exact realized input-addressed output and provision `$TRUST` as described in [node-state filesystem authority](node-state-filesystem-authority.md#startup-source-evidence-and-operator-refresh). `$SIGNED_OUTPUT` must be that exact output and contain `bin/molten`, `bin/molten-service-host-cohort` and `bin/molten-addressable-counter`. The explicit experimental policy must authorize the selected actor/delivery/assignment operations; `$ISSUER_SEED` and `$HOLDER_SEED` remain externally held raw Ed25519 seed files, never fixture outputs. `$PLACEMENT_JSON` is an independently reviewed proposal input containing the actual plan, resource reservation, fencing and authority references, not a generated Active record.

```sh
"$SIGNED_OUTPUT/bin/molten-service-host-cohort" create \
  --profile counter --source-trust "$TRUST" --signed-output "$SIGNED_OUTPUT" \
  --issuer-key "$ISSUER_SEED" --holder-key "$HOLDER_SEED" --policy "$EXPERIMENTAL_POLICY" \
  --output-parent "$COHORT_PARENT" --name counter \
  --actor-namespace-ref "$NAMESPACE_REF" --actor-type counter --actor-key tenant:one \
  --counter-placement "$PLACEMENT_JSON"

python3 "$COHORT_PARENT/counter/counter-flow.py" \
  --source-trust "$TRUST" --signed-output "$SIGNED_OUTPUT" \
  --issuer-key "$ISSUER_SEED" --holder-key "$HOLDER_SEED" --policy "$EXPERIMENTAL_POLICY"
```

The authored script drives the real `molten addressable-actor` command through independent processes. It installs both exact manifests, commits Proposed-to-Active placement through the existing assignment shell and actual native lifecycle, initializes from an actual drained checkpoint, submits and reads a request, checkpoints/sleeps, exits the host, reopens/restores/wakes for another request, and reads the first original result again. Each `SIGN` line is an exact current tuple for the external authorizer; a `READY` handshake only permits the existing verifier to examine the credential. The script writes `counter-execution.json` exclusively only after all assertions actually complete. Authoring that script does not create this execution artifact or prove the flow passed.

Full qualification additionally requires three separately authored fresh cohorts using the same exact source selection and explicitly approved placement inputs: one for the interrupted host, one for admission boundaries, and one for older-ready/new-request FIFO isolation. Do not copy a positive node's durable state or infer a grant from its success. Invoke the positive flow only once on its fresh root, supplying the independent negative roots:

```sh
python3 "$POSITIVE_COHORT/counter-flow.py" \
  --source-trust "$TRUST" --signed-output "$SIGNED_OUTPUT" \
  --issuer-key "$ISSUER_SEED" --holder-key "$HOLDER_SEED" --policy "$EXPERIMENTAL_POLICY" \
  --negative-cohort "$UNKNOWN_COHORT" \
  --boundary-cohort "$BOUNDARY_COHORT" --qualify-revocation \
  --ordering-cohort "$ORDERING_COHORT"
```

The positive flow reads a pending request before native completion and requires
`unknown` without a native-state/callback change. It also reads the first result
after later work and compares its retained original evidence with duplicate
replay. The ordering cohort enqueues X before submitting Y, requires queued
progress with no semantic effect, and then independently claims X to establish
that Y did not consume its claim. The refused-original/separately authorized
acknowledgement boundary is covered through the existing delivery planner,
actual delivery/actor `Local` owners, bridge and shared consumer result-evidence
observer; the public command adds no independent acknowledgement/retry surface.

The negative driver SIGKILLs the actual host at its native-commit signing boundary and requires explicit uncertainty on reopening without redispatch. Boundary requests carry explicit stale fences or unsupported survival claims and compare native state/callback observations before and after rejection. Revocation qualification uses the external authorizer's `--revoked-trust-output` option to export canonical trust containing the actual signed capability's CID; the external test controller installs it before `READY`, and the host's existing Current verifier must deny it before a protected callback. No host admin shortcut or successful-admission flag substitutes for that check. Expected state/checkpoint fixtures are assertions, never observations until compared with real outputs. Failed runs preserve their evidence and require newly authored roots rather than destructive resets or retries.

### Qualification boundary

The command and cohort producer require an independently selected, authenticated signed immutable source output and separately provisioned experimental operator authority. Interactive `SIGN`/`READY` transports exact current-operation credentials; `READY` is not permission, and the host never receives signing seeds or invents passing source evidence. Authored configuration and canonical references are not runtime evidence.

No positive/negative native cohort execution is claimed by this implementation record. Parent verification must retain the actual source/output/configuration, per-operation credentials and canonical receipts for init/install/request/result/checkpoint/sleep, host process exit, restore/wake, replay and stale/key/placement/revocation/survival negatives. The recovery worktree's tmpfs location provides no machine-loss or NVMe durability evidence.


## Reference boundary

The design review used `rivet-dev/actors` at revision
`71f371ba4eab1234d8b6b6c419e6748cc6fc9911` under Apache-2.0.

The review selected keyed addressability, generation fencing, sleep and rewake separation, and explicit persisted classes. It did not adopt Rivet APIs, storage formats, TypeScript behavior, benchmark values, global-key claims, transport behavior, or service guarantees.
