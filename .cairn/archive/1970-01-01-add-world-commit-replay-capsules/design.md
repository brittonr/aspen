## Context

World commits provide immutable causal identities. Existing replay summaries compare selected semantic boundaries, but they do not prove that every replay step produced the expected complete world identity.

Existing simulation exports also package bounded artifacts. A world replay capsule needs a stricter closure contract without replacing content storage, replication, or evidence owners.

## Decisions

### Decision: Bind every transition to an expected successor commit

**Choice:** Define `world-transition-trace-v1` with one initial commit and a bounded ordered sequence. Each step binds its command or event input, deterministic profile, expected parent, and exact expected successor commit.

Replay captures an actual successor commit after each step. Verification stops at the earliest mismatch.

**Rationale:** Comparing only the final state can hide compensating or transient divergence. Per-step world identity gives one deterministic boundary across every typed root.

### Decision: Keep replay planning pure and execution in the shell

**Choice:** The core validates trace shape, bounds, ancestry, closure, profile compatibility, and expected transitions. It emits ordered restore, execute, capture, and compare intents.

The shell materializes objects, restores the selected profile, executes bounded work, and returns observations. It cannot declare success without core validation of the resulting commit.

**Rationale:** Replay meaning stays deterministic. Runtime, storage, and process effects remain visible application-owned capabilities.

### Decision: Report the earliest typed divergence

**Choice:** A mismatch record binds step index, expected and actual commit identities, first differing typed root, optional bounded field path, and refs-only context.

The record does not include secret bytes, raw bearer material, or unbounded state dumps.

**Rationale:** The earliest complete-world mismatch is more useful than a later summary mismatch and preserves the existing redaction boundary.

### Decision: Package one immutable closure manifest

**Choice:** Define `world-replay-capsule-v1` as a canonical manifest over the root trace, world commits, typed root objects, artifacts, schemas, policies, runtime cohorts, snapshot descriptors, transition inputs, and required content manifests.

Every member has a typed identity, byte length, codec, and role. Locator hints, mirrors, paths, and transport tickets remain detached.

**Rationale:** Portable replay needs a complete declared closure. It does not need another blob store or archive protocol.

### Decision: Reuse content and reproduction mechanisms

**Choice:** Export and import use existing Molten content manifests, sealed reproduction bundle mechanics, and content-exchange ports. The capsule adds world-specific closure and transition meaning only.

**Rationale:** Molten already owns bounded chunk verification, exchange, redaction, and simulation exports. Duplicating those mechanisms would weaken claim boundaries.

### Decision: Validate before imported content becomes usable

**Choice:** Import first validates manifest identity, member bounds, canonical codecs, complete closure, object identities, profile support, and secret policy. Only then can the shell publish availability records.

Import never activates a branch, restores a runtime, releases an effect, or grants authority.

**Rationale:** Transport completion is not replay readiness or execution admission.

### Decision: Keep private material and current authority detached

**Choice:** Capsules may contain ciphertext objects and protection descriptors under an explicit private profile. They never contain plaintext private keys, bearer capabilities, live handles, environment values, or implicit credential lookup instructions.

Restore and replay recheck current authority and recreate host handles through existing adapters.

**Rationale:** A portable snapshot must not become a portable authority grant.

## Rollout

1. Define trace and capsule DTOs, canonical codecs, and bounds.
2. Add observation-only closure and replay planning.
3. Verify one logical fixture step by step.
4. Add export and import round-trip through existing content adapters.
5. Add one exact opaque profile after ChaosControl publishes its descriptor contract.
6. Integrate the operator workflow only after positive and negative rails pass.

## Risks / Trade-offs

- Complete closures can be large. Chunk manifests and bounded streaming avoid unbounded assembly.
- Some external effects cannot replay. Profiles must use simulation, sealed observations, or explicit unsupported results.
- Opaque replay remains cohort-bound. A capsule does not make a VM snapshot portable across incompatible hosts.
- Encrypted members can be unavailable at replay time. Missing decryption authority remains a normal blocker.
- Exact commit comparison can expose previously hidden nondeterminism. That is a valid divergence result, not a reason to weaken the contract.
