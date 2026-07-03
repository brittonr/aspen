# Design: Raft membership admission

## Scope

This change specifies and prepares the stronger admission path for future Raft/control-plane membership joins. It does not make ordinary peering a membership operation and does not require normal actor traffic to use Raft.

## Membership records

`raft-membership-change-request-v1` names the group, target node/peer, requested role, prior configuration ref, proposed configuration ref, peer/session refs, authority refs, policy refs, resource refs, source-gate/provenance refs, snapshot/readiness refs, and operator evidence refs.

`raft-membership-preflight-receipt-v1` records pass/deny checks before any log mutation. `raft-membership-commit-receipt-v1` records the eventual committed membership change only after the consensus layer admits and commits it.

## Admission checks

Membership preflight is pure over canonical evidence values and checks:

- peer session is admitted for the membership scope,
- operation authority grants membership change authority,
- policy and resource refs match the requested group and role,
- source-gate and provenance evidence are current for the node/control-plane artifact set,
- target node supports the required state machine, schema, transport, replay, and snapshot profile,
- snapshot/log catch-up evidence is present when required,
- quorum preservation and configuration transition rules pass Trellis/Raft predicates.

## CLI/runbook

Operators get a dry-run preflight command first. Mutating membership changes remain explicit control-plane operations and require committed receipts. Diagnostics must make clear that `peer connected` is insufficient.

## Functional core

The core computes preflight decisions and membership safety diagnostics from in-memory evidence. The shell reads ledgers, invokes node-control or Raft mutation paths, writes receipts, and renders summaries.

## Non-goals

- No automatic voter addition during peer connect.
- No lease/read shortcut for membership safety.
- No global peer directory.
- No replacement for existing Raft log, snapshot, read-index, and commit receipts.
