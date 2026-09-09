# Design: Admit typed service, state-machine, and task facades

## Context

The system-extension callback vocabulary already covers what `GenServer` structures: initialize, request, message, timer, health, checkpoint, recover, drain, shutdown. The addressable-actor profile already requires `current state + admitted event` turns with staged commits and bounds concurrency. The facades are a typed lowering on top; they add no semantic decisions the core does not already own.

## Approach

1. **Facade profiles as admitted manifests.** Each facade is a manifest profile that binds the event type, state type (via existing schema identity), reply and effect vocabularies, and profile-specific knobs: state-machine phase table and postponement bounds; task completion schema. Admission validates the table: every phase's legal events exist, postponement bounds are finite, and timers carry fencing requirements.
2. **Single lowering function.** The facade core is one pure function per profile: `(state, admitted event) -> (next state, replies, effect intents, timer intents)` or a typed rejection. The library lowers outputs into the canonical envelopes the extension interface already defines, byte-for-byte comparable with hand-written envelopes; an equivalence fixture proves it.
3. **State-machine semantics.** Phase-dependent event legality; state timeouts implemented as ordinary incarnation-fenced timers carrying the originating phase; postponement with an explicit per-phase bound, after which the event dead-letters through the existing delivery path instead of queueing forever.
4. **Serialization contract.** The facade documents and asserts that concurrent events to one logical service serialize through the existing single-writer turn commit; a concurrency-limit-above-one configuration partitions by key or funnels through the controlled commit, and a fixture proves two parallel events cannot interleave inside one state transition.
5. **Task completion.** A `Task` facade maps successful completion to the existing stopped transition, integrating with the restart-class semantics from `admit-recovery-group-supervision`.

## Alternatives considered

- A generic "actor" macro handling all three shapes via flags. Rejected: three narrow profiles with distinct admitted knobs are easier to validate, document, and deny at admission.
- Building facades on the addressable-actor profile only. Rejected as the only surface: system extensions without actor keys also deserve the structure; the facade lowers to the extension interface, which the actor profile itself composes.
- Making the facades the default authoring path. Rejected: opt-in keeps ordinary pure functions ordinary and avoids churn in existing extensions.

## Non-claims

- Facade use does not change failure isolation, authority, or durability guarantees; those remain owned by the existing contracts.
- Resemblance to `GenServer` or `gen_statem` is conceptual prior art, not compatibility.

## Risks

- The lowering must track canonical envelope evolution; the equivalence fixture is the tripwire.
- Bounded postponement interacts with timer fencing; fixtures must cover a postponed event plus a stale-state timer arriving together.
