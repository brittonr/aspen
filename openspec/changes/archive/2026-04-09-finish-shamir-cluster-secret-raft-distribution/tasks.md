## 1. Raft Request Plumbing

- [x] 1.1 Add a `TrustInitialize` application request payload to `crates/aspen-raft-types` for epoch, per-node share bytes, and epoch digests.
- [x] 1.2 Teach both redb and in-memory state-machine dispatch paths to handle `TrustInitialize` requests.
- [x] 1.3 Track the local node ID inside `SharedRedbStorage` so trust-init apply can select only the local share.

## 2. Cluster Init Integration

- [x] 2.1 Change `RaftNode::initialize_trust()` to submit the committed `TrustInitialize` request through `raft.client_write()` instead of writing leader-local storage directly.
- [x] 2.2 Update the cluster-init control-plane flow to await the async trust initialization hook and surface trust-init write failures cleanly.

## 3. Regression Tests

- [x] 3.1 Add a state-machine test proving `TrustInitialize` stores only the local share while persisting shared digests.
- [x] 3.2 Add a node-level regression test covering multi-node trust init request construction and threshold validation.
- [x] 3.3 Run targeted trust-enabled tests for `aspen-raft` and `aspen-trust`.
- [x] 3.4 Add a multi-node cluster-init integration test that proves follower share persistence after the committed trust-init request is applied.
