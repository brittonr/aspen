## Context

Aspen nodes run as long-lived processes (typically under systemd on NixOS). The cluster already has graceful shutdown via `shutdown_signal()` + `node_mode.shutdown()`, a `Supervisor` with restart/backoff in `aspen-raft`, distributed coordination primitives (locks, leader election) in `aspen-coordination`, and the worker/job framework. What's missing is the orchestration that ties these into a quorum-safe rolling upgrade.

The CI pipeline already produces Nix store paths and blob artifacts. This phase makes the cluster consume those artifacts — an operator runs `cluster deploy <store-path>` and the cluster handles the rest.

## Goals / Non-Goals

**Goals:**

- A node can replace its own binary and restart without losing cluster membership
- Rolling upgrades maintain Raft quorum at all times
- Deployment state survives leader failover (stored in KV with CAS)
- Operator can rollback a failed or completed deployment
- CLI provides full deploy lifecycle: initiate, monitor, rollback

**Non-Goals:**

- CI pipeline integration (phase 2: `ci-deploy-loop`)
- Dogfood script changes (phase 2)
- NixOS VM integration test for the full build→deploy loop (phase 2)
- Blue-green or canary with traffic splitting (future)
- Cross-cluster federation deployment
- Wire format version negotiation between old and new binaries
- Windows/macOS support

## Decisions

### 1. Nix profile switch as primary upgrade mechanism

On systems with Nix, the node runs `nix-env --profile /nix/var/nix/profiles/aspen-node --set <store-path>` to atomically switch the binary, then restarts via systemd. Nix preserves previous generations for rollback.

**Alternative considered**: `execve` to replace the process in-place. Rejected — bypasses systemd supervision, loses journal correlation, complicates rollback. Systemd gives us restart tracking and watchdog for free.

For bare-binary (no Nix): download blob → staging path → SHA-256 validate → `--version` smoke test → atomic rename → preserve `.bak` → restart. This is the fallback.

### 2. Leader-coordinated rolling deployment

The Raft leader runs the coordinator. It picks one node at a time (followers first, leader last), sends `NodeUpgrade` RPCs, waits for health, then proceeds.

State lives in KV at `_sys:deploy:current` using CAS writes:

```
_sys:deploy:current       → { deploy_id, artifact, status, nodes: [{id, status}...] }
_sys:deploy:history:{ts}  → completed deployment records
```

**Alternative considered**: Peer-to-peer where each node watches for new artifacts and upgrades itself. Rejected — can't guarantee quorum safety if multiple nodes restart simultaneously.

### 3. Drain-before-upgrade

Before upgrading, the node:

1. Stops accepting new client RPC connections
2. Returns `NOT_LEADER` for write proposals (clients rotate to other nodes)
3. Continues serving Raft replication traffic
4. Waits up to `DRAIN_TIMEOUT_SECS` (30s) for in-flight ops
5. After drain, proceeds with binary swap and restart

Mirrors the existing `WorkerService::shutdown()` pattern.

### 4. Health-gated progression

After each node restarts, the coordinator checks three things:

1. Node appears in Raft membership
2. Node responds to `GetHealth` with healthy status
3. Node's Raft log gap < 100 entries (caught up)

All three must pass within `DEPLOY_HEALTH_TIMEOUT_SECS` (120s). Failure halts the deployment — no auto-rollback, operator decides next step.

### 5. Quorum safety invariant

Before upgrading node N, verify:

```
(healthy_voters - nodes_currently_upgrading - 1) >= ceil((total_voters + 1) / 2)
```

`max_concurrent` defaults to 1. Upper bound is `(N-1)/2`. For 3 nodes: max 1 at a time. For 5 nodes: up to 2.

### 6. Leader upgrades last

Followers upgrade first. The leader upgrades itself last. During leader upgrade:

- Leader writes `deploying` for its own node in `_sys:deploy:current`
- Leader drains and restarts
- Cluster elects new leader during the gap
- New leader reads `_sys:deploy:current`, sees all nodes upgraded, marks deployment `completed`

Brief unavailability is acceptable — single-node restart window.

### 7. Feature flag gating

Everything behind a `deploy` feature flag. Feature chain:

```
root deploy → aspen-deploy, aspen-cluster/upgrade, aspen-cluster-handler/deploy, aspen-rpc-handlers/deploy
```

CLI always compiles deploy commands (they just fail gracefully if the server doesn't have the feature).

## Risks / Trade-offs

**[Risk: Leader failover during deployment]** → Deployment state is in KV (linearizable). New leader reads `_sys:deploy:current` and resumes. CAS prevents duplicate upgrades.

**[Risk: Upgraded node can't rejoin cluster]** → Health check timeout catches this. Deployment halts. Old binary preserved (Nix generations / `.bak` file). Operator runs `cluster rollback`.

**[Risk: Wire format incompatibility between versions]** → Not solved here. Postcard discriminant stability is a known concern (napkin has 5+ incidents). Operators must test compatibility before deploying. Version negotiation is future work.

**[Risk: Nix store path not on target node]** → Target node fetches from binary cache substituter. If cache miss and no blob fallback, upgrade fails for that node and deployment halts.

**[Trade-off: One at a time is slow]** → Safety over speed. `max_concurrent` is configurable. Default of 1 is correct for 3-node clusters where any two simultaneous restarts lose quorum.

**[Trade-off: No auto-rollback]** → Halt-and-wait gives the operator information to decide. Auto-rollback could mask the real problem. Rollback is one CLI command away.

## Open Questions

1. Should `cluster deploy` accept a Nix flake reference (e.g., `.#aspen-node`) in addition to store paths and blob hashes? Leaning no — keep it concrete, let the caller resolve.
2. Should deploy write to the observability pipeline (traces/metrics)? Probably yes for production, but can defer to a follow-up.
3. Do we need `cluster deploy --dry-run` that shows which nodes would upgrade without doing it?
