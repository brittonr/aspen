# Design: Octet burn-down, ambient clock and structural-scan recursion

## Context

`ambient_clock` flags `Instant::now`, `SystemTime::now`, and chrono `now` outside test context. Molten's time
authority is the fabric-time port: `TimerClockAdapter` has live (`LiveClockAdapter`) and deterministic
(`VirtualClockAdapter`) implementations. Two of the nine sites are inside the live adapter itself, where the host
clock is read by design.

## Decisions

### Decision: Reasoned allows only at the live clock capability

**Choice:** Two item-level allows: on `LiveClockAdapter::new` (monotonic origin) and on `observe_wall` (wall clock).
Each reason names the adapter as the documented live clock capability. The adapter's third site (`await_ticks`) is
repaired through its own `now_ticks`.

**Rationale:** The director's allow policy admits `ambient_clock` only at this boundary. Everything that needs time
elsewhere goes through the port.

### Decision: One deadline helper for both clocks

**Choice:** `TickDeadline { deadline_ticks }` is built from `clock.now_ticks() + timeout_ticks` (checked). It reports
`remaining_ticks` (saturating) and `is_expired` (`remaining == 0`). `SupervisionDeadline` pairs it with a
`LiveClockAdapter` admitted under a named `molten.fabric-time.process-supervision` live profile. Live monotonic ticks
are nanoseconds. Timeouts over the profile's one-hour maximum are denied. The largest caller bound is Sightglass
`MAX_SIGHTGLASS_RUN_SECONDS` = 3600, which is admitted.

**Rationale:** Supervision loops keep their exact comparison semantics. Tests drive the same helper on the virtual
clock.

### Decision: Explicit frame stack for the structural scan

**Choice:** Each frame holds the not-yet-visited `(path segment, child)` pairs of one open container. A node is counted
and bounded before its children are listed. Children beyond the remaining node budget are not materialized, because
reaching any of them would first exceed `max_nodes`.

**Rationale:** This reproduces the recursive preorder, first match, and first error. A throwaway differential test
compared 1,680,000 scans of generated values across limits, predicates, and scopes against the recursive
implementation and found them identical. Its source is kept in the evidence.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: the diff, the Octet, clippy, test, and fixture
evidence, and the differential scan evidence.

## Failure behavior

A clock failure returns the port error through the existing `MoltenError` conversion, or for Sightglass a
`PerformanceDenial`. A supervision loop that cannot read the clock stops with an error instead of waiting unbounded.

## Risks / Trade-offs

- Each supervision loop admits a small live profile per call. That is one BLAKE3 hash of a fixed descriptor.
