# Design: Octet burn-down, bounded channels and fixed-width public integers

## Context

`unbounded_channel` flags `tokio::sync::mpsc::unbounded_channel`. `usize_in_public_api` flags public
functions and inherent methods whose signature contains `usize`/`isize`, including inside a generic or reference
argument.

## Decisions

### Decision: Inbox capacity is the admitted scheduler queue depth

**Choice:** `time_for` derives the inbox capacity from `AdmittedTimeProfile::max_scheduler_queue_depth` with
`usize::try_from` and denies zero.

**Rationale:** The inbox carries the time port's scheduled events, and the profile's admitted queue depth is the
existing limit for that queue.

### Decision: Backpressure for timers, typed denial for control observations

**Choice:** Each timer task awaits `send`. `ChannelReplicaControlPort::publish` uses `try_send` and maps `Full` to a
denial error and `Closed` to the existing unavailable error.

**Rationale:** Timer events must not be lost; parking the timer task delays them. The control port is synchronous and
must not block the service loop, so it reports the full queue to the caller instead of dropping the observation.

### Decision: No self-deadlock

**Choice:** Timer sends run in their own spawned tasks and hold no lock. The service owns the receiver and drains it in
`next_event`; it never sends on its own inbox. Control publishing never blocks.

**Rationale:** A blocking send could only deadlock if the sender were the receiver's task or held a lock the receiver
needs. Neither is true here. A test fills a capacity-1 inbox with two armed timers and proves both events are delivered
once the receiver drains.

### Decision: Checked fixed-width integers at the public boundary

**Choice:** Public signatures use `u64`/`u32`. `crate::bounded::{usize_from_u64,u64_from_usize}` convert at the
boundary and return a labelled `MoltenError` when a value does not fit. Crate-internal callers that already hold a
`usize` bound use a crate-private `parse_within`. Arity constants become `u64`.

**Rationale:** This follows the handoff's recommendation (change the public signature and carry the conversion at the
boundary), and uses no `as` truncation.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: the diff and the Octet, clippy, test, and fixture
evidence.

## Failure behavior

- A full control queue returns an error and publishes nothing.
- A zero admitted queue depth denies node setup.
- A non-fitting integer returns a labelled error at the conversion point.

## Risks / Trade-offs

- A slow-draining live node now delays timer events at the admitted queue depth, where before its queue grew
  without limit.
