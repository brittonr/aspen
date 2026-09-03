# Addressable actor portfolio search

## Goal

Select a bounded actor composition that adds keyed sleep and wake behavior without adding another persistence, mailbox, scheduler, placement, authority, or evidence core.

## Reviewed source

The design reference is `https://github.com/rivet-dev/actors` at immutable revision
`71f371ba4eab1234d8b6b6c419e6748cc6fc9911`. The repository uses Apache-2.0.

Reviewed file BLAKE3 identities:

- `LICENSE`: `9730ca2805f3a9f8b81e75ce828f611b26f01c762b1b4186976c5df18039d22e`;
- `engine/packages/pegboard/src/workflows/actor2/runtime.rs`: `8eaa12389fa10271cb51d54880834e2a62abab5eb83a089a2d53542bf7e5e100`;
- `engine/packages/pegboard/src/workflows/actor2/keys.rs`: `f67e0a25bdccaeaa8fed49f28c8ed007e184a2e9c80d500bf3bef184680c9cbd`;
- `rivetkit-typescript/packages/rivetkit/src/common/actor-persist-versioned.ts`: `60608ff1b2c7d71792c43c44c12323079e07291c2def0605adb424dc850ef5f5`.

The review used these concepts only:

- actor keys select an existing actor or reserve a new identity;
- runtime generations fence work after allocation changes;
- sleep intent and a wake received during shutdown remain separate facts;
- durable actor state and scheduled events are explicit persisted classes;
- runtime processes and ordinary live connections are not durable state by default.

The review did not adopt Rivet APIs, storage formats, benchmark values, global-key claims, transport behavior, retry policy, or service guarantees.

## Candidate A: Extend the generic system-extension state machine

This option would add dormant and actor-specific states to the generic system-extension lifecycle.

It was rejected. It would make actor meaning part of every extension and would weaken the current generic lifecycle boundary.

## Candidate B: Add a second actor runtime core

This option would create new mailbox, placement, persistence, scheduling, and supervision components.

It was rejected. Molten already owns these mechanisms. A second core would create competing authority and recovery semantics.

## Candidate C: Add a profile-specific functional core and thin shell

This option defines canonical actor keys, an actor view of lifecycle state, a closed survival matrix, and deterministic transition plans. The shell uses existing admitted fabric facts and executes only typed plans after current generation and authority checks.

This option was selected because it adds only the missing product composition. It keeps storage, delivery, time, placement, resources, supervision, authority, and evidence in their existing owners.

## Adversarial checks

The selected design must deny or quarantine these cases:

- stale actor key, placement, generation, lifecycle sequence, or profile;
- missing policy, capability, resource, placement, or adapter admission;
- duplicate wake requests;
- claims that processes, streams, sessions, callbacks, or in-flight deltas survived;
- an external effect whose terminal outcome is unknown;
- a commit or effect receipt used as mutation authority;
- an effect execution attempt without a fresh pre-effect admission observation.

## Result

Implement Candidate C. Treat all receipts and simulations as bounded evidence. They do not prove exactly-once effects, global key uniqueness, process survival, transport delivery, semantic correctness, or production readiness.
