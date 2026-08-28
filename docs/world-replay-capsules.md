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
- required sealed reproduction bundles.

Member order and role order are canonical. Duplicate, missing, and undeclared roles fail validation.

Locator hints and transport tickets stay outside the capsule identity. Transport completion does not establish replay readiness.

## Replay flow

`plan_world_replay` emits this deterministic operation order:

1. Materialize every declared member.
2. Restore the exact logical or opaque profile.
3. Recheck current authority, artifact, schema, resource, runtime, and effect admission.
4. Execute one transition.
5. Capture its complete successor commit.
6. Compare that commit with the expected successor.
7. Stop on the first divergence, or continue to the next step.
8. Publish a bounded replay receipt.

Opaque replay requires the exact cohort and snapshot descriptor. It never falls back to logical restore.

A denied current admission emits a denial receipt before transition execution.

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

Replay and import receipts bind exact trace, capsule, profile, horizon, dependencies, observations, divergence, and diagnostics.

They do not prove:

- universal determinism;
- logical and opaque semantic equivalence;
- capability or authority transfer;
- external effect completion;
- branch movement or runtime activation during import;
- release eligibility.

Canonical Preserves records use domain-separated BLAKE3 identities. Rendered logs remain diagnostic views.
