# Aspen CLI Feature Recommendations

**Date**: 2025-12-26
**Analysis Type**: ULTRA mode deep analysis
**Context**: Research-driven recommendations for new aspen-cli commands and arguments

## Executive Summary

After comprehensive analysis of aspen-cli's current implementation (~100+ operations across 15 command categories) and research into industry-leading CLIs (etcdctl, CockroachDB, Consul, FoundationDB, redis-cli), this document provides prioritized recommendations for new features.

**Current Strengths**:

- Comprehensive command coverage (cluster, kv, sql, coordination primitives)
- Clean clap-based architecture with consistent patterns
- JSON output for scripting
- Full Tiger Style compliance (bounded operations, explicit errors)

## High Priority Recommendations

### 1. Shell Completion Generation

**Why**: Industry standard for production CLIs. Reduces friction, prevents typos, improves discoverability.

**Implementation**:

```rust
// In cli.rs, add to Commands enum:
/// Generate shell completion scripts.
Completion {
    /// Shell to generate completions for.
    #[arg(value_enum)]
    shell: clap_complete::Shell,
},
```

**Supported shells**: bash, zsh, fish, powershell, elvish

**Usage**:

```bash
# Install completions
aspen-cli completion bash > ~/.local/share/bash-completion/completions/aspen-cli
aspen-cli completion zsh > ~/.zfunc/_aspen-cli
aspen-cli completion fish > ~/.config/fish/completions/aspen-cli.fish

# In .bashrc/.zshrc
eval "$(aspen-cli completion bash)"
```

**Priority**: HIGH - Low effort, high impact

---

### 2. Watch Operations (Real-Time Event Streaming)

**Why**: Critical for distributed coordination use cases. etcd and Consul both provide watch primitives. Enables reactive applications without polling.

**Commands**:

```bash
# Watch single key
aspen-cli kv watch <key> [--revision=N]

# Watch prefix (all keys with prefix)
aspen-cli kv watch --prefix <prefix>

# Watch cluster events (leader changes, membership changes)
aspen-cli cluster watch

# Output format
aspen-cli kv watch mykey --json
# {"type": "put", "key": "mykey", "value": "newval", "revision": 42}
```

**RPC Protocol Addition**:

```rust
// In client_rpc.rs
WatchKey {
    key: String,
    prefix: bool,
    start_revision: Option<u64>,
},

WatchClusterEvents,
```

**Implementation Notes**:

- Use streaming over the existing Iroh QUIC connection
- Server sends events as they occur
- Client should handle reconnection with revision-based resume
- Bounded buffer (Tiger Style: max 1000 queued events)

**Priority**: HIGH - Key differentiator for distributed coordination

---

### 3. Snapshot Export/Restore (Backup & Recovery)

**Why**: Critical for disaster recovery. etcd's `snapshot save/restore` is the gold standard. Currently aspen-cli only has `cluster snapshot` (triggers internal snapshot).

**Commands**:

```bash
# Export snapshot to file
aspen-cli cluster snapshot export /path/to/backup.snap
aspen-cli cluster snapshot export --output-dir=/backups/

# Show snapshot metadata without restoring
aspen-cli cluster snapshot status /path/to/backup.snap
# Revision: 12345
# Members: 3
# Size: 45 MB
# Hash: blake3:abc123...

# List available snapshots (if stored on node)
aspen-cli cluster snapshot list

# Restore from snapshot (dangerous - new cluster)
aspen-cli cluster snapshot restore /path/to/backup.snap \
  --data-dir=/new/aspen/data \
  --initial-cluster=node1=addr1,node2=addr2
```

**RPC Protocol Addition**:

```rust
ExportSnapshot {
    /// Include metadata in export.
    include_metadata: bool,
},

GetSnapshotStatus {
    /// Optional: hash of snapshot to check (if not provided, check latest).
    snapshot_hash: Option<String>,
},

ListSnapshots {
    limit: u32,
},
```

**Implementation Notes**:

- Snapshot export streams SQLite database + metadata
- Use iroh-blobs for efficient transfer (content-addressed, resumable)
- Restore is typically a node-side operation, not RPC
- Include integrity verification (BLAKE3 hash)

**Priority**: HIGH - Essential for production deployments

---

### 4. Cluster Diagnostics & Debug Bundle

**Why**: CockroachDB's `debug zip` is incredibly useful for support and troubleshooting. Collect logs, metrics, config, and state in one command.

**Commands**:

```bash
# Automated health checks
aspen-cli cluster diagnose
# Checking membership... OK (3 voters)
# Checking Raft consensus... OK (leader: node-1, term: 42)
# Checking replication lag... OK (max: 5ms)
# Checking disk usage... WARNING (85% full)
# Checking connection health... OK (all peers reachable)

# Collect debug bundle for support
aspen-cli cluster debug-bundle /path/to/aspen-debug.zip
# or stream to stdout
aspen-cli cluster debug-bundle --stdout | gzip > debug.tar.gz

# Options
aspen-cli cluster debug-bundle \
  --include-logs \
  --include-metrics \
  --include-config \
  --redact-secrets \
  --duration=1h  # collect last hour of logs
```

**Debug bundle contents**:

- Raft metrics snapshot
- Cluster topology
- Recent log entries
- SQLite state machine stats
- Prometheus metrics
- Node configuration
- Peer connection status

**RPC Protocol Addition**:

```rust
GetDiagnostics,

CollectDebugBundle {
    include_logs: bool,
    include_metrics: bool,
    include_config: bool,
    redact_secrets: bool,
    log_duration_seconds: Option<u64>,
},
```

**Priority**: HIGH - Critical for production support

---

### 5. Node Removal Commands

**Why**: Currently have `cluster add-learner`, `cluster promote`, `cluster change-membership`, but no explicit graceful node removal. This is a gap vs etcd/Consul.

**Commands**:

```bash
# Graceful removal (waits for data migration)
aspen-cli cluster remove-node --node-id=3

# Force removal (for failed nodes that can't respond)
aspen-cli cluster remove-node --node-id=3 --force

# Decommission (mark for removal, allows graceful drain)
aspen-cli cluster decommission --node-id=3
aspen-cli cluster recommission --node-id=3  # cancel
```

**RPC Protocol Addition**:

```rust
RemoveNode {
    node_id: u64,
    force: bool,
},

DecommissionNode {
    node_id: u64,
},

RecommissionNode {
    node_id: u64,
},
```

**Priority**: HIGH - Essential for cluster operations

---

## Medium Priority Recommendations

### 6. Multi-Operation Transactions

**Why**: etcd's `txn` command enables atomic multi-key operations with conditions. Current `kv cas` only works on single keys.

**Commands**:

```bash
# Transaction with conditions
aspen-cli kv txn << 'EOF'
# Conditions (all must be true)
compare key1 = "expected_value"
compare key2 version > 5

# Success operations (applied atomically if conditions pass)
success put key1 "new_value"
success put key2 "another_value"
success delete key3

# Failure operations (applied if conditions fail)
failure get key1
EOF

# Single-line transaction
aspen-cli kv txn \
  --compare='key1 = "old"' \
  --success='put key1 "new"' \
  --failure='get key1'
```

**RPC Protocol Addition**:

```rust
Transaction {
    compares: Vec<TxnCompare>,
    success: Vec<TxnOp>,
    failure: Vec<TxnOp>,
},

// Supporting types
struct TxnCompare {
    key: String,
    target: CompareTarget,  // Value, Version, CreateRevision, ModRevision
    result: CompareResult,  // Equal, NotEqual, Greater, Less
    value: Option<Vec<u8>>,
    version: Option<u64>,
}

enum TxnOp {
    Put { key: String, value: Vec<u8> },
    Delete { key: String },
    Get { key: String },
}
```

**Priority**: MEDIUM - Powerful but complex; CAS covers many use cases

---

### 7. Configuration File Support

**Why**: Production deployments need persistent configuration. Currently requires environment variables or repeated flags.

**Implementation**:

```toml
# ~/.config/aspen/config.toml

[default]
ticket = "aspen..."
token = "eyJ..."
timeout = 5000

[clusters.production]
ticket = "aspen..."
token = "eyJ..."

[clusters.staging]
ticket = "aspen..."
```

**Commands**:

```bash
# Use specific cluster profile
aspen-cli --cluster=production kv get mykey

# List configured clusters
aspen-cli config list

# Add/update cluster config
aspen-cli config set production --ticket="aspen..."

# Show current config
aspen-cli config show
```

**Priority**: MEDIUM - Quality of life for operators

---

### 8. Progress Indicators

**Why**: Long-running operations (snapshot export, debug bundle, batch writes) should show progress.

**Implementation**:

- Use `indicatif` crate for progress bars and spinners
- Show progress for operations >2 seconds
- Support `--no-progress` flag for scripting

**Example**:

```
$ aspen-cli cluster snapshot export backup.snap
Exporting snapshot... ━━━━━━━━━━━━━━━━━━━━ 45/45 MB (100%)
Verifying integrity... OK
Saved to backup.snap
```

**Priority**: MEDIUM - UX improvement

---

### 9. Output Format Options

**Why**: Different use cases need different formats. Currently only `--json` or human-readable.

**Implementation**:

```bash
aspen-cli --output=json kv scan      # Current --json behavior
aspen-cli --output=yaml kv scan      # YAML output
aspen-cli --output=table kv scan     # Explicit table (default)
aspen-cli --output=compact kv scan   # TSV for grep/awk
aspen-cli --output=quiet kv set key val  # Minimal output

# For table/tsv output
aspen-cli --no-headers kv scan       # Skip column headers
```

**Priority**: MEDIUM - Scripting ergonomics

---

### 10. Defragmentation

**Why**: SQLite and Redb accumulate fragmentation over time. etcd has `defrag` command.

**Commands**:

```bash
# Compact SQLite state machine
aspen-cli cluster defrag

# Show fragmentation stats
aspen-cli cluster defrag --status
# SQLite: 15% fragmented (12 MB reclaimable)
# Redb log: 8% fragmented (5 MB reclaimable)
```

**RPC Protocol Addition**:

```rust
Defragment,
GetDefragmentStatus,
```

**Priority**: MEDIUM - Maintenance operation

---

## Lower Priority Recommendations

### 11. Interactive Mode / REPL

**Why**: Exploratory workflows benefit from persistent connection and command history.

**Implementation**:

```bash
$ aspen-cli --cluster=production interactive
aspen> kv get mykey
"hello world"
aspen> kv set mykey "updated"
OK
aspen> cluster status
...
aspen> exit
```

**Priority**: LOW - aspen-tui already provides excellent interactive experience

---

### 12. Key History / Audit Log

**Why**: Debugging and compliance often require seeing who changed what and when.

**Commands**:

```bash
# Show revision history for a key
aspen-cli kv history mykey --limit=10

# Show all operations in revision range
aspen-cli cluster audit --from-revision=100 --to-revision=200
```

**Priority**: LOW - Requires significant backend changes

---

### 13. Performance Profiling

**Why**: Debugging slow operations.

**Commands**:

```bash
# CPU profile (requires tokio-console or pprof integration)
aspen-cli cluster profile --cpu --duration=30s --output=profile.pb

# Slow operation log
aspen-cli cluster slow-log --threshold=100ms
```

**Priority**: LOW - Advanced debugging

---

## Implementation Roadmap

### Phase 1 (Immediate - Low Effort, High Impact)

1. **Shell Completion** - 1-2 hours, no protocol changes
2. **Node Removal Commands** - Uses existing membership RPC

### Phase 2 (Near-Term - Medium Effort, High Value)

3. **Watch Operations** - Requires streaming RPC support
4. **Snapshot Export/Restore** - Builds on existing snapshot infrastructure
5. **Cluster Diagnostics** - Aggregates existing metrics

### Phase 3 (Future - Higher Effort)

6. **Transactions** - Significant protocol and state machine changes
7. **Configuration File** - Quality of life
8. **Progress Indicators** - UX polish

---

## Technical Notes

### Clap Shell Completion Implementation

Add to `Cargo.toml`:

```toml
[dependencies]
clap_complete = "4"
```

Add to `cli.rs`:

```rust
use clap_complete::{generate, Shell};
use std::io;

// In Commands enum:
/// Generate shell completion scripts.
Completion {
    #[arg(value_enum)]
    shell: Shell,
},

// In run():
Commands::Completion { shell } => {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "aspen-cli", &mut io::stdout());
    Ok(())
}
```

### Watch Streaming Protocol

For watch operations, consider:

- Reuse existing QUIC streams with bidirectional messaging
- Server pushes events, client can send keepalive
- Include revision numbers for resume on reconnect
- Bounded event queue (Tiger Style: max 1000 pending events)

### Debug Bundle Format

Consider tarball with:

```
aspen-debug/
  metadata.json          # timestamp, version, cluster ID
  raft-metrics.json      # openraft metrics snapshot
  cluster-state.json     # nodes, voters, learners
  prometheus-metrics.txt # Prometheus format
  sqlite-stats.json      # page count, WAL size, fragmentation
  peer-connections.json  # connection status per peer
  config.json           # redacted node configuration
  logs/                 # recent log entries (if requested)
```

---

## Sources

- etcd documentation: https://etcd.io/docs/
- CockroachDB CLI: https://www.cockroachlabs.com/docs/stable/cockroach-commands
- FoundationDB fdbcli: https://apple.github.io/foundationdb/command-line-interface.html
- Consul CLI: https://developer.hashicorp.com/consul/commands
- redis-cli: https://redis.io/docs/latest/develop/tools/cli/
- CLI UX Guidelines: https://clig.dev/
- clap_complete: https://docs.rs/clap_complete/latest/clap_complete/
