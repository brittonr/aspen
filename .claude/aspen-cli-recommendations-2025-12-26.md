# Aspen CLI Recommendations

Generated: 2025-12-26

## Executive Summary

The aspen-cli currently exposes **21 commands** across 3 categories (cluster, kv, sql), but the underlying RPC layer supports **80+ operations**. This represents significant untapped functionality. Based on analysis of the codebase and research into mature distributed systems (etcd, FoundationDB, Consul, CockroachDB), this document provides prioritized recommendations for CLI expansion.

## Current State

### Existing Commands

**Cluster (11 commands):** init, status, health, metrics, prometheus, add-learner, promote, change-membership, snapshot, checkpoint-wal, ticket

**KV (8 commands):** get, set, delete, cas, cad, scan, batch-read, batch-write

**SQL (2 commands):** query, file

### Architecture Strengths

- Clean clap derive-based argument parsing
- Environment variable support (ASPEN_TICKET, ASPEN_TOKEN)
- JSON/human-readable output modes
- Retry logic with backoff
- Tiger Style resource bounds enforced

## Priority 1: High-Impact Missing Features

These are operations that mature distributed systems universally expose and users expect:

### 1.1 Coordination Primitives (`aspen coord` or separate subcommands)

**Distributed Locks** - Critical for coordination workflows:

```
aspen lock acquire <key> --holder <id> --ttl <ms> [--timeout <ms>]
aspen lock try-acquire <key> --holder <id> --ttl <ms>
aspen lock release <key> --holder <id> --token <fencing_token>
aspen lock renew <key> --holder <id> --token <fencing_token> --ttl <ms>
```

**Atomic Counters** - Common distributed primitive:

```
aspen counter get <key>
aspen counter incr <key>
aspen counter decr <key>
aspen counter add <key> <amount>
aspen counter set <key> <value>
aspen counter cas <key> --expected <val> --new-value <val>
```

**Signed Counters** (for financial/accounting use cases):

```
aspen signed-counter get <key>
aspen signed-counter add <key> <amount>  # Can be negative
```

**Sequence Generators** - For distributed ID generation:

```
aspen sequence next <key>
aspen sequence reserve <key> <count>
aspen sequence current <key>
```

### 1.2 Lease Management (`aspen lease`)

Essential for ephemeral keys and session management (etcd-style):

```
aspen lease grant <ttl_seconds> [--id <lease_id>]
aspen lease revoke <lease_id>
aspen lease keepalive <lease_id>
aspen lease ttl <lease_id> [--keys]
aspen lease list
aspen kv set <key> <value> --lease <lease_id>  # Extend existing set command
```

### 1.3 Watch/Streaming (`aspen watch`)

Real-time change notifications (like etcd watch):

```
aspen watch <prefix> [--from-revision <index>] [--prev-value]
aspen watch status [--id <watch_id>]
aspen watch cancel <watch_id>
```

Note: This requires streaming support which may need special client handling.

## Priority 2: Operational Excellence

Features that improve day-to-day operations and debugging:

### 2.1 Enhanced Cluster Commands

**Node Information:**

```
aspen cluster info                 # GetNodeInfo - endpoint details
aspen cluster leader              # GetLeader - just the leader ID
```

**Combined Ticket for Multi-Peer Bootstrap:**

```
aspen cluster ticket --combined [--endpoints <ids>]
```

### 2.2 Blob Storage (`aspen blob`)

Content-addressed storage is fully implemented but not CLI-accessible:

```
aspen blob add <file_or_data> [--tag <name>]
aspen blob get <hash> [-o <output_file>]
aspen blob has <hash>
aspen blob ticket <hash>
aspen blob list [--limit <n>] [--token <continuation>]
aspen blob protect <hash> --tag <name>
aspen blob unprotect --tag <name>
```

### 2.3 Service Registry (`aspen service`)

Full service mesh discovery is implemented:

```
aspen service register <name> <instance_id> <address> [--version <v>] [--tags <json>] [--weight <n>]
aspen service deregister <name> <instance_id> --token <fencing_token>
aspen service discover <name> [--healthy-only] [--tags <json>]
aspen service list [--prefix <p>] [--limit <n>]
aspen service instance <name> <instance_id>
aspen service heartbeat <name> <instance_id> --token <fencing_token>
aspen service health <name> <instance_id> --token <fencing_token> --status <healthy|unhealthy>
```

### 2.4 Rate Limiting (`aspen ratelimit`)

Token bucket rate limiter:

```
aspen ratelimit try-acquire <key> --tokens <n> --capacity <max> --rate <per_second>
aspen ratelimit acquire <key> --tokens <n> --capacity <max> --rate <per_second> [--timeout <ms>]
aspen ratelimit available <key> --capacity <max> --rate <per_second>
aspen ratelimit reset <key> --capacity <max> --rate <per_second>
```

## Priority 3: Advanced Coordination

For users building sophisticated distributed applications:

### 3.1 Distributed Barriers (`aspen barrier`)

```
aspen barrier enter <name> --participant <id> --count <required> [--timeout <ms>]
aspen barrier leave <name> --participant <id> [--timeout <ms>]
aspen barrier status <name>
```

### 3.2 Distributed Semaphores (`aspen semaphore`)

```
aspen semaphore acquire <name> --holder <id> --permits <n> --capacity <max> [--ttl <ms>] [--timeout <ms>]
aspen semaphore try-acquire <name> --holder <id> --permits <n> --capacity <max> [--ttl <ms>]
aspen semaphore release <name> --holder <id> [--permits <n>]
aspen semaphore status <name>
```

### 3.3 Read-Write Locks (`aspen rwlock`)

```
aspen rwlock read <name> --holder <id> --ttl <ms> [--timeout <ms>]
aspen rwlock try-read <name> --holder <id> --ttl <ms>
aspen rwlock write <name> --holder <id> --ttl <ms> [--timeout <ms>]
aspen rwlock try-write <name> --holder <id> --ttl <ms>
aspen rwlock release-read <name> --holder <id>
aspen rwlock release-write <name> --holder <id> --token <fencing_token>
aspen rwlock downgrade <name> --holder <id> --token <fencing_token> --ttl <ms>
aspen rwlock status <name>
```

### 3.4 Distributed Queues (`aspen queue`)

Full SQS-style distributed queue:

```
aspen queue create <name> [--visibility-timeout <ms>] [--ttl <ms>] [--max-attempts <n>]
aspen queue delete <name>
aspen queue enqueue <name> <payload> [--ttl <ms>] [--group <id>] [--dedup <id>]
aspen queue dequeue <name> --consumer <id> [--max <n>] --visibility <ms>
aspen queue dequeue <name> --consumer <id> [--max <n>] --visibility <ms> --wait <ms>
aspen queue peek <name> [--max <n>]
aspen queue ack <name> <receipt_handle>
aspen queue nack <name> <receipt_handle> [--to-dlq] [--error <msg>]
aspen queue extend <name> <receipt_handle> --add <ms>
aspen queue status <name>
aspen queue dlq <name> [--max <n>]
aspen queue redrive <name> <item_id>
```

## Priority 4: Multi-Cluster Operations

For federated/hybrid deployments:

### 4.1 Peer Cluster Management (`aspen peer`)

```
aspen peer add <ticket>
aspen peer remove <cluster_id>
aspen peer list
aspen peer status <cluster_id>
aspen peer filter <cluster_id> --type <full|include|exclude> [--prefixes <json>]
aspen peer priority <cluster_id> <priority>
aspen peer enable <cluster_id>
aspen peer disable <cluster_id>
aspen peer origin <key>  # Which cluster did this key come from?
```

## Priority 5: Developer Experience Improvements

### 5.1 Interactive Mode

Like `fdbcli` or `redis-cli`:

```
aspen shell [--ticket <t>]
> kv get foo
> counter incr mycount
> exit
```

### 5.2 Transaction Mode

Like etcd's txn or FoundationDB's begin/commit:

```
aspen txn begin
> set foo bar
> set baz qux
> commit
```

Or single-line:

```
aspen txn --if 'foo == bar' --then 'set foo baz' --else 'delete foo'
```

### 5.3 Conditional Batch Write

Expose the full etcd-style transaction API:

```
aspen kv batch-write-if \
  --condition 'foo == bar' \
  --condition 'version(baz) > 5' \
  --then 'set foo newval' \
  --then 'delete baz'
```

### 5.4 Additional Global Options

```
--format <table|json|csv|minimal>  # More output formats
--no-headers                       # For scripting
--color <auto|always|never>        # Terminal color control
--endpoint <addr>                  # Connect to specific node (bypass discovery)
--retry <n>                        # Override retry count
--consistency <linearizable|stale> # Default consistency level
```

### 5.5 Debug/Diagnostic Commands

Inspired by CockroachDB's debug commands:

```
aspen debug zip --output <file>    # Collect diagnostic bundle
aspen debug topology               # Show shard topology
aspen debug connections            # Show active connections
aspen debug raft-log [--limit <n>] # Inspect Raft log entries
```

## Implementation Recommendations

### Command Organization

Recommended subcommand structure:

```
aspen
├── cluster (existing)
├── kv (existing)
├── sql (existing)
├── lock (new)
├── counter (new)
├── sequence (new)
├── lease (new)
├── watch (new)
├── blob (new)
├── service (new)
├── ratelimit (new)
├── barrier (new)
├── semaphore (new)
├── rwlock (new)
├── queue (new)
├── peer (new)
├── debug (new)
└── shell (new - interactive mode)
```

### Implementation Order

**Phase 1 (Immediate value):**

1. lock, counter, sequence - Most common coordination primitives
2. lease - Essential for ephemeral state management
3. blob - Already implemented, just needs CLI exposure

**Phase 2 (Operational):**
4. service - Service discovery is production-critical
5. watch - Real-time notifications
6. debug - Operational excellence

**Phase 3 (Advanced):**
7. ratelimit, barrier, semaphore, rwlock - Advanced coordination
8. queue - Distributed queuing
9. peer - Multi-cluster federation

**Phase 4 (DX):**
10. shell - Interactive mode
11. txn - Transaction syntax sugar

### Code Structure

For each new subcommand category, create:

- `src/bin/aspen-cli/commands/<category>.rs` - Command definitions
- Add to `src/bin/aspen-cli/commands/mod.rs` - Module export
- Add to `Commands` enum in `src/bin/aspen-cli/cli.rs` - Registration
- Add output types to `src/bin/aspen-cli/output.rs` - Formatting

### Testing Strategy

1. Unit tests for argument parsing
2. Integration tests with mock cluster
3. End-to-end tests with real cluster (smoke tests)
4. Output format validation (JSON schema, human-readable)

## Metrics for Success

- All 80+ RPC operations accessible via CLI
- 100% parity between programmatic and CLI access
- Consistent argument patterns across command categories
- Complete JSON output for scripting
- Comprehensive help text for each command
