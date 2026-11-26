# Protocol and coordination reference

Mode: Reference

Use this reference when classifying an artifact or locating its producer. It covers the inspected protocol-session core, coordination CLI/model, and Raft membership receipt helpers. It is source-checked reference material, not an execution transcript or a complete wire-schema specification. Return to the [Handbook](../README.md); consult the [architecture](../../architecture.md#choreography-layer-trellis-backed-protocol-shape) for governing ownership rules.

## Ownership lookup

| Question | Owner to inspect | What this does not establish |
| --- | --- | --- |
| Is a global workflow projectable? | `install_protocol_manifest` in protocol session part p001 | Transport delivery or application authorization |
| Is this endpoint operation legal? | `evaluate_protocol_endpoint_transition` in session part p007 | Execution of a shell effect |
| Does supplied lifecycle evidence replay and terminate? | Session gate in parts p003–p005 | Completion of arbitrary external work |
| How is a coordination batch materialized? | `run_apply` in CLI coordination `ops.rs` | A persisted remote cluster transaction |
| Does a repeated operation preserve model state? | Coordination part p013 | Exactly-once effects across crashes |
| What evidence categories are required for membership preflight? | `src/raft/membership.rs` | Resolution or current validity of every supplied reference |

The [typed facade](../../choreography-typed-facade.md) returns descriptors and non-effect evidence. It is not a second transport or admission system. The [transport session companion](../../technical/transport/session-admission-and-transitions.md) covers a different session domain; a transport handle must not be substituted for a protocol state.

## Protocol records and useful fields

Field names in this table describe Rust view fields unless quoted as wire labels. The canonical Preserves record, not the Rust layout, owns identity.

| Record | Fields to retain | Producer or consumer |
| --- | --- | --- |
| `protocol-manifest-v1` | Protocol identifier; role, label, and payload registries; global script/choice; policy, capability, and resource references | Manifest constructor/parser, session p001 |
| `protocol-endpoint-v1` | `protocol_ref`, `role`, `role_id`, `local_state` | Projection and endpoint conversion, session p006 |
| Session state | `state_ref`, `protocol_ref`, `session_id`, `role`, `sequence`, `endpoint`, `local_state`, `seen_message_refs` | Startup and operation advancement |
| `protocol-message-v1` | `protocol_ref`, `session_id`, `from_role`, `to_role`, `label`, `payload_tag`, `body_or_ref`, `sequence`, `evidence_refs` | Send constructor and receive matching |
| `protocol-operation-receipt-v1` | `operation`, `decision`, `prior_state_ref`, optional `message_ref`, optional `next_state_ref`, admission/carrier references, diagnostics | Send/receive/branch/offer helpers |
| `protocol-session-gate-receipt-v1` | Install/protocol references, initial-state references, operation/message references, terminal-state references, decision | Lifecycle gate and receipt parser |

An operation receipt encodes wire fields `prior-state`, `message`, `next-state`, `authority`, `resource`, and `carrier`. Optional message and state references are explicit optional values. Absence of `next_state_ref` is meaningful on denial; do not fill it using the last state you happen to possess.

The session source bounds item collections at 1,024 and script/replay steps at 256. These are implementation bounds, not throughput promises. A terminal local state requires both an empty action list and the `End` terminal variant.

## Coordination input and output map

| Surface | Important inputs or output fields | Owner |
| --- | --- | --- |
| Service manifest | `service_id`, `services`, `control_group_ref`, queue/semaphore capacities, rate-limit quota, barrier parties, policy/resource refs | Coordination part p001 |
| Request | `service`, `operation`, `key`, `client_session`, `operation_id_ref`, `read_consistency_mode`, optional payload, authority/resource/policy refs | Coordination part p001 |
| Receipt | Decision, request/state refs, optional Raft/token refs, transition kind, before/after/preserved state refs, output refs, optional prior receipt, the refs of the status assertions actually emitted, diagnostics | Parts p002 and p013 |
| Status assertion (`coordination-status-assertion-v3`) | Service, key, read-consistency mode, state ref, control-plane receipt ref (the Raft commit receipt of a mutation or the registry read receipt of a read) | Part p002; observation-only |
| Commit intent (`coordination-commit-intent-v1`) | Request ref, before-state ref, after-state ref without token-claimed entries, token claims (service, key, owner, token, lease epoch, lease expiry) | Part p016; the control-registry `set-coordination-state` target |
| `report.preserves` from `apply` | `coordination-apply-report-v1`: decision, manifest, final state, receipt/assertion/evidence refs | CLI `run_apply` |
| `report.preserves` from fixture | `coordination-fixture-report-v1`: fixture decision and evidence links | Part p011 |
| `evidence-N.preserves` | Indexed canonical values, starting at index zero | CLI `write_indexed_values` |

Supported service names are `lock`, `queue`, `semaphore`, `rate-limit`, `election`, `barrier`, and `registry`. Service membership does not imply every operation is valid for every service. Request read mode defaults to `READ_CONSISTENCY_LINEARIZABLE` in the CLI; the implementation also names `READ_CONSISTENCY_LOCAL_STALE`. Inspect the declared mode rather than interpreting every successful read as equivalent.

Despite its wire name, `rate-limit` is a per-key acquisition quota for the lifetime of the coordination state: the manifest's `rate-limit` value is the number of acquisitions each key may make, and nothing resets the count. The quota does not consult the coordination clock described below, so there is no time window; an exhausted key is denied with `rate-limit quota exhausted; it does not reset` until a new coordination state or key is provisioned.

## Fencing tokens, leases, and resignation

A lock acquire or election grants a fencing token bound to the control-plane commit receipt of that transition. The control registry commits a token-free commit intent; the committed `LockState` or `ElectionState` then carries exactly the `token_ref` of the issued `FencingToken`, and the receipt's `token` field, the status assertion's fact, and the state snapshot's retention refs name that same ref. The status assertion binds the control-plane receipt rather than the coordination receipt, so the receipt's `dataspace` refs list the canonical refs of the assertions actually emitted.

Every lock and leader term carries a lease bounded on the coordination clock. Coordination time comes only from the runtime's shell-owned `CoordinationClock` port, a `fabric_time` `TimerClockAdapter` bound to one time basis. Requests never carry time, and other mutations never advance it. The live constructors `new_coordination_runtime` and `new_coordination_runtime_with_control_group` accept only a sealed live clock from `CoordinationClock::live()`, whose basis must match its provenance, whatever crate features are enabled. That clock is the node monotonic clock under the admitted live lease-clock profile, with nanosecond ticks since its own monotonic origin. Its basis ref binds the profile, the wall instant the origin was anchored at, and a process-local open sequence. Live shells (`molten test coordination apply`, the job worker schedule) use it. Deterministic clocks are accepted only by `new_coordination_simulation_runtime`, which in turn refuses live clocks. `run-fixture` uses a simulation runtime frozen at tick zero, so its leases never expire. Adjustable fixture clocks (`CoordinationClock::fixture`, `fixture_on_basis`, `new`) exist only under `cfg(test)` or the dev-only `test-fixtures` feature, and a downstream crate that enables that feature still cannot hand them to a live constructor. The runtime binds the clock's basis into the state when it is built. It samples the clock once for each admitted mutation, denies a sample from another basis (`does not match committed time basis`) or a regressing sample (`regresses below committed time`), and records the committed sample as `(time_basis_ref, ticks)`. That pair appears in the state snapshot's `time` record, the commit intent's `time` record, and `CoordinationApplyResult::time_sample`. Replaying the committed requests with their recorded samples therefore reproduces the committed state.

An acquire or elect payload may request `<lease ttl>` (elections also accept `<candidate name <lease ttl>>`); without one the lease lasts `DEFAULT_COORDINATION_LEASE_TICKS` (30 seconds on the live basis). `ttl` must be between 1 and `MAX_COORDINATION_LEASE_TICKS` (one hour on the live basis), and the lease expires `ttl` ticks after the clock sample of its own commit. A stream of unrelated committed mutations does not age a lease. An idle crashed holder's lease becomes expirable as soon as the clock passes its expiry.

No committed sample passes `MAX_COORDINATION_TIME_TICKS` (`u64::MAX - MAX_COORDINATION_LEASE_TICKS`), so every granted lease expiry is representable. A sample past the ceiling is denied with `coordination time exhausted` with the state preserved, and all tick and expiry arithmetic is checked.

A clock read failure is not a decision. `apply_coordination_request` returns the port's classified failure (for example `capability unavailable`) before it records any receipt or operation id. A retry with the same operation id therefore samples the clock again rather than replaying a denial.

The runtime's clock is private. A clock on a different basis is installed only through `recreate_coordination_live_clock(runtime, authority_refs)`. This is a node capability, not a request operation: the node shell calls it when it recreates its live clock, for example after reopening the monotonic clock source, so ticks restart near zero on a new basis. No coordination request, payload, or CLI command reaches it. The caller supplies no clock: the runtime opens a fresh sealed live clock itself and checks that the clock's basis ref matches its provenance. It refuses a runtime whose current clock is not live, so simulation runtimes keep their deterministic clock. The transition commits a `coordination-clock-transition-v1` record (retiring basis and its latest observation, new basis and start sample, before and after state refs) through the control registry at `coordination:clock` / `time-basis`.

- **Lease re-anchoring:** every check (live provenance, live runtime, new basis, authority evidence) runs first. Then one reading of the node monotonic source is converted into both the retiring and the new basis, since every live clock counts nanoseconds from an origin on that source. Each lock or leader lease's new expiry is `new_now + (old_expiry - old_now)`, saturating at zero. No time is lost between two separate samples, so an already-expired lease stays expired, and no number of back-to-back recreations, whatever authority ref accompanies them, extends a lease.
- **Restart gap:** if the retiring clock no longer reads on the committed basis, as after a restart, `old_now` falls back to the last committed sample. This conservative gap can keep a lease live for the unobserved time between that sample and the restart, but it never expires one early.
- **Fencing state:** holders, tokens, token refs and the next fencing token are unchanged.
- **Replay:** replaying the committed transitions together with the committed requests and their samples reproduces the state.
- **Denials:** a transition to the same basis, or one without authority evidence, is denied. Ordinary requests that sample a clock on another basis are denied with `does not match committed time basis`.

- `expire` on `lock` or `election` takes no payload. It removes the holder once the clock sample reaches the lease expiry. An unexpired lease is denied with `is unexpired`, and a presented time is denied with `takes no payload`; either way the state is preserved.
- `resign` on `election` with `<token n>` steps the leader down; a session other than the leader is denied with `resignation by non-leader`, and a stale token is denied with `stale fencing token`.

After expiry or resignation, the next acquire or election receives a strictly larger fencing token, so the previous holder's token stays stale.

## Worked classification: a duplicate acquire

In the checked-in coordination fixture, the first two requests acquire the same resource with the same operation identity. For an exact matching request, the duplicate path emits a distinct `duplicate-replay` receipt whose `prior_receipt_ref` identifies the earlier receipt. It preserves current state, supplies prior output references, and supplies neither a new Raft commit nor new assertions. A changed request under that identity instead yields `conflicting-duplicate-deny`.

The distinction explains why the README phrase “replay the prior receipt” should not be read as byte-identical receipt reuse. Compare semantic references, not output-file indices. Also preserve the runtime scope: each CLI `apply` constructs a fresh control-registry model with an initially empty applied-operation map.

## Membership receipt boundary

`RaftMembershipRequest` names group, target peer/session, requested role, and configuration. Preflight requires `raft-membership` scope, a supported role (`voter`, `non-voter`, or `learner`), and ten nonempty evidence collections. The receipt serializes subject fields, quorum-safety refs, diagnostics, and `peer-connectivity-is-not-membership` boundary text. It does not serialize every input evidence collection. The commit helper checks the supplied preflight decision; it is not itself a live membership change or a full request-to-preflight binding verifier.

## Sources

- [Handbook](../README.md)
- [Typed facade boundary](../../choreography-typed-facade.md)
- [Connectivity and membership theory](../../technical/membership/connectivity-versus-membership.md)
- [Protocol types and bounds](../../../src/protocol/parts/session/p000/body.rs)
- [Protocol operation wire fields](../../../src/protocol/parts/session/p007/body.rs)
- [Coordination types](../../../src/coordination/parts/mod/p000/body.rs)
- [Coordination receipt and runtime](../../../src/coordination/parts/mod/p002/body.rs)
- [CLI output ownership](../../../src/cli/workflow/coordination/ops.rs)
- [Membership helpers](../../../src/raft/membership.rs)
