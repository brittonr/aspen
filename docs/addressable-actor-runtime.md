# Addressable actor runtime profile

## Scope

The addressable actor is a versioned system-extension profile. It composes existing Molten fabric mechanisms. It does not add another mailbox, store, scheduler, placement service, transport, authority engine, or evidence engine.

The profile uses canonical actor keys. A key contains a namespace reference, an actor type, and a bounded key string. Molten derives the actor-key identity with a domain-separated BLAKE3 projection.

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

Unknown state blocks wake, delivery, and recovery work. An operator must supply an explicit resolution reference before checkpoint recovery can begin. The resolution does not authorize an automatic retry of the uncertain external effect.

## Storage and reconciliation

`LocalActorStore` stores canonical actor state in `addressable-actor.redb` under a capability-rooted storage namespace. Compare-and-commit binds the expected state reference, revision, and engine epoch.

If a commit result is unknown, the service reads the state back once. It classifies the result as applied, not applied, or unknown. It does not issue a blind retry.

## Evidence

Commit receipts bind the planned state, final state, effect-admission observations, effect outcomes, status observation, currentness, durability, and engine epoch. Receipts and status do not grant mutation, effect, retry, activation, release, or production authority.

Deterministic simulation uses the same pure transition planner and service shell with in-memory ports. The child-process fixture reopens the Redb state and proves that an old generation cannot wake after replacement.

## Reference boundary

The design review used `rivet-dev/actors` at revision
`71f371ba4eab1234d8b6b6c419e6748cc6fc9911` under Apache-2.0.

The review selected keyed addressability, generation fencing, sleep and rewake separation, and explicit persisted classes. It did not adopt Rivet APIs, storage formats, TypeScript behavior, benchmark values, global-key claims, transport behavior, or service guarantees.
