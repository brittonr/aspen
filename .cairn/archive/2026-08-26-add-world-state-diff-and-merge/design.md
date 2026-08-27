## Context

Branching creates useful alternatives only when operators can compare them. Some state classes can also combine safely under explicit application rules.

Molten durable values have canonical Preserves representations and schema identities. Choregraph has branch ancestry and merge envelopes. Schema Migration Core can classify declared migration paths.

No existing component owns semantic merge for Molten worlds. The implementation must avoid treating all content-addressed roots as interchangeable trees.

## Decisions

### Decision: Diff every root, merge only admitted roots

**Choice:** World comparison classifies every typed root as equal, changed, absent, unavailable, incompatible, or profile-excluded.

Merge planning accepts only roots with a declared merge mode and complete required inputs.

**Rationale:** Operators need full difference visibility. Merge authority must remain narrower than inspection.

### Decision: Use one exact common ancestor input

**Choice:** The caller supplies a Choregraph-derived common ancestor and declared source heads. Molten validates ancestry before loading merge inputs.

Ambiguous or missing ancestors block automatic merge.

**Rationale:** Guessing a base from timestamps or nearest available content can silently discard state.

### Decision: Define closed merge modes

**Choice:** Support these initial modes:

- `identical_only` accepts equal left and right roots.
- `ancestor_replacement` accepts one changed side when the other still equals the base.
- `keyed_durable_values` compares canonical keyed values under exact schemas.
- `application_handler` invokes one exact immutable pure handler profile.

Unknown modes fail closed.

**Rationale:** A small closed set is testable. It also leaves application semantics with explicit owners.

### Decision: Default runtime-sensitive roots to non-mergeable

**Choice:** Tasks, scheduler queues, authority observations, capability state, effect attempts, external observations, and opaque machine snapshots reject merge by default.

A later change can add a root-specific merge profile after it proves safe semantics and negative cases.

**Rationale:** Combining these roots can duplicate work, authority, or external effects.

### Decision: Gate values through schema identity and migration

**Choice:** Exact schema equality is the normal path. Differing schemas require an explicit Schema Migration Core plan and Molten-owned contextual admission.

Migration planning and merge planning remain separate. The shell materializes migrated canonical values before the pure merge handler runs.

**Rationale:** Shape similarity does not prove semantic compatibility.

### Decision: Bind application handlers by semantic identity

**Choice:** An application merge handler is an immutable artifact with exact input schemas, output schema, behavior identity, bounds, and policy identity.

The pure core receives already-loaded values and returns a merged value or typed conflicts. It performs no I/O, time, entropy, authority lookup, or effects.

**Rationale:** Handler names or matching signatures cannot prove equivalent behavior.

### Decision: Conflicts are durable artifacts, not partial mutations

**Choice:** A merge operation first creates a deterministic conflict or result plan. The shell publishes result roots and then a new world commit only after every output is durable.

Any unresolved conflict leaves branch heads unchanged.

**Rationale:** Partial root publication must not look like a completed semantic merge.

### Decision: Merge commits preserve all declared parents

**Choice:** A successful result commit names every merged source head in canonical declared order. Its detached merge receipt names the base, profile, handler identities, conflicts resolved, and output roots.

**Rationale:** Parent identity preserves causality. Detached receipts preserve evidence without changing commit identity.

## Rollout

1. Add root diff reports for exact identities only.
2. Add identical-only and ancestor-replacement merge modes.
3. Pilot keyed durable-value merge on one bounded Molten fixture.
4. Add one pure application-handler fixture with explicit schema migration denial cases.
5. Add result commit publication and branch-head integration after conflict rails pass.

## Risks / Trade-offs

- Application handlers can contain semantic bugs. Receipts identify the handler but do not prove correctness.
- Large values can exhaust merge bounds. Return bounded diagnostics without partial output.
- Schema migration can be lossy. Require explicit product admission and preserve migration evidence.
- Refusing runtime-sensitive roots limits automatic merge. This prevents duplicated authority, tasks, and effects.
