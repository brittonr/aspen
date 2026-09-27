# Exact replay and bounded capsules

World replay compares an observed transition chain against exact expected world commits. It is stronger than checking a final application value, but narrower than proving universal determinism. This article assumes familiarity with world commit roots and snapshot profiles. The [replay capsule contract](../../world-replay-capsules.md) is authoritative; the [Technical companion](../README.md) connects replay to promotion, restore, and retention.

## A trace is an ordered commitment

`WorldTransitionTrace` names an initial commit and an ordered list of steps. Each step binds its position, expected parent, input kind, input schema and byte length, replay profile, and expected successor. Contiguous positions and parent-to-prior-successor equality prevent an apparently successful suffix from concealing a missing transition.

A command input, event input, and recorded-effect input have different provenance even when they eventually affect the same root. In particular, replaying recorded effect knowledge does not authorize a new external dispatch. The [promotion contract](../../world-promotion-and-effect-release.md) separately governs the acknowledged observation needed for a logical recorded-effect successor.

The shell's [replay runner](../../../src/world_replay/service/run.rs) first checks the supplied initial commit against the trace, publishes canonical input records, materializes members, restores the selected profile, and obtains current admission. A denial produces a bounded denial receipt with no executed transitions. Publication of the trace or plan therefore does not mean execution was admitted.

## Why comparison stops at the earliest mismatch

For each admitted step, the runner executes the transition, captures its successor, and compares that captured world with the expectation. A divergence record is published and the loop stops immediately. The receipt's matched horizon counts the successful prefix; the captured divergent transition is evidence, not another matched step.

Illustratively, consider the chain W0 → W1 → W2 → W3. At the second transition, the durable-state root matches W2 but the scheduler root differs. A comparison limited to business records would miss the discrepancy. Complete-world comparison reports the earliest differing root and a bounded reference-based path, as specified by the [trace contract](../../world-replay-capsules.md).

The third transition is not executed to see whether the system “converges back.” Doing so would introduce an execution outside the verified prefix and make later results ambiguous: they would follow the actual divergent state, not the expected parent. Earliest-divergence reporting is therefore both diagnostic discipline and an execution boundary. It does not claim that the reported root is the ultimate causal origin of the defect.

## A capsule binds roles, not transport locations

A capsule packages a complete bounded closure for its trace: commits, typed roots, transition inputs, runtime profiles and cohorts, snapshot descriptors where required, and the declared artifacts, schemas, policies, manifests, and sealed reproduction bundles. Each member binds a reference, roles, codec, byte length, and protection profile. Canonical member and role ordering prevents transport order from becoming semantic identity.

Locator hints and transport tickets remain detached. Receiving bytes from a working locator is not evidence that every declared role is present or that replay can decrypt protected members. Existing content manifests and sealed bundles keep their own meanings; the capsule assigns world-specific roles without replacing their formats.

Opaque replay chooses `restore_opaque_exact`; logical replay chooses `restore_logical`. The runner has no fallback branch between them. The [snapshot contract](../../world-execution-snapshots.md) explains why exact opaque cohort compatibility cannot be replaced with approximate logical equivalence.

## Import has a publication boundary

The [import service](../../../src/world_replay/service/import.rs) plans the request, builds the supplied payload map, and reviews every declared member before staging. It checks payload length directly and inspects validation observations for reference and length agreement, canonical form, identity verification, sensitive plaintext, bearer material, and ciphertext decryption availability. Undeclared payloads also produce diagnostics.

If review diagnostics exist, import emits a denial receipt with no staged references. Once the complete verification pass succeeds, the shell stages members and calls `publish_available` for the complete staged set. This is a visibility boundary, not a claim that every port operation is a single storage transaction. A staging-port failure may occur after earlier staging effects; the inspected orchestration does not establish automatic rollback.

A valid ciphertext capsule can remain blocked when current decryption authority is absent. Content validity and replay availability are different predicates. Import receipts explicitly deny branch movement, runtime activation, and authority grants, so availability cannot be reused as an activation capability.

## Verification and limits

Suggested verification, not executed evidence: inspect the [existing import tests](../../../src/world_replay/tests/import.rs) for tampered, missing, extra, sensitive, bearer-bearing, and unavailable ciphertext members. Exercise a multi-step replay with a deliberate earlier root divergence and observe that later transition ports are not called. Check the matched horizon and divergence reference rather than relying on a rendered “replayed” message.

Canonical Preserves records and domain-separated identities bind precise evidence; terminal logs remain diagnostic views. A successful bounded replay demonstrates the inspected trace under its exact profile, dependencies, admission, and observations. It does not prove all possible executions deterministic, transfer capabilities, discharge external effects, establish logical/opaque equivalence, or confer release eligibility.

## Sources

- [World replay capsules](../../world-replay-capsules.md)
- [World execution snapshot profiles](../../world-execution-snapshots.md)
- [World promotion and effect release](../../world-promotion-and-effect-release.md)
- [Replay orchestration](../../../src/world_replay/service/run.rs)
- [Import verification and publication](../../../src/world_replay/service/import.rs)
- [Import boundary tests](../../../src/world_replay/tests/import.rs)
- [Technical companion](../README.md)
