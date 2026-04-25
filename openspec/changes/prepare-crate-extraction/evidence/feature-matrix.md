# Feature Matrix Evidence

Generated: 2026-04-25T02:16Z

## Reusable KV Stack Crates

| Crate | Feature set | Status | Classification |
| --- | --- | --- | --- |
| `aspen-kv-types` | default | PASS | reusable default |
| `aspen-raft-kv-types` | default | PASS | reusable default |
| `aspen-redb-storage` | default (no raft-storage) | PASS | reusable default (pure verified helpers only) |
| `aspen-redb-storage` | `raft-storage` | PASS | named reusable feature (OpenRaft storage traits) |
| `aspen-raft-kv` | default | PASS | reusable default (facade skeleton) |
| `aspen-raft-network` | default | PASS | reusable default (network adapter) |

## Foundational/Core Rails

| Command | Status |
| --- | --- |
| `cargo check -p aspen-core --no-default-features` | PASS |
| `cargo check -p aspen-core-no-std-smoke` | PASS |
| `cargo check -p aspen-core-shell --features layer,global-discovery,sql` | PASS |
| `cargo check -p aspen-traits` | PASS |

## Aspen Compatibility Consumers

| Command | Status |
| --- | --- |
| `cargo check -p aspen-raft` | PASS |
| `cargo check -p aspen-raft --no-default-features` | PASS |

## Feature Classification

### Reusable defaults (no opt-in needed)
- `aspen-kv-types`: KV command/response types
- `aspen-raft-kv-types`: OpenRaft app type config, typed enums
- `aspen-redb-storage` (default): pure verified helpers, chain integrity, CAS
- `aspen-raft-kv`: facade configuration and trait surface
- `aspen-raft-network`: network adapter types

### Named reusable features (opt-in required)
- `aspen-redb-storage[raft-storage]`: OpenRaft RaftLogStorage/RaftStateMachine impls

### Runtime/app features (forbidden in reusable defaults)
- trust, secrets, sql, coordination, sharding
- iroh endpoint construction
- handler registry, dogfood, TUI
