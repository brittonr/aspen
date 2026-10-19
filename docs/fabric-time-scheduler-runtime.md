# Fabric time, scheduler, and entropy runtime

Molten exposes time, timers, runnable scheduling, entropy, deadlines, retries, and local lease decisions as admitted fabric services. Their transition laws live in the pure `molten-core` functional core. Wall-clock reads, monotonic clock reads, sleeps, operating-system entropy, artifact writes, and CLI output remain in thin adapter shells.

This boundary does not make clocks, randomness, scheduler behavior, process state, service names, transport identity, or Rust layout canonical authority. Canonical profiles and evidence are Preserves values identified with BLAKE3.

## Time domains

The core keeps four non-interchangeable value types:

- `WallClockObservation` is an untrusted UTC observation with uncertainty and a monotonic observation sequence.
- `MonotonicInstant` supports process-local elapsed-time decisions.
- `LogicalEventTime` represents explicit event ordering.
- `VirtualInstant` is controlled by deterministic simulation.

`CheckedDuration` carries its profile and domain. Comparison, addition, subtraction, deadline construction, and timer admission reject profile or domain mismatch and checked-arithmetic overflow/underflow. Crossing domains or profiles requires an `ExplicitTimeConversion` naming source and target domains, a signed offset, uncertainty, and conversion-evidence ref. No implicit wall/monotonic/logical/virtual conversion exists.

Wall-clock movement is never a lease or authority oracle by itself. The live shell records wall and monotonic observations separately. `classify_wall_clock_observation` makes backward jumps, large forward jumps, and excessive uncertainty explicit.

## Exact profiles and ports

A `molten.fabric-time.profile.v1` descriptor selects either `live` or `deterministic-simulation`, supported domains, duration and uncertainty bounds, timer/runnable/entropy limits, scheduler concurrency and queue limits, optional fairness bounds, evidence granularity, and the complete non-claim set. Admission rejects malformed identity, missing domains/non-claims, duplicate values, zero or inconsistent limits, and hard-cap violations. There is no profile fallback.

Each admitted profile produces exact `v1` descriptors for:

- `molten.fabric.time.clock`
- `molten.fabric.time.timer`
- `molten.fabric.scheduler.runnable`
- `molten.fabric.entropy.stream`

The existing fabric registry enforces exact version, class, operation, schema, authority, resource, determinism, replay, and implementation-profile matches. A sandboxed plugin cannot acquire time or scheduling authority from operation shape or plugin metadata. A reviewed system extension may receive those authorities only with full system-tier evidence. Applications consume admitted services through application-service authority rather than inheriting adapter authority.

## Timers

Timer identity binds service id, extension generation, and sequence. Admission also binds profile, domain, deadline, one-shot or periodic behavior, stable ordering key, coalescing, lateness, overload policy, and resource charge.

Pure transitions provide:

- one-shot and drift-free periodic deadlines;
- stable simultaneous-expiry ordering;
- cancellation and terminal duplicate-fire denial;
- periodic catch-up, latest coalescing, or missed-period skipping;
- bounded lateness decisions;
- explicit retain/reject, backpressure, or recorded drop under overload;
- stale-generation discard; and
- generation cleanup for shutdown, restart, upgrade, and rollback.

The live and virtual adapters supply observations to the same timer core. They do not implement separate timer semantics.

## Runnable scheduler

The scheduler tracks generation-bound runnable ids through ready, running, blocked, completed, and cancelled states. Wake, choose, yield, block, complete, cancellation, and generation cleanup are explicit transitions. Queue depth, total runnables, and concurrent selections are bounded.

Every transition into ready - a new wake, a blocked wake, and yield - passes the same ready-capacity admission. A new occurrence charges the active-runnable envelope and the ready queue; an existing occurrence already holds its active slot, so only the ready queue applies. A denied transition returns `RejectedOverload` or `Backpressure` from the admitted overload policy, publishes no accepted wake or yield action, and leaves phases, counts, priorities, and sequence positions unchanged. Wake resumes only from `blocked`; ready, running, completed, and cancelled occurrences remain duplicates. Successful entry into ready takes a checked fresh enqueue position. Wake applies the supplied priority and resets the wait counter; Yield retains its occurrence's priority. Stale-generation commands are discarded without effects. Rejected Yield retains Running ownership: retry requires an explicit later command, not an adapter fallback. The canonical command and action schemas remain `v1`.

Deterministic profiles order by admitted FIFO or priority/FIFO policy and record every choice. Recorded-choice profiles require an eligible recorded choice during replay. A fairness claim exists only when the selected profile includes a finite fairness bound. Otherwise, canonical evidence preserves `does-not-prove-fairness`. Bounded fairness promotes the longest-waiting overdue runnable before ordinary priority ordering.

### Owning shell and receiving effects

[`SchedulerRuntime<P>`](../src/fabric_time/shell/owned.rs) owns one admitted profile,
`ExtensionTimeContext`, scheduler state, activated capacity, and an application-owned
[`SchedulerWakePort`](../src/fabric_time/ports.rs). Extensions construct their context
with `ExtensionTimeContext::from_host`; the executable application fixture and
conformance consumer use `for_application` with their explicitly admitted profile.
That constructor does not establish system-extension authority or a manifest envelope.

The owner previews the authoritative core command/selection decision without cloning
the scheduler snapshot, derives active/Ready deltas from the actual source and target
phases, stages checked capacity use, and prepares fallible delivery before publication.
With exclusive mutable ownership it publishes scheduler, capacity, and Ready indices
together, then delivers only `Woken`/`Yielded`. Denial, stale discard, invalid phase,
overflow, replay divergence, and preparation failure preserve the complete scheduler
and capacity state. No pending delivery is added by those paths. A delivery error after
publication is `SchedulerRuntimeError::CommittedEffectUncertain`: state stays committed,
the result is not overload/denial, and the owner never retries automatically.

Delivery receives `SchedulerWakeCommit`, borrowing the owner's actual published
scheduler and capacity fields plus the stable occurrence index. It is not a copied
expected/future state and adds no allocation to valid owning transitions. Concrete
adapters retain their narrow thread-unpark and simulated-queue mechanisms.

[`ThreadSchedulerWakeAdapter`](../src/fabric_time/parts/adapters/p003/body.rs) prepares
the exact generation-qualified target and unparks it after commit.
`SimulationSchedulerWakeAdapter` independently buffers bounded retained-occurrence
indices. `drain_next` returns a `u64`; consumers check native index conversion,
then read committed state/accounting. It never invokes the thread adapter or
synthesizes an admitted scheduler action.
A full undrained delivery queue is a fallible preparation error, not scheduler overload.
The executable fixture and scheduler conformance consumer use the owner rather than
manually charging capacity or routing pure transitions.

The existing adapter regression paths now target this owning API, paired live and
simulation receivers, phase-derived accounting, both overload policies, stale/service/
profile fences, replay and arithmetic errors, preparation failure, terminal cleanup,
and release. The receiving regressions capture the actual borrowed fields at the
delivery boundary before the concrete effect, not later behind the command mutex.
A deliberate wrong-order control delivers the real Running/Blocked owner fields
through each concrete adapter; the same receiver predicate must reject those fields
and accept normal committed publication. Per-command preparation/delivery sequences
and FIFO receiver checkpoints account for denied work before shutdown, without an
absence-by-timeout inference or hiding coalesced wake permits. Accepted live progress
is observed before any checkpoint control wake. The [recorded two-profile CLI
smoke](../evidence/cairn-source-remediation-2026-10-15/attempt-19-shared-scheduler-source-acceptance/actual-fabric-time-cli-readback.json)
exercises the owning runtime and its concrete conformance consumers. Molten fabric-time
maintainers own these regressions and fixture; neither establishes global liveness
or measured performance.


## Scheduler capacity activation

The capacity core derives one plan from the admitted profile and active generation.
The plan binds runnable, queue, and concurrency slots with checked conversion and arithmetic.
It adds no second capacity configuration.

The shell fallibly preallocates the actual `SchedulerState::runnables` storage and
Ready-index queue before activation; these are used by admitted transitions, not
inert placeholder reservations. An allocation error denies activation without a
smaller scheduler or hidden fallback. Shared core preview/apply decisions preserve
the existing FIFO, priority/fairness, replay, cleanup, and overload laws without an
independent adapter admission implementation. The owner transfers a new occurrence's
owned command key into backing storage; snapshot/canonical evidence allocation is
separate from transition publication.

Neither backing widens after activation. The admitted logical slot counts remain
binding even if an allocator reserves more bytes than requested. Active usage counts
Ready, Running and Blocked phases; queue usage counts only Ready phases. Selection
releases one Ready slot, Complete/Cancel release the actual active/Ready charges,
and generation cleanup sums the affected phases. Release requires quiescence and
fences further transitions while retaining terminal identity history.

Completed and Cancelled rows are deliberately retained in runnable backing and are
never silently purged or reused. Thus a scheduler can have zero active usage yet
no physical space for a new identity after its retained history fills the plan.
New identities then receive the existing Reject/Backpressure action without state
mutation; an existing Blocked occurrence can still resume if Ready capacity exists.
This finite-history/full-backing limit is not terminal reclamation or an implementation
of the separate `bound-terminal-scheduler-retention` change. Rust layout and stable
in-process indices do not replace canonical Preserves identity or occurrence fencing.

Capacity use rejects stale generations, wrong profiles/plans, released capacity,
underflow and overflow. Scheduler overload leaves usage, high-water and exhaustion
observations unchanged; it is not a speculative capacity charge. Explicit standalone
capacity exhaustion observations remain distinct from denied scheduler transitions.

Capacity observations bind the plan, profile, generation, usage, high-water values, exhaustion count, and release state.
These observations do not prove global latency, fairness, liveness, host memory stability, or whole-runtime zero allocation.

## Entropy

Entropy streams bind profile, stream id, purpose, capability ref, generation, mode, replay class, and byte position. Requests are bounded byte draws or bounded choices.

Deterministic simulation derives a chunk-invariant byte stream from an explicit seed and absolute stream position. Admission also requires a BLAKE3 deterministic-input ref, which evidence records so replay identity is bound without exposing the raw seed. The algorithm is deterministic test input, not cryptography. Production mode forbids deterministic seeds and requires caller-supplied cryptographic bytes. The current Unix shell reads `/dev/urandom` and fails closed if that source cannot be opened or filled; it has no deterministic or weak fallback.

Entropy evidence contains purpose, capability-bound stream metadata, generation, mode, replay class, and consumed positions. It never includes generated bytes or secret seed material. Production replay therefore requires an independently authorized secret input rather than embedding secrets in receipts.

## Deadlines, retries, and local leases

Deadlines use monotonic, logical, or virtual domains; wall-clock deadlines are denied. Decisions are pending, expired, or indeterminate within explicit uncertainty. Retry plans have finite attempts, checked fixed/exponential delay, a maximum delay, and optional entropy-backed bounded jitter. A retry plan does not prove that retrying an application operation is safe.

Exponential retry delay uses a width guard and checked multiplication before the maximum-delay cap.
Every admitted large attempt saturates without an attempt-sized loop or a narrowing error.
Jitter addition and target-instant addition retain their existing checked-overflow errors.
The current coordination-delivery profile uses fixed delay and does not expose the exponential defect.
Molten fabric-time maintainers own these arithmetic boundary tests.

The correction keeps the profile and event schemas at `v1` because their shapes do not change.
Fixed-delay and non-overflow exponential plans retain their values.
Reproduction still requires the exact implementation cohort.
Historical wrapped delays differ from corrected canonical events and their BLAKE3 references.
Do not rewrite historical receipts or treat cross-cohort output as an exact replay.
The fixture records both `retry-planned` target ticks and `retry-delay` ticks after successful admission.
Rejected plans append no retry observations. These fixture observations do not prove timer execution or safe application retries.

The correction is natively synced and archived with its
[executed acceptance and review](../.cairn/archive/1970-01-01-saturate-exponential-retry-arithmetic/evidence/2026-10-16-acceptance.json).
The record distinguishes the saturated public-API smoke, ordinary CLI fixture,
canonical strict source gate, feature cohorts, and bounded non-claims.

`replay_fixture_retry` recomputes the fixture observations from explicit time, policy, attempt, and jitter inputs.
It requires the exact canonical deadline and delay events in their original order.
Missing, extra, altered, or historical wrapped events produce a `retry replay diverged` diagnostic.
The function borrows the recorded events and never rewrites them.
Its fixture generation and subject remain fixed. It does not authenticate the historical inputs or replay a live timer.

Lease decisions are local observations unless the request names a reviewed fenced consistency profile and a fresh fencing token. Without both, an exclusive action is denied. Even a passing local decision does not prove global time, synchronized clocks, remote deadline agreement, partition absence, or distributed lease exclusivity.

## System-extension integration

`ExtensionTimeContext` snapshots the admitted system-extension service id, active lifecycle generation, timer/runnable/concurrency/byte envelopes, and capability refs from `system_extension::Dispatcher`. It rejects cross-service timer/runnable identities, stale generations, exhausted timer or scheduler resources, over-budget entropy requests, and unadmitted entropy capabilities. Retired generations are cancelled through the same pure cleanup laws.

Every timer, scheduler, entropy, deadline, lease, clock-anomaly, fault, and conformance event carries a non-zero generation and canonical profile ref. A receipt cannot reactivate stale work or substitute for current manifest, capability, resource, policy, provenance, or port admission.

## Deterministic simulation and faults

`VirtualClockAdapter` supports explicit virtual/logical advancement and synthetic wall observations. Fault inputs include backward/forward wall jumps, timer delay, recorded timer drop, scheduler saturation, and partition windows. Faults are visible inputs and evidence events; they never silently alter the pure transition relation.

A shared adapter conformance fixture runs live monotonic and virtual timer adapters through the same observable schedule/fire, stale-generation, and cancellation contract. The executable fixture additionally covers periodic coalescing, overload/drop evidence, deterministic scheduler replay, production and deterministic entropy, deadline/retry/lease decisions, wall-clock anomalies, and partition evidence.

Run it with:

```console
molten fabric-time run-fixture \
  --profile both \
  --out artifacts/fabric-time-fixture
```

`--profile` accepts `live`, `deterministic-simulation`, or `both`. The output directory contains canonical live and simulation profile artifacts, an aggregate run report, and bounded event artifacts under `evidence/`. Validate and render the secret-free operator summary with `molten fabric-time show artifacts/fabric-time-fixture/report.preserves`; the readback parser rejects event artifacts, malformed report shapes, unknown profiles, invalid refs, and wrong field types.

## Non-claims

Fabric-time evidence explicitly does not prove:

- global time;
- synchronized clocks;
- distributed lease exclusivity;
- fairness unless a bounded profile says so;
- liveness;
- safe retry;
- absence of partitions; or
- remote deadline agreement.

It also inherits the fabric-wide non-claims for consensus, ordering, persistence, transport delivery, Byzantine tolerance, protocol compatibility, extension semantic correctness, database correctness, and production readiness.
