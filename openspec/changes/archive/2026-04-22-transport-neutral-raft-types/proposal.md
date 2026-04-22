## Why

`extend-no-std-foundation-and-wire` explicitly deferred the `aspen-raft-types` transport seam, but the remaining runtime boundary still stores concrete iroh address types in the Raft metadata crate. That keeps a foundational type crate coupled to runtime transport details and makes the trust payload path carry shell-only types deeper than necessary.

The current implementation moves that seam to `NodeAddress`, but it needs its own OpenSpec change plus hardening for the review findings: `RaftMemberInfo::default()` must stay parseable for runtime helpers, invalid address conversions must stop failing silently, and the claimed cargo validation must be saved under durable evidence.

## What Changes

- Replace `iroh::EndpointAddr` in `crates/aspen-raft-types` membership and trust payload types with transport-neutral `aspen_core::NodeAddress` values.
- Keep all concrete iroh conversion at runtime shell boundaries in `aspen-raft`, `aspen-cluster`, and top-level node binaries.
- Preserve a parseable default endpoint id for `RaftMemberInfo::default()` so runtime membership watchers, relay access-control updates, and blob-topology extraction keep deterministic behavior in tests/helpers.
- Surface explicit warnings or deterministic forwarding failures when runtime code cannot convert stored membership metadata back into an iroh address.
- Save targeted cargo check/test transcripts and OpenSpec verification artifacts for this seam.

## Non-Goals

- Removing iroh from runtime crates such as `aspen-raft` or `aspen-cluster`.
- Changing Raft wire protocol semantics, gossip behavior, or trust quorum rules beyond the address representation seam.
- Reopening the already-completed `extend-no-std-foundation-and-wire` storage/traits/client-api/protocol work.

## Capabilities

### Modified Capabilities

- `transport`: Raft membership metadata and trust payloads now preserve transport-neutral addresses until runtime shells open network connections.
- `trust-reconfiguration`: trust membership payloads carry `NodeAddress` data and reject invalid runtime conversion paths explicitly.

## Impact

- **Files**: `crates/aspen-raft-types/`, `crates/aspen-raft/`, `crates/aspen-cluster/`, `src/bin/aspen_node/`, test helpers, and new OpenSpec evidence under `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/`.
- **APIs**: `RaftMemberInfo`, `TrustInitializePayload`, and `TrustReconfigurationPayload` become transport-neutral at the type level; runtime callers convert back to iroh as needed.
- **Dependencies**: `crates/aspen-raft-types` drops its direct `iroh` dependency.
- **Testing**: targeted `cargo check` and `cargo test` rails must be saved as durable evidence, including the new parseable-default regression test and trust-path coverage.

## Verification

Save the following exact rails under `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`:

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

Keep the root mapping in `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/verification.md`, save the reviewed diff under `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/implementation-diff.txt`, save this change's preflight transcript under `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/openspec-preflight.txt`, and reserve `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/warning-behavior.md` for explicit warning-behavior coverage before archive.
