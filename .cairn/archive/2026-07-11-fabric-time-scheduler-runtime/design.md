## Context

System extensions need a portable event source for protocol timeouts and background work. Live runtimes naturally expose OS clocks, async timers, and randomness, while deterministic simulation needs a virtual clock, a controlled runnable queue, and reproducible choices. The extension protocol core must not know which shell it is using.

## Decisions

### 1. Time domains are explicit and non-interchangeable

**Choice:** Canonical values distinguish wall-clock observations, monotonic instants and durations, logical event positions, and simulation virtual time. Conversion is available only through declared adapter operations carrying uncertainty and non-claims.

**Rationale:** Comparing values from different domains silently creates incorrect deadline and lease logic.

### 2. Timers are generation-scoped commands and events

**Choice:** Extensions schedule one-shot or periodic timers through opaque ids bound to the active service generation. Firing, cancellation, lateness, coalescing, skipped periods, overload, and terminal state are explicit events.

**Rationale:** Direct sleeps cannot be cancelled or replayed consistently and may awaken stale service instances.

### 3. Scheduler choices are controlled inputs

**Choice:** The deterministic profile owns runnable selection and records canonical choice positions. The live profile obeys the same callback concurrency and cancellation contract but does not claim deterministic OS scheduling.

**Rationale:** Protocol logic can be replayed while preserving honest differences between deterministic simulation and production scheduling.

### 4. Entropy is an admitted replayable port

**Choice:** Extensions request bounded bytes or bounded choices from an entropy stream. Deterministic streams derive from explicit run and stream seeds; production streams use an admitted cryptographic source where required. Evidence stores stream identity and positions, not secret bytes.

**Rationale:** Randomized elections, backoff, identifiers, and sampling must be controllable in simulation without weakening production entropy.

### 5. Lease decisions require an explicit assumption profile

**Choice:** Lease helpers consume monotonic time, observed uncertainty, owner generation or fencing token, and a consistency profile. A timer firing only means the local deadline boundary was observed; it does not prove that remote actors agree or that an old holder cannot act.

**Rationale:** Distributed leases are protocol claims, not clock-port guarantees.

### 6. Time evidence is aggregate by default

**Choice:** Emit evidence for profile admission, deterministic run setup, material clock anomalies, scheduler replay, timer leaks, and selected semantic deadlines. Do not require one receipt per clock read or timer poll.

**Rationale:** Time is a hot-path dependency.

## Functional core / imperative shell split

- Pure core: time-domain validation, duration arithmetic with checked bounds, timer transitions, coalescing decisions, deadline classification, scheduler queue and choice transitions, entropy position accounting, lease precondition checks, and evidence payloads.
- Shell: read live clocks, arm runtime timers, obtain production entropy, advance virtual time, wake callbacks, persist replay traces, and enforce cancellation.

## Risks / Trade-offs

- A deterministic scheduler can create false liveness confidence. Keep scheduling assumptions and explored-choice coverage explicit.
- Wall-clock corrections can break naive deadlines. Require monotonic durations for local deadlines and model wall-clock anomalies separately.
- Recording entropy values can leak secrets. Record only non-secret deterministic seeds in test artifacts and refs or positions in production evidence.
