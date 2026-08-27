## Context

The world protocol requires immutable roots and cheap branch references. It does not require one physical store. Molten content chunks, Redb indexes, and ChaosControl snapshots have different sharing and restore behavior.

A benchmark rail must compare those mechanisms without erasing profile boundaries or treating timing as semantic correctness.

## Decisions

### Decision: Measure exact resource facts before duration

**Choice:** Required metrics include logical input bytes, physical bytes written, new objects, reused objects, copied pages, mapped pages, traversed references, compared keys, emitted conflicts, transferred bytes, retained objects, and planned deletions.

Duration and peak memory are secondary observations bound to a hardware and host cohort.

**Rationale:** Exact operation and byte facts are more stable than wall-clock measurements and reveal structural sharing directly.

### Decision: Use typed named benchmark profiles

**Choice:** Nickel profiles declare dataset identity, operation sequence, warm or cold state, branch count, mutation set, bounds, repetitions, hardware class, and acceptance thresholds.

Every nontrivial numeric value has a named typed field. Rust revalidates the projection before execution.

**Rationale:** Reviewers must see why each limit exists. Hidden constants make results irreproducible.

### Decision: Keep logical and opaque cohorts separate

**Choice:** Logical profiles measure typed state, chunks, task roots, and semantic diff. Opaque profiles measure exact snapshot and VM Cohort sharing facts.

Receipts cannot compare the cohorts as semantically equivalent or rank one as universally better.

**Rationale:** The profiles preserve different state and have different compatibility boundaries.

### Decision: Separate preparation from measured operations

**Choice:** Dataset construction, prepopulation, cache warming, and compaction occur in explicit preparation phases with their own observations. The measured phase begins from an identified state.

A receipt that cannot prove its preparation state is invalid.

**Rationale:** Hidden warm caches or preexisting objects can create false sharing and latency results.

### Decision: Benchmark retention planning, not deletion authority

**Choice:** The rail measures reachability traversal, pin evaluation, protected-object reuse, candidate classification, and deletion-plan size. It does not execute deletion as part of the benchmark verdict.

Correctness fixtures independently verify that reachable, pinned, witnessed, quarantined, or policy-retained objects never enter a deletion plan.

**Rationale:** Fast unsafe cleanup is not useful evidence.

### Decision: Make extraction a reviewed decision

**Choice:** A pure classifier consumes accepted receipts and typed policy. It returns:

- `retain-current` when requirements pass,
- `optimize-in-place` when one owned adapter is the bounded blocker,
- `evaluate-shared-component` when repeated product-neutral limits fail across at least two credible consumers.

The result does not create a repository or approve a dependency.

**Rationale:** A generic branchable-state project needs measured demand and more than one consumer, not architectural preference alone.

## Rollout

1. Define profiles, metrics, receipts, and deterministic synthetic datasets.
2. Add exact operation counters for root branching, mutation, diff, and content reuse.
3. Add replication, retention, and GC-plan measurements.
4. Add exact ChaosControl and later VM Cohort observations.
5. Run one downstream-shaped fixture under cold and declared warm states.
6. Review the extraction decision only after stable repeated receipts exist.

## Risks / Trade-offs

- Instrumentation can change performance. Exact count metrics reduce dependence on timing.
- Host caches can leak between runs. Profiles must identify preparation and reject unknown state.
- Finite datasets do not prove asymptotic behavior. Receipts must retain this non-claim.
- Compression can obscure copied-byte costs. Record logical and physical bytes separately.
- A passing benchmark can regress later. Source, profile, and dataset identities make freshness explicit.
