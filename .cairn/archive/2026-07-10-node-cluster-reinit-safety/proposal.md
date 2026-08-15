## Why

Node and cluster initialization are destructive lifecycle boundaries: they write durable config, identity, manifest, and receipt files into explicit state roots. Re-running init over an existing node root or cluster manifest could hide stale lifecycle evidence, overwrite operator intent, or leave partial state that looks fresh.

Operators need init to fail closed unless they explicitly request a reset, and cluster state roots must not accidentally point at ambient current or parent directories.

## What Changes

- Deny `molten node init` and profile-backed init when the target state root already has initialized, running, stopped, or inconsistent lifecycle evidence.
- Deny non-force `molten cluster init` when the cluster manifest already exists or any planned node root has lifecycle evidence.
- Add `molten cluster init --force` to reset only the planned node roots before writing new node lifecycle state and the cluster manifest.
- Reject ambient `.` and `..` cluster state roots before planning nodes.
- Document the explicit reset behavior and add positive/negative unit and CLI coverage.

## Impact

- **Files**: `src/node/parts/daemon/*`, `src/cluster.rs`, `src/cli/ops/cluster.rs`, `tests/parts/cliharness/*`, `README.md`, Cairn lifecycle artifacts.
- **Testing**: focused daemon/cluster unit tests, CLI integration tests for force/non-force behavior, format, Cairn validation/gates, and broader Cargo validation.
- **Safety**: reset remains explicit and scoped to planned node roots; lifecycle classifiers are evidence guards only and do not grant authority, policy, resource, provenance, transport, or release trust.
