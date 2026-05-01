## Context

The jobs/CI extraction manifest already identifies `aspen-ci-core` and `aspen-jobs-protocol` as reusable surfaces, but `aspen-jobs` remains a runtime integration crate. Its default dependency graph includes distributed coordination, concrete Iroh networking, Tokio runtime services, Redb/bincode storage, chrono wall-clock helpers, monitoring, worker pools, and optional VM/blob plugin paths. Downstream consumers that only need job model or scheduler/run-state contracts should not inherit that graph.

Existing tests in `aspen-job-primitives-test` exercise dependency tracking, scheduling, DLQ, saga, workflow, affinity, and deterministic replay through the runtime crate. This change starts with the low-risk portable core and keeps runtime behavior in place through compatibility re-exports.

## Goals / Non-Goals

**Goals:**

- Create an independently-checkable reusable jobs/CI core surface.
- Preserve existing `aspen-jobs` public paths for workspace consumers.
- Make the first slice small: pure types and deterministic helpers before moving storage/service orchestration.
- Add evidence that reusable defaults avoid app, handler, transport, worker, executor, Redb, process, VM, and Nix/SNIX dependencies.

**Non-Goals:**

- Rewriting `JobManager`, `SchedulerService`, worker pools, durable workflow execution, or runtime storage in the first slice.
- Changing wire compatibility for `aspen-jobs-protocol`.
- Publishing crates or declaring a repository split before license/publication policy is decided.
- Removing compatibility re-exports from `aspen-jobs` in this change.

## Decisions

### 1. New jobs core crate first

**Choice:** Add `aspen-jobs-core` for portable job model and deterministic policy helpers rather than expanding `aspen-ci-core` to own jobs concepts.

**Rationale:** Jobs contracts are shared by CI but not CI-specific. A jobs-named core keeps ownership clear and lets `aspen-ci-core` depend on it only when CI schemas need shared job/run descriptors.

**Alternative:** Put all reusable scheduler/run-state concepts into `aspen-ci-core`. Rejected because non-CI job users should not import CI package names or schema dependencies.

**Implementation:** Start with types currently in `aspen-jobs/src/job.rs`, `types.rs`, and verified pure helpers where they can be extracted without storage/runtime imports. The runtime crate re-exports the moved surface.

### 2. Runtime shell remains the compatibility owner

**Choice:** Keep managers, scheduler service, worker traits/pools, Redb-backed storage, monitoring, tracing, VM/blob workers, durable executor, saga executor, and Iroh integration in `aspen-jobs` for now.

**Rationale:** These modules depend on runtime services and have broad consumers. Moving them before the pure contract is proven would create high churn and weak verification.

**Alternative:** Split all scheduler and manager code immediately. Rejected as too large for a first independently-verifiable slice.

### 3. Evidence before readiness raise

**Choice:** Do not mark jobs/CI as extraction-ready until policy, downstream fixture, negative fixture, dependency-boundary report, and compatibility checks are all saved.

**Rationale:** Prior extraction work relies on durable evidence, not subjective assessment.

## Risks / Trade-offs

**Duplicate model definitions during migration** → Prefer moving definitions and re-exporting from `aspen-jobs`; if temporary duplication is needed, add explicit conversion/golden tests.

**Serde/chrono/std compatibility drift** → Preserve existing serialization shape and add roundtrip/golden checks for moved types before changing internals.

**Runtime dependency leak into core** → Add negative fixture and checker policy for root app, handlers, Iroh/IRPC, Tokio runtime services, Redb, worker/executor crates, process/Nix/VM/SNIX adapters.

**Over-scoping into service orchestration** → First slice stops at pure contracts and deterministic helpers; service extraction becomes a later task only after fixtures pass.
