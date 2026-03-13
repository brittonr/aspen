## Context

The deploy system has a coordinator (runs on Raft leader) that dispatches `NodeUpgrade` RPCs to cluster nodes. Each node's handler accepts the RPC immediately and spawns a background `NodeUpgradeExecutor` task. The coordinator then polls `GetHealth` to track progress.

The feedback gap: when the executor fails before restarting (e.g., binary validation), the node never goes down, `GetHealth` returns healthy, and the coordinator concludes success. The executor logs the error but never writes a failure status to KV.

## Goals / Non-Goals

**Goals:**

- Coordinator detects pre-restart executor failures within one health poll interval
- Binary validation path configurable (default `bin/aspen-node`, overridable per deploy)
- Single-node VM test proves happy path with configurable binary
- Multi-node VM test proves sad path with graceful failure handling

**Non-Goals:**

- Fixing NixOS ExecStart to use profile path (separate change — the profile switch mechanism)
- Making the deploy system a general-purpose deployment tool
- Changing the async RPC-then-poll architecture (the pattern is fine, just needs the feedback channel)

## Decisions

### 1. Executor writes Failed status to KV on error

The spawned task in `handle_node_upgrade` already has access to the KV store. On `executor.execute()` returning `Err`, write `NodeDeployStatus::Failed(reason)` to the node's deploy status key (`_sys:deploy:node:{node_id}`).

Alternative considered: returning the result synchronously from the RPC. Rejected — the drain+swap+restart cycle can take minutes, blocking the RPC stream.

Alternative considered: a callback channel from executor to coordinator. Rejected — over-engineering. KV is already the shared state medium between coordinator and executors.

### 2. Coordinator checks KV node status during health polling

Add a KV read to `poll_node_health` that checks `_sys:deploy:node:{node_id}` for `Failed` status. This runs alongside the existing `GetHealth` RPC. If KV says Failed, the coordinator stops waiting and marks the node failed immediately.

The KV check uses stale reads (works on any node). The status key is written by the executor on the same node, so it's available locally even during network partitions.

### 3. `expected_binary` field in NodeUpgradeConfig

Add `pub expected_binary: Option<String>` to `NodeUpgradeConfig`. `upgrade_nix()` uses this instead of hardcoded `"bin/aspen-node"`. When `None`, skip the binary existence check entirely (just verify the store path exists via `nix-store --realise`).

The value flows from the deploy CI config → `NodeUpgrade` RPC → handler → executor config.

### 4. Wire protocol change: optional field on NodeUpgrade

`ClientRpcRequest::NodeUpgrade` gains `expected_binary: Option<String>`. Postcard handles `Option<T>` as a tagged enum — adding an `Option` field is backwards-compatible as long as it's appended. Old nodes that don't know about the field will deserialize it as `None` (the default), preserving the existing `bin/aspen-node` behavior.

### 5. Two VM test strategies

**Single-node (happy path):** CI config passes `expected_binary = "bin/cowsay"` in the deploy job. The executor validates `bin/cowsay` exists in the cowsay store path, passes. Profile switch + restart runs the original aspen-node (NixOS ExecStart is static). Health check passes. Deploy completes successfully.

**Multi-node (sad path):** CI config uses default (no `expected_binary`). Executor checks for `bin/aspen-node` in cowsay store path, fails. Executor writes `Failed` to KV. Coordinator detects via KV poll, marks deploy failed. Test asserts: deploy stage status is "failed", error contains expected message, cluster still healthy, KV data intact, leadership transfer still occurred.

## Risks / Trade-offs

**[Risk] KV write from executor races with coordinator read** → The coordinator polls on a 5s interval. The executor writes Failed within milliseconds of failure. Even with replication lag, the status will be visible within one poll cycle. Stale reads are acceptable for failure detection (false-negative delay is bounded by poll interval).

**[Risk] Adding Optional field to RPC enum changes wire format** → Postcard's encoding of `Option<String>` adds a tag byte. This is safe for new fields appended to struct-like enum variants. Existing nodes without the field will see `None`. Verified by postcard's documented backwards-compatibility guarantees for optional trailing fields.

**[Trade-off] NixOS ExecStart remains static** → After profile switch, systemd still runs the original binary. This means the happy-path test doesn't actually test binary replacement. Acceptable: the test validates coordination mechanics. Real binary replacement is tested by the self-build dogfood test.
