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

Deterministic profiles order by admitted FIFO or priority/FIFO policy and record every choice. Recorded-choice profiles require an eligible recorded choice during replay. A fairness claim exists only when the selected profile includes a finite fairness bound. Otherwise, canonical evidence preserves `does-not-prove-fairness`. Bounded fairness promotes the longest-waiting overdue runnable before ordinary priority ordering.

## Scheduler capacity activation

The capacity core derives one plan from the admitted profile and active generation.
The plan binds runnable, queue, and concurrency slots with checked conversion and arithmetic.
It adds no second capacity configuration.

The live shell reserves storage for runnable states and queue keys before activation.
An allocation error denies activation without a smaller scheduler or hidden fallback.
The owned reservations use the concrete Rust slot types for this implementation cohort.
They do not replace canonical Preserves state or make Rust layout authoritative.

Steady-state accounting rejects a stale generation, wrong profile, wrong plan, released capacity, underflow, overflow, or physical exhaustion.
The shell does not widen either reservation after activation.
Existing scheduler ordering, fairness, replay, cleanup, and overload decisions remain authoritative.

Capacity observations bind the plan, profile, generation, usage, high-water values, exhaustion count, and release state.
These observations do not prove global latency, fairness, liveness, host memory stability, or whole-runtime zero allocation.

## Entropy

Entropy streams bind profile, stream id, purpose, capability ref, generation, mode, replay class, and byte position. Requests are bounded byte draws or bounded choices.

Deterministic simulation derives a chunk-invariant byte stream from an explicit seed and absolute stream position. Admission also requires a BLAKE3 deterministic-input ref, which evidence records so replay identity is bound without exposing the raw seed. The algorithm is deterministic test input, not cryptography. Production mode forbids deterministic seeds and requires caller-supplied cryptographic bytes. The current Unix shell reads `/dev/urandom` and fails closed if that source cannot be opened or filled; it has no deterministic or weak fallback.

Entropy evidence contains purpose, capability-bound stream metadata, generation, mode, replay class, and consumed positions. It never includes generated bytes or secret seed material. Production replay therefore requires an independently authorized secret input rather than embedding secrets in receipts.

## Deadlines, retries, and local leases

Deadlines use monotonic, logical, or virtual domains. Wall-clock deadlines are denied. Decisions are pending, expired, or indeterminate within explicit uncertainty. Retry plans have finite attempts, a maximum delay, and optional entropy-backed bounded jitter.

Exponential retry delay equals `min(base_delay_ticks * 2^attempt, maximum_delay_ticks)` over mathematical integers. The core rejects exhausted attempts before this calculation. A type-width guard and checked multiplication prevent loss of high bits. Large admitted attempts saturate without a loop or narrowing conversion. Fixed delays and non-overflow exponential outputs retain their values.

The core validates supplied jitter before checked addition and the final delay cap. Jitter-addition overflow and deadline overflow remain errors. The core reads no clock or entropy source. A retry plan does not prove that an application operation is safe to repeat.

The current coordination-delivery profile selects fixed backoff with no jitter. F12 demonstrates a generic exponential calculation defect, not a failure through that fixed profile. The arithmetic bound is not a measured performance claim.

Lease decisions are local observations unless the request names a reviewed fenced consistency profile and a fresh fencing token. Without both, an exclusive action is denied. Even a passing local decision does not prove global time, synchronized clocks, remote deadline agreement, partition absence, or distributed lease exclusivity.

## System-extension integration

`ExtensionTimeContext` snapshots the admitted system-extension service id, active lifecycle generation, timer/runnable/concurrency/byte envelopes, and capability refs from `SystemExtensionHost`. It rejects cross-service timer/runnable identities, stale generations, exhausted timer or scheduler resources, over-budget entropy requests, and unadmitted entropy capabilities. Retired generations are cancelled through the same pure cleanup laws.

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

### Retry fixture and replay boundary

The saturation case samples tick 35 from the run's shared virtual clock.
Base 2 at attempt 63 produces delay 128 and deadline 163.
The timer delivers at 163, and the report finishes at 164.
The ordinary jittered case retains deadline 52.
The selected delivery profile remains fixed backoff without jitter.

Private replay recomputes canonical delay and deadline observations from explicit inputs.
Each retry observation records its time domain in the existing details field.
Equal ticks from virtual and monotonic clocks do not identify the same observation.
Matching fixed and saturated observations pass.
Wrapped, domainless, cross-domain, changed, reordered, or missing observations produce visible divergence without history mutation.
The fixture records `wrapped-replay-rejected` only after it rejects the wrapped counterexample.

The current profile and outer observation schema remain unchanged. Only the corrected arithmetic rule is admitted.
Domain data changes the retry event references. Replay rejects older domainless retry observations instead of silently upgrading them.
Replay does not authenticate history, verify timer effects, or authorize another timer.
The public `show` command checks report shape, not arbitrary event-history replay.
See the F12 change's `evidence/fixture-replay/validation.md` for commands, artifacts, source identities, and limits.

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
