## Context

`ForgeNode::set_federation_settings` and `ForgeResourceResolver::get_resource_state` already exist and are tested. The handler stub just needs to call them. The sync handler already connects, handshakes, and lists resources — it just stops there instead of querying per-resource state.

The `ForgeServiceExecutor` has `self.forge_node`, `self.federation_cluster_identity`, and `self.iroh_endpoint` — all needed to construct `FederatedId` and to make outbound connections.

## Goals / Non-Goals

**Goals:**

- `federation federate <repo_id>` persists settings so the repo shows up in remote `ListResources`
- `federation sync --peer` returns ref heads per resource after querying `GetResourceState`
- Synced ref state written to local KV for persistence
- NixOS test proves the full flow end-to-end

**Non-Goals:**

- Syncing actual git objects (blobs, trees, commits) — only ref heads
- Multi-seeder quorum orchestration (single peer sync only)
- Automatic/periodic sync (manual CLI trigger only)

## Decisions

**1. `handle_federate_repository` constructs FederatedId from cluster identity + repo BLAKE3 hash**

The repo ID in Forge is a BLAKE3 hash. `FederatedId::from_blake3(cluster_pubkey, repo_hash)` gives a globally unique ID. The handler needs `self.federation_cluster_identity` to get the origin key — pass it through from the executor.

**2. Store synced refs in KV under `_fed:sync:<origin_short>:<local_id_short>:refs/<name>`**

This is a read-only mirror of remote state. The `_fed:sync:` prefix keeps it separate from local Forge data. Short hex prefixes (first 16 chars) keep keys readable.

**3. Extend `SyncPeerResourceInfo` with ref head data**

Add `ref_heads: HashMap<String, String>` (ref name → hex hash) to the response so the CLI can display what was synced.

## Risks / Trade-offs

- [Risk] `handle_federate_repository` needs cluster identity but currently only gets `forge_node` → Pass identity through executor, or read it from the executor's stored field.
- [Risk] NixOS test creates repo with no commits, so refs may be empty → Test pushes a commit via `git push` to create `refs/heads/main` before federating.
