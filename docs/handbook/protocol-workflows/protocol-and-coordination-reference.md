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
| Service manifest | `service_id`, `services`, `control_group_ref`, queue/semaphore capacities, rate limit, barrier parties, policy/resource refs | Coordination part p001 |
| Request | `service`, `operation`, `key`, `client_session`, `operation_id_ref`, `read_consistency_mode`, optional payload, authority/resource/policy refs | Coordination part p001 |
| Receipt | Decision, request/state refs, optional Raft/token refs, transition kind, before/after/preserved state refs, output refs, optional prior receipt, assertions, diagnostics | Parts p002 and p013 |
| Status assertion | Service, key, read-consistency mode, state ref, receipt ref | Part p002; observation-only |
| `report.preserves` from `apply` | `coordination-apply-report-v1`: decision, manifest, final state, receipt/assertion/evidence refs | CLI `run_apply` |
| `report.preserves` from fixture | `coordination-fixture-report-v1`: fixture decision and evidence links | Part p011 |
| `evidence-N.preserves` | Indexed canonical values, starting at index zero | CLI `write_indexed_values` |

Supported service names are `lock`, `queue`, `semaphore`, `rate-limit`, `election`, `barrier`, and `registry`. Service membership does not imply every operation is valid for every service. Request read mode defaults to `READ_CONSISTENCY_LINEARIZABLE` in the CLI; the implementation also names `READ_CONSISTENCY_LOCAL_STALE`. Inspect the declared mode rather than interpreting every successful read as equivalent.

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
