# Identity Persistence

> Automatic persistence of node identity for stable cluster membership across restarts.

## Overview

Aspen nodes have two identities:

1. **Raft NodeId** (`u64`): Configured in TOML, identifies the node within Raft consensus
2. **Iroh EndpointId** (Ed25519 public key): Derived from a 32-byte secret key, used for P2P authentication

For nodes to maintain their identity across restarts, both must remain stable. The Raft NodeId comes from configuration, but the Iroh secret key must be persisted to disk.

## The Problem

Without secret key persistence:

```
Node starts → Generates random secret_key → EndpointId = X
Node restarts → Generates NEW random secret_key → EndpointId = Y

Other nodes see EndpointId Y as a completely different peer,
even though it has the same Raft NodeId.
```

This causes:

- Connection failures (peers try to reach old EndpointId)
- Split-brain scenarios in peer discovery
- Gossip routing issues

## The Solution

Aspen automatically persists the Iroh secret key to `{data_dir}/iroh_secret_key`:

```
Node starts → Checks for existing key file
  ├─ File exists → Load key → Same EndpointId
  └─ File missing → Generate key → Save to file
```

## Configuration

### Automatic (Recommended)

When `data_dir` is configured, secret key persistence is automatic:

```toml
# config.toml
node_id = 1
data_dir = "./data/node-1"

# Secret key will be saved to: ./data/node-1/iroh_secret_key
```

### Manual Override

For advanced use cases, you can explicitly provide a secret key:

```toml
# config.toml
node_id = 1
data_dir = "./data/node-1"

[iroh]
secret_key = "deadbeef...64hex..."  # 32 bytes as 64 hex characters
```

When an explicit key is provided, it takes priority over any file.

### Programmatic API

```rust
use aspen_cluster::IrohEndpointConfig;

// Automatic: key loaded/saved from path
let config = IrohEndpointConfig::new()
    .with_secret_key_path("/data/node-1/iroh_secret_key");

// Manual: explicit key (highest priority)
let config = IrohEndpointConfig::new()
    .with_secret_key(my_secret_key)
    .with_secret_key_path("/data/node-1/iroh_secret_key");  // Not used for loading
```

## File Format

The secret key file contains:

- 64 hexadecimal characters (32 bytes)
- Optional trailing newline
- File permissions: `0600` (owner read/write only) on Unix

Example:

```
a1b2c3d4e5f6...64 hex chars total...
```

## Priority Order

When resolving which secret key to use:

1. **Explicit key** (`with_secret_key()`) - Highest priority
2. **File** (`with_secret_key_path()`) - If file exists and is valid
3. **Generate new** - Creates random key, saves to file if path configured

## Security Considerations

- Secret key files are created with `0600` permissions (Unix only)
- Store `data_dir` on encrypted storage in production
- Back up secret key files for disaster recovery
- Never commit secret keys to version control

## Operational Best Practices

1. **Always configure `data_dir`** - Enables automatic identity persistence
2. **Use absolute paths** - Avoids confusion with working directory changes
3. **Back up the key file** - Required for node recovery
4. **Monitor for key changes** - Log warns if EndpointId changes unexpectedly

## Troubleshooting

### Node not recognized after restart

**Symptom:** Other nodes treat restarted node as new peer

**Check:**

1. Is `data_dir` configured?
2. Does `{data_dir}/iroh_secret_key` exist?
3. Are file permissions correct (readable by process)?

### Permission denied on key file

**Symptom:** `Failed to read secret key from file: Permission denied`

**Fix:**

```bash
chmod 600 /path/to/data_dir/iroh_secret_key
chown <user> /path/to/data_dir/iroh_secret_key
```

### Invalid key format

**Symptom:** `Failed to parse secret key: invalid hex`

**Check:** Key file must contain exactly 64 hex characters (0-9, a-f, A-F)

## Related

- `crates/aspen-cluster/src/lib.rs` - `IrohEndpointConfig` and `IrohEndpointManager`
- `crates/aspen-cluster/src/bootstrap.rs` - `build_iroh_config_from_node_config()`
- `.claude/decisions/2026-01-06_cluster_persistence_analysis.md` - Full persistence analysis
