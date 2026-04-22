## Context

`aspen-raft-types` sits below the runtime transport crates, but it still encoded concrete `iroh::EndpointAddr` values in `RaftMemberInfo` and the trust reconfiguration payloads. That made a foundational metadata crate depend on shell-only transport types even though the persisted data Aspen actually needs is endpoint id plus transport addresses.

Review also found two hardening gaps in the first implementation pass:

1. `RaftMemberInfo::default()` used a placeholder endpoint id string that runtime parsers rejected.
2. Several shell paths skipped invalid address conversions without logging, which hid why peer seeding or leader forwarding was unavailable.

## Goals / Non-Goals

**Goals**

- Persist transport-neutral membership and trust address data in `aspen-raft-types`.
- Convert to iroh only at runtime shell boundaries.
- Keep default/test-helper membership metadata parseable by runtime helpers.
- Make invalid runtime conversion paths visible through warnings or deterministic errors.
- Save durable validation transcripts and verification metadata for review.

**Non-Goals**

- Eliminating iroh from runtime crates.
- Reworking transport discovery or trust quorum algorithms.
- Broadening this change into another large no-std foundation sweep.

## Decisions

### 1. Store `NodeAddress` in raft metadata and trust payloads

**Choice:** `RaftMemberInfo`, `TrustInitializePayload.members`, and `TrustReconfigurationPayload.members` use `aspen_core::NodeAddress`.

**Rationale:** `NodeAddress` preserves the endpoint id plus transport-address set without binding the metadata crate to runtime iroh types.

**Alternative considered:** keep `iroh::EndpointAddr` in the leaf crate. Rejected because it keeps `aspen-raft-types` coupled to runtime transport and directly contradicts the deferred seam this change now picks up.

### 2. Convert to iroh only in runtime shells

**Choice:** `aspen-raft` exposes small conversion helpers that turn `NodeAddress` / `RaftMemberInfo` back into `iroh::EndpointAddr` for connection-opening code.

**Rationale:** this keeps the functional data model transport-neutral while letting shell code report context-rich conversion failures.

**Alternative considered:** teach `aspen-raft-types` how to convert directly. Rejected because it would reintroduce the iroh dependency into the leaf crate.

### 3. Keep `RaftMemberInfo::default()` parseable

**Choice:** use the historical zero-seed iroh public key string `3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29` as the default endpoint id constant in `crates/aspen-raft-types/src/member.rs`.

**Rationale:** existing runtime helpers still parse endpoint ids even for test-helper/default nodes. A parseable deterministic id preserves old behavior without reintroducing runtime transport types.

**Alternative considered:** leave a human-readable placeholder string. Rejected because it causes runtime helper paths to silently drop the default member.

### 4. Warn on invalid runtime conversion and fail deterministically

**Choice:** the following runtime paths log explicit warnings when stored metadata is invalid:

- bootstrap peer seeding in `crates/aspen-cluster/src/bootstrap/node/mod.rs`
- leader forwarding in `crates/aspen-raft/src/node/kv_store.rs`
- batched/direct write forwarding in `crates/aspen-raft/src/write_batcher/{flush.rs,direct_write.rs}`
- current-leader and membership refresh lookups in `crates/aspen-raft/src/node/{mod.rs,membership_refresh.rs}`
- relay allowlist extraction in `crates/aspen-cluster/src/relay_server.rs`
- blob-topology extraction in `src/bin/aspen_node/main.rs`

Forwarding paths then return the existing deterministic retryable failure (`NotLeader` / unavailable target) instead of silently pretending no target existed.

**Rationale:** invalid persisted metadata is exceptional. Operators and tests need a visible reason when a peer/leader cannot be reached.

### 5. Save exact seam-validation artifacts

**Choice:** save the exact rails below under `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`, plus `verification.md`, `evidence/implementation-diff.txt`, and `evidence/openspec-preflight.txt`:

- `cargo test -p aspen-raft-types`
- `cargo check -p aspen-raft`
- `cargo check -p aspen-cluster`
- `cargo check -p aspen-raft --features trust`
- `cargo tree -p aspen-raft-types -e normal --depth 1`
- `cargo test -p aspen-raft test_raft_member_info_default_endpoint_id_is_parseable --lib`
- `cargo test -p aspen-raft test_member_endpoint_addr_rejects_invalid_node_address --lib`
- `cargo test -p aspen-raft test_raft_member_info_construction --lib`
- `cargo test -p aspen-raft --features trust test_collect_old_shares_accepts_valid_remote_share_from_stored_address --lib`
- `cargo test -p aspen-cluster --features relay-server test_extract_endpoint_ids_skips_invalid_member_endpoint_id --lib`

Reserve `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/warning-behavior.md` for explicit warning-behavior coverage before archive.

## Risks / Trade-offs

- **More logging on corrupt metadata** → acceptable because these warnings indicate real state mismatch or bad persisted data.
- **Targeted verification only** → acceptable for this change while it stays scoped to the metadata seam; keep broader consumer coverage as an explicit follow-up task before archive.
