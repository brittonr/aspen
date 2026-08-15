## Why

The node daemon, control inbox/outbox, ledger imports, service locks, endpoint identity, and persisted Iroh secret all operate beneath one operator-selected state root, but production code repeatedly reconstructs ambient `PathBuf` values and calls `std::fs`. Some destructive decisions use lexical checks such as `request_path.starts_with(state_root.join(...))`; endpoint secret creation and permission inspection also reopen ambient paths.

These are stronger `cap-std` candidates than ordinary explicit CLI input files because the node is long-lived, ingests peer-derived identifiers, performs destructive cleanup, and protects transport secret material beneath a stable authority boundary.

## What Changes

- Introduce a typed `NodeStateRoot` opened once by the node CLI or daemon shell and retained for the node lifetime.
- Represent inbox, outbox, ledger, identity, service, ingress, receipt, and secret locations as fixed or validated relative node-state locators.
- Replace lexical containment checks and ambient request-path deletion with capability-relative directory entries and leaf operations.
- Open, create, inspect, and rotate endpoint secret files relative to the identity capability while preserving owner-only permission policy.
- Add node-scoped structural enforcement and adversarial tests for symlinks, path replacement, wrong-root handles, stale entries, and secret-file substitution.

## Impact

- **Files**: `src/node/**`, `src/cli/ops/node/**`, node-facing ledger helpers, `src/local_store.rs` or its successor filesystem port, node tests, authority-audit rules, and node filesystem documentation.
- **Testing**: node init/run/status/stop, control queue and archive, service lock recovery, ledger imports, identity persistence and rotation, plus positive and negative authority fixtures.
- **Sequencing**: depends on `complete-cap-std-store-threading` so node state reuses the operational capability-root API and scoped audit model.
- **Claims**: a node-state capability limits local filesystem reach. It does not grant peer, operation, policy, provenance, transport, retention, deployment, or release authority and does not by itself protect secret bytes from a compromised process.
