## Context

Molten has most prerequisite pieces but not one coherent live-binding and retirement contract:

- artifact name views point to immutable refs, but normative use currently pins exact refs and does not define an explicit late-bound runtime boundary;
- system extensions, timers, transport sessions, durable state, tasks, effects, and callbacks carry generations, but cleanup is subsystem-specific;
- retention reference indexes protect content, but they answer deletion eligibility rather than whether a runtime generation is still reachable;
- effect manifests and handler profiles bind effect ids, operation strings, schemas, capabilities, resources, and evidence, but they do not yet require Kamacite semantic operation keys that re-key on behavior drift.

The adoption must preserve Molten's stronger authority and evidence posture. Shared binding mechanics and Kamacite identities are inputs; neither may execute a swap, authorize a handler, declare roots complete, delete content, or mint Molten runtime evidence.

## Decisions

### Decision: Split lower ownership across two providers

**Choice:** Consume `artifact-binding-core` for generic binding/reachability mechanics and Kamacite for canonical semantic effect-operation identity. Molten owns Preserves projections, runtime root collection, interface/effect admission, atomic registry mutation, evidence, deployment, retention, and GC.

**Rationale:** The mechanisms have different semantic owners. Combining them in one shared crate would make artifact binding interpret effects or make Kamacite own runtime generations.

### Decision: Adopt only immutable reviewed revisions

**Choice:** Development may use explicit sibling overrides, but permanent Cargo/Nix adoption requires aligned immutable source revisions and compatibility fixtures. No ambient path or floating branch becomes release evidence.

**Rationale:** This follows existing stack extraction/adoption rules and keeps source identity reviewable.

### Decision: Add Molten-owned canonical wrappers

**Choice:** Define canonical Preserves artifacts for binding records, snapshots, resolution receipts, transition receipts, root inventories, generation attribution, retirement reports, and deploy diagnostics. These artifacts wrap shared pure outputs but remain Molten schemas.

**Rationale:** The shared crate deliberately defines no wire format or receipt. Molten needs replayable and auditable product evidence.

### Decision: Resolve once per explicit unit of work

**Choice:** Late binding is opt-in and occurs at the top of an admitted request, turn, callback pass, job, or protocol session. Resolution returns one exact artifact and dependency closure pinned below that boundary. Nested late binding requires another explicit operation and appears in effect/replay evidence.

**Rationale:** Resolving every internal call would destroy subtree consistency and replay identity. Never resolving would provide no live cutover.

### Decision: Cutover uses load-validate-swap

**Choice:** The shell loads and verifies successor artifacts, computes product compatibility and migration obligations, validates the shared compare-and-swap plan, runs authority/policy/provenance/resource/lifecycle gates, and finally publishes one new binding revision atomically. Failures leave the old binding current.

**Rationale:** The pure core cannot prove I/O or atomicity. The product shell must own mutation and receipts.

Rollback is another successor revision targeting a prior immutable artifact, subject to current state and migration policy.

### Decision: Root registration is mandatory for retirement claims

**Choice:** Every execution profile that can hold generation refs must expose registered roots and dependency edges under one inventory snapshot. Known root classes include active callbacks/execution units, sessions, tasks, cells/durable values, queues, timers, registries, effect handles, snapshots, rollback/retention pins, and remote/cache pins.

**Rationale:** Molten cannot safely scan arbitrary native stacks, remote heaps, or uninstrumented plugin state. If a profile cannot account for holders, retirement stays incomplete.

### Decision: Separate runtime retirement from content retention

**Choice:** A generation can be retired while its artifacts remain retained for rollback, legal hold, replay, evidence, remote uncertainty, or policy. Retirement reports feed but never replace existing retention plan/apply/execute/audit gates.

**Rationale:** Runtime liveness and deletion authority are different questions.

### Decision: Attribute shared and exclusive artifacts explicitly

**Choice:** Retirement inputs distinguish generation-scoped roots, generation-exclusive artifacts, and immutable refs shared by multiple generations. Ambiguous attribution blocks retirement.

**Rationale:** Unchanged content should not falsely pin an old generation, but missing ownership facts must not produce false retirement.

### Decision: Consume exact Kamacite semantic operation keys

**Choice:** Effect manifests, handler bindings, handles, requests, responses, logs, replay/cache identities, remote execution, and upgrade diagnostics bind exact Kamacite semantic operation identities. Exact equality is the default match.

Directional compatibility requires both a Kamacite compatibility artifact and Molten-owned contextual admission. A replay-only compatibility artifact cannot authorize live execution.

### Decision: Emit actionable but non-authoritative diagnostics

**Choice:** Reports identify stale binding revisions, incompatible target contracts, unreachable successors, semantic handler mismatches, missing root classes, ambiguous attribution, and stable root-to-artifact pin paths.

**Rationale:** Operators need concrete blockers, but reports must not kill work, authorize migration, or delete content.

## Rollout

1. Publish and pilot `artifact-binding-core`; publish Kamacite semantic operation descriptors and migration fixtures.
2. Add Molten pure adapters and compatibility tests without changing runtime dispatch.
3. Add semantic operation checks to effects, replay, cache, remote execution, and upgrade gates.
4. Add opt-in late binding for one system-extension service profile.
5. Add root inventory and retirement reporting in observation-only mode.
6. Integrate deploy diagnostics and retention handoff only after negative incomplete-root and stale-CAS rails pass.

## Risks / Trade-offs

- **False retirement from missing roots:** Mitigate by requiring profile-scoped completeness and returning incomplete for uninstrumented native/remote holders.
- **Extra runtime bookkeeping:** Root registration and attribution add state. Keep observations bounded and reuse existing generation/session/timer/retention inventories where possible.
- **Identity migration churn:** Kamacite semantic keys will invalidate caches and fixtures. Version and preview the migration; never reinterpret old refs.
- **Hot-swap overreach:** Operators may expect arbitrary in-flight replacement. State explicitly that old work remains pinned and only explicit new resolution boundaries see successors.
- **Authority confusion:** A resolved ref or matching operation key may look trusted. Preserve all existing capability, policy, provenance, resource, handle, source-gate, and lifecycle gates.
- **Shared dependency drift:** Pin Cargo and Nix to identical immutable revisions and keep Molten fallback/rollback until adoption evidence passes.
