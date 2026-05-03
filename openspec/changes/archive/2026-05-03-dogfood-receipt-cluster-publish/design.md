## Context

A local full dogfood run intentionally stops and removes its ephemeral cluster after acceptance. That means in-cluster receipt publication cannot replace local durable receipts yet. It can, however, prove that the live Aspen cluster can host its own evidence during operator acceptance and gives long-running/non-ephemeral deployments an Aspen-native receipt path.

## Goals / Non-Goals

**Goals**
- Publish only validated canonical receipt JSON.
- Use an explicit deterministic KV namespace for dogfood evidence.
- Reuse existing receipt validation and rendering for cluster-loaded values.
- Avoid credential material; tickets remain only in local state and are never written into the receipt value.

**Non-Goals**
- No new cluster RPC variants.
- No schema migration for receipt JSON.
- No guarantee that ephemeral `full` receipts remain queryable after `stop` removes the cluster.

## Decisions

### 1. Use core KV before adding a dogfood evidence service

**Choice:** Store receipt JSON at `dogfood/receipts/<run-id>.json` via existing `WriteKey`/`ReadKey` RPCs.

**Rationale:** KV is already Raft-backed, Iroh-accessible, and sufficient for a first operator-visible self-hosting proof.

**Alternative:** Add dedicated receipt RPCs. Rejected for this slice because the evidence path can be proven without expanding public protocol surface.

### 2. Publish remains explicit

**Choice:** Add an explicit `receipts publish` command for a running cluster; do not make local receipt persistence depend on cluster publication.

**Rationale:** Failure receipts are most valuable when the cluster may be unhealthy. Local receipt persistence must remain best-effort independent of live cluster reachability.

### 3. Cluster show validates before rendering

**Choice:** `receipts cluster-show` parses and validates the KV value as `aspen.dogfood.run-receipt.v1` before printing text or JSON.

**Rationale:** Operators should not accidentally treat arbitrary KV bytes as trusted receipt evidence.

## Risks / Trade-offs

- **Ephemeral full cleanup:** A normal local `full` stops and removes the cluster, so in-cluster query is a live acceptance affordance, not a replacement archive.
- **KV namespace collisions:** The deterministic key prefix is dogfood-specific and tests cover key construction.
