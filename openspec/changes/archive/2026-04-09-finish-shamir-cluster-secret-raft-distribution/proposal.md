## Why

The archived `shamir-cluster-secret` change landed the cryptography pieces, CLI flags, and local storage helpers, but the cluster-init path still stops at leader-local persistence. In a multi-node cluster, followers do not receive their assigned share through a Raft-committed application entry, so the repo does not yet satisfy the `cluster-secret-lifecycle` requirement that share distribution be part of consensus.

## What Changes

- Replace leader-local trust initialization with a Raft-committed trust-init application request.
- Persist each node's assigned share when that committed request is applied on the local state machine.
- Keep share digests replicated through the same committed request so every node sees the same verification metadata.
- Add regression tests that prove multi-node trust init goes through the apply path instead of direct local storage.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cluster-secret-lifecycle`: Tighten cluster-init behavior so every initial member stores its own share only by applying a committed trust-init request.

## Impact

- `crates/aspen-raft-types/` - new trust-init application request payload.
- `crates/aspen-raft/` - cluster init flow, state-machine dispatch, redb storage plumbing, tests.
- `openspec/specs/cluster-secret-lifecycle/spec.md` - clearer requirement for committed trust-init distribution.
