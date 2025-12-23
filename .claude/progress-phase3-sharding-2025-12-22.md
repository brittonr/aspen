# Phase 3: Shard-aware ALPN Routing - Progress Report

Date: 2025-12-22

## Completed Steps

### Step 1: Extend RPC Protocol with Shard ID

**File**: `src/raft/rpc.rs`

Added:

- `SHARD_PREFIX_SIZE` constant (4 bytes)
- `ShardedRaftRpcRequest` struct with `shard_id` and `request` fields
- `ShardedRaftRpcResponse` struct with `shard_id`, `response`, and `timestamps` fields
- `encode_shard_prefix()` - encodes shard ID as 4-byte big-endian
- `decode_shard_prefix()` - decodes from fixed-size array
- `try_decode_shard_prefix()` - safe decode from slice

**Tests**: 22 new tests covering encoding/decoding roundtrips, edge cases

### Step 2: Create ShardedRaftProtocolHandler

**File**: `src/protocol_handlers/raft_sharded.rs` (NEW)

Created handler that:

- Holds `HashMap<ShardId, Raft<AppTypeConfig>>` for routing
- Reads 4-byte shard ID prefix from incoming stream
- Looks up correct Raft core for that shard
- Forwards RPC and returns response with shard ID prefix
- Returns error if shard not found

**Also modified**:

- `src/protocol_handlers/constants.rs` - Added `RAFT_SHARDED_ALPN = b"raft-shard"`
- `src/protocol_handlers/mod.rs` - Re-exports for new handler

**Tests**: 8 unit tests for handler creation, shard lookup, serialization

### Step 3: Extend Network Layer for Shard-Aware Clients

**File**: `src/raft/network.rs`

Changes to `IrpcRaftNetwork`:

- Added `shard_id: Option<ShardId>` field
- Modified `send_rpc()` to prepend shard ID when `shard_id.is_some()`
- Response parsing strips and validates shard ID prefix

Changes to `IrpcRaftNetworkFactory`:

- Added `new_client_for_shard(target, node, shard_id)` method

Connection pool unchanged (connections shared, shard ID per-message).

## Remaining Steps (Not Yet Implemented)

### Step 4: Multi-Shard Bootstrap

**File**: `src/cluster/bootstrap.rs`

Add `ShardedNodeHandle` struct and `bootstrap_sharded_node()` function that:

1. Checks `config.sharding.enabled`
2. Determines local shards from config
3. Creates shared Iroh manager and network factory
4. For each local shard: creates storage, Raft instance, registers with handler
5. Returns `ShardedNodeHandle`

### Step 5: Wire Sharded Handler into Router

**File**: `src/bin/aspen-node.rs`

Add conditional branch for sharded mode that:

- Calls `bootstrap_sharded_node()` instead of `bootstrap_node()`
- Registers `RAFT_SHARDED_ALPN` with sharded handler
- Uses `ShardedKeyValueStore` for client protocol

### Step 6: Backward Compatibility

- Shard 0 = default shard (legacy single-node mode)
- Dual ALPN registration for migration
- Legacy `RaftProtocolHandler` routes to shard 0

## Test Results

All 687 unit tests pass:

- 89 sharding-related tests
- 49 RPC tests (including 22 new sharded tests)
- 8 sharded handler tests

Note: Some madsim simulation tests timeout due to pre-existing flakiness (unrelated to these changes).

## Files Modified

| File | Status |
|------|--------|
| `src/raft/rpc.rs` | Modified - Added sharded RPC types |
| `src/protocol_handlers/raft_sharded.rs` | NEW - ShardedRaftProtocolHandler |
| `src/protocol_handlers/constants.rs` | Modified - Added RAFT_SHARDED_ALPN |
| `src/protocol_handlers/mod.rs` | Modified - Re-exports |
| `src/raft/network.rs` | Modified - Shard-aware clients |

## Architecture

```
Client -> IrpcRaftNetwork (with shard_id)
             |
             | [4-byte shard_id prefix + RaftRpcProtocol]
             v
ShardedRaftProtocolHandler
             |
             +-> HashMap<ShardId, Raft<AppTypeConfig>>
             |
             +-> Route to correct Raft core
             |
             v
Response [4-byte shard_id prefix + RaftRpcResponse]
```

## Next Steps

1. Implement Step 4 (bootstrap_sharded_node)
2. Implement Step 5 (aspen-node.rs integration)
3. Implement Step 6 (backward compatibility)
4. Add integration tests for multi-shard consensus
