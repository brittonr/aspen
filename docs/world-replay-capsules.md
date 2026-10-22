# World replay capsules

Molten binds deterministic replay to exact world-commit transitions. A trace names the expected parent and successor for every step.

A capsule packages the complete bounded closure for one trace. Capsule possession does not grant runtime authority.

## Ownership

The functional core owns:

- trace shape and bounds;
- expected parent and successor order;
- complete typed-closure requirements;
- stable BLAKE3 identities;
- replay operation order;
- earliest-divergence meaning;
- receipt claim limits.

The shell owns:

- content materialization;
- logical or opaque restore;
- current admission checks;
- transition execution;
- successor capture;
- import staging and publication;
- record storage and exchange.

Existing content manifests and sealed reproduction bundles remain separate mechanisms. The capsule gives them world-specific roles without replacing their formats.

## Transition traces

`WorldTransitionTrace` contains one initial commit and an ordered step list. Each step binds these values:

- position;
- expected parent commit;
- command, event, or recorded-effect input;
- input schema and byte length;
- replay profile;
- expected successor commit.

Positions must be contiguous. Each expected parent must equal the prior expected successor.

`compare_world_replay` stops at the first mismatch. It does not execute or classify later steps after that mismatch.

A divergence record names the step, expected and actual commits, first differing root, and a bounded field path. It contains references only.

## Capsule closure

Each member binds an object reference, one or more roles, a codec, a byte length, and a protection profile.

The complete closure includes:

- the trace;
- every initial and expected commit;
- every typed root;
- transition inputs;
- runtime profiles and cohorts;
- opaque snapshot descriptors;
- required artifacts, schemas, policies, and content manifests;
- required sealed reproduction bundles;
- the source inventory, the observation ledger, and each detached opaque diagnostic.

Member order and role order are canonical. Duplicate, missing, and undeclared roles fail validation.

Locator hints and transport tickets stay outside the capsule identity. Transport completion does not establish replay readiness.

## Replay flow

`plan_world_replay` emits this deterministic operation order:

1. Materialize every declared member.
2. Admit the observation boundary. Planning has already validated it, and the plan binds its inventory and ledger references.
3. Restore the exact logical or opaque profile.
4. Recheck current authority, artifact, schema, resource, runtime, and effect admission.
5. Execute one transition against the sealed observations.
6. Capture its complete successor commit.
7. Compare that commit with the expected successor.
8. Stop on the first divergence, or continue to the next step.
9. Publish a bounded replay receipt.

Opaque replay requires the exact cohort and snapshot descriptor. It never falls back to logical restore.

A denied current admission emits a denial receipt before transition execution.

## Observation boundary

r[impl molten.world_replay_boundary.claims] Replay records nondeterminism at Molten's semantic host boundary: the runtime hostcalls, typed effect ports, scheduler decisions, asynchronous deliveries, clocks, entropy streams, external reads, and runtime observations a transition consumes. Native syscalls inside an admitted runtime are outside logical replay. A ChaosControl native-process replay trace may answer a forensic question, but it does not define a world transition.

Each replay profile carries a closed source inventory (`world_replay::boundary`). Every source declares one handling class:

- **deterministic:** a pure function of replayed state, such as a scheduler decision under the deterministic policy;
- **simulated:** reproduced by a seeded or virtual model, such as virtual time, the seeded entropy stream, timer firings, and declared fault activations;
- **recorded observation:** replay returns the sealed recorded value, as for external reads, deliveries, hostcalls, and every effect port;
- **unsupported:** the profile cannot replay the source.

The runtime catalog (`src/world_replay/boundary/sources.rs`) binds each existing path to a source identifier with an exhaustive match: `RuntimeHostcall` capabilities, `EffectKind` ports, `SchedulerChoiceKind` deliveries, the scheduler replay policy, the virtual clock, and the entropy stream. A new variant cannot reach replay without a handling decision. An external mutation is always a recorded observation or unsupported; the inventory rejects one declared deterministic or simulated.

The observation ledger names the sources the recording reached, the slots each transition consumed, and the recorded observations. Each observation binds its source, transition step, logical position, operation, canonical request, result or error class with its length, protection, protection profile, adapter cohort, and a domain-separated BLAKE3 identity. The ledger binds its inventory and trace.

Slots order by step, then by explicit logical position. Wall-clock arrival is kept as an observation and never orders anything; thread wake order and adapter iteration order are not inputs.

`admit_replay_observations` decides completeness before any transition runs, and the shell cannot waive its result. It blocks replay with an exact reason for:

- an unknown or unsupported reached source;
- a missing, extra, or duplicate observation;
- an observation bound to another request or source;
- the earliest ordering divergence;
- profile, adapter, or protection drift;
- a sensitive value stored as public plaintext;
- a tampered observation or ledger;
- an opaque diagnostic outside its admitted cohort, or one that claims anything but detached diagnostics.

During replay, transitions read observations only through `SealedReplayObservations`. It serves the next slot in logical order from the admitted ledger and fails the replay on an unknown source, an out-of-order demand, a demand bound to another request, or a slot left unconsumed. Every attempt to execute an original write, send, publish, release, or remote call is denied and fails the replay. Current effect release still requires the reservation and promotion protocol in [world promotion and effect release](world-promotion-and-effect-release.md).

An opaque diagnostic attaches as a capsule member under the one opaque-diagnostic cohort the inventory admits. It cannot satisfy a missing observation, become a world root, grant authority, or establish semantic equivalence. Binding a specific ChaosControl native-process descriptor format waits for that profile contract.

Trace members are identified by content and length. A path, shared inode, hard link, or reflink is a locator, not an identity. `plan_replay_member_sharing` keeps a reflink or deduplicated copy only after the source identity, immutability, sharing policy, and destination readback all pass; a mismatched source or readback blocks the member.

## Import and export

`export_world_replay_capsule` exports only declared members. Exchange observations and locator hints remain detached.

`import_world_replay_capsule` verifies every member before publication. It checks canonical form, identity, length, protection, plaintext-secret markers, bearer material, and decryption availability.

Failed verification publishes a denial receipt. It does not stage or publish availability.

A passing import stages verified members first. It publishes one availability reference only after the complete set passes.

Import never moves a branch, activates a runtime, or grants authority.

## Protection profiles

Public members contain only admitted public data. Ciphertext members bind a protection descriptor.

Ciphertext can remain unavailable when current decryption authority is absent. The capsule remains valid content, but replay remains blocked.

Plaintext secrets, bearer capabilities, private keys, credentials, and live handles are not capsule members.

## Receipts and non-claims

Replay and import receipts bind exact trace, capsule, profile, horizon, dependencies, observations, divergence, and diagnostics. A replay receipt also states its observation boundary: inventory and ledger, handling-class counts, observation horizon, adapter and runtime cohorts, sealed effects and repeated-effect attempts, opaque diagnostics, redaction, its one claim (logical replay of that profile), and the boundary non-claims. Canonicalization rejects a receipt that claims more, drops a non-claim, or claims replay after an effect attempt.

They do not prove:

- universal determinism;
- logical and opaque semantic equivalence;
- capability or authority transfer;
- external effect completion;
- branch movement or runtime activation during import;
- release eligibility.

Canonical Preserves records use domain-separated BLAKE3 identities. Rendered logs remain diagnostic views.
