# Reviewing consensus scope and evidence

Mode: Review checklist

Apply this checklist to a change, operational report, or proposed demonstration that combines protocol sessions, coordination, and Raft membership. Record an answer and an artifact/source pointer for each relevant question; mark an inapplicable item with its reason. This is a source-checked review procedure, not a report that the checks ran. Return to the [Handbook](../README.md) for related workflows.

The governing [architecture](../../architecture.md#consensus-layer-trellis-backed-raft-for-control-plane) restricts Raft to explicit control-plane state. Ordinary actor messages, ordinary choreography traffic, blob transfer, gossip fanout, and local-only assertions are outside that scope. OpenRaft is not selected or adapted. The [consistency companion](../../technical/membership/consistency-and-fastpath-nonclaims.md) explains why a consistency-shaped artifact is not proof of a live service.

## Scope and ownership acceptance

- [ ] **What state needs agreement?** Name the exact registry pointer, configuration, receipt index, replay ledger, membership configuration, or explicit lock/lease. Evidence: the request/command and its owning implementation, not the word “distributed” in a design description.
- [ ] **Are ordinary messages still outside consensus?** Trace one representative actor or choreography message. Evidence: separate message and control-plane command paths. Reject a justification that every workflow step must enter Raft merely because one step changes control-plane state.
- [ ] **Does choreography stop at orchestration?** Identify the endpoint transition and the consensus operation it invokes, if any. Evidence: the protocol manifest and adapter boundary. Do not accept choreography as a replacement implementation of Raft internals.
- [ ] **Is the execution class explicit?** Label each item pure law, local fixture/model, filesystem CLI effect, or live adapter observation. Evidence: constructors and actual recorded execution. A pure helper returning a commit-shaped record is not automatically a live commit.

## Protocol evidence acceptance

- [ ] **Does the install match the manifest?** Retain the canonical manifest, install receipt, decision, and projected endpoint values. Evidence: the [install/replay path](../../../src/protocol/parts/session/p003/body.rs), including denial diagnostics when applicable.
- [ ] **Are operations tied to their real prior states?** Check protocol, session, role, prior-state reference, message reference, and next-state reference together. Evidence: canonical values sufficient for the [operation gate](../../../src/protocol/parts/session/p004/body.rs), not only their displayed names.
- [ ] **Is completion a complete supplied trace?** Require initial states and unique passing successors to terminal states. Evidence: no missing-state or ambiguous-successor diagnostics from the relevant gate. Do not infer live acknowledgement or effect completion from a terminal local state alone.
- [ ] **Are facade descriptors kept non-effectful?** Evidence: a caller that still admits actual effects through its normal gates. The [typed facade document](../../choreography-typed-facade.md) explicitly excludes authority, policy, resource, provenance, and transport trust grants.

## Coordination evidence acceptance

- [ ] **Was the actual runtime constructor reviewed?** The inspected [coordination constructor](../../../src/coordination/parts/mod/p002/body.rs) uses a fixture manifest and `new_control_registry_model_runtime`. Evidence: the selected call path, not a manifest field that merely names a control group. Do not approve a live-cluster claim using this CLI model alone.
- [ ] **Is replay scope stated accurately?** Exact duplicates emit a new `duplicate-replay` receipt linking prior output, preserve state, and create no new commit/assertions. Evidence: `prior_receipt_ref`, transition kind, preserved-state reference, and [duplicate handler](../../../src/coordination/parts/mod/p013/body.rs). Changed requests under the same operation identity must be reviewed as conflicts.
- [ ] **Are report aggregation rules understood?** Evidence: the [CLI apply loop](../../../src/cli/workflow/coordination/ops.rs) versus the [fixture report constructor](../../../src/coordination/parts/mod/p011/body.rs). Apply records a nonpassing aggregate when an operation denies; the mixed-case fixture constructs a passing fixture report despite intentional negative cases. Neither is a claim of atomic rollback of the entire batch.
- [ ] **Is read consistency explicit?** Retain the requested mode and corresponding evidence. An observation-only status assertion must not silently become permission to act or a stronger freshness claim.

## Membership evidence acceptance

- [ ] **Is connectivity separated from membership admission?** Evidence: `peer_session_scope` is `raft-membership`, and the requested role is one of `voter`, `non-voter`, or `learner`. A connected peer alone is insufficient; the [connectivity companion](../../technical/membership/connectivity-versus-membership.md) explains the broader distinction.
- [ ] **Who resolves the ten evidence categories?** Preflight checks nonempty authority, policy, resource, source-gate, provenance, compatibility, snapshot, replay, quorum-safety, and operator-evidence collections. Evidence: their legitimate producers and the caller's validation path. The helper's presence checks are not independent verification of their content or currentness.
- [ ] **Is preflight bound to the intended commit externally?** In [membership source](../../../src/raft/membership.rs), `commit_raft_membership` checks the supplied preflight's decision, then constructs a receipt from the supplied request. It does not independently compare a stored preflight subject to that request. Record this limitation instead of describing the helper as a complete admission transaction.

## Worked review outcome

Suppose a proposal presents a passing local coordination fixture report and a passing membership commit receipt as proof that a connected node became a live voter. The checklist rejects that conclusion without discarding the artifacts. The fixture demonstrates a model path; the membership helper evaluates supplied evidence categories and emits records. Neither artifact alone establishes a live configuration change, durability, current authority, or quorum participation.

The review should request the actual admitted membership operation, its subject/configuration binding, effect owner, and observed live outcome. Preserve the narrow source observations: receipt serialization includes subject fields and quorum-safety refs but not all evidence collections, and commit does not independently bind its preflight to the request. These are source-review limits, not reproduced vulnerabilities or executed failure results.

Final acceptance should state exactly what was executed, what was only inspected, and which claim remains unsupported. Canonical Preserves+BLAKE3 identifies evidence; receipt existence, signatures, and fixture success do not create authority or establish exactly-once effects or production readiness.

## Sources

- [Handbook](../README.md)
- [Architecture and consensus scope](../../architecture.md#consensus-layer-trellis-backed-raft-for-control-plane)
- [Typed choreography facade](../../choreography-typed-facade.md)
- [Consistency and fast-path non-claims](../../technical/membership/consistency-and-fastpath-nonclaims.md)
- [Connectivity versus membership](../../technical/membership/connectivity-versus-membership.md)
- [Coordination model constructor](../../../src/coordination/parts/mod/p002/body.rs)
- [Duplicate evidence implementation](../../../src/coordination/parts/mod/p013/body.rs)
- [Membership preflight, commit, and tests](../../../src/raft/membership.rs)
