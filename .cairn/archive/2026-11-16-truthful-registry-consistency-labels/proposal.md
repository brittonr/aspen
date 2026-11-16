## Why

Issue #47: neither consensus path calls Trellis, yet the single-process control registry (`raft_control_plane`) names its predicate receipts `trellis-*`, marks each with a passing `trellis-predicate` check, records a `trellis-quorum-commit` with `quorum-members` and a commit receipt `quorum` list that no quorum produced, and stamps every read `linearizable`: both callers pass `read_index: committed_index`, so the stale-read check can never fire. Evidence-chain checkpoints declare a `trellis-raft` control-plane mode that no code implements. AGENTS.md and the architecture and runtime references place coordination locks, leases and registries on replicated Raft, which is not connected to those paths.

## What Changes

- The control registry's predicates and checks are renamed for what the process computes (`registry-append-consistency`, `registry-local-commit`, `registry-commit-advancement`, `registry-committed-read`, `registry-snapshot-restore`, evaluator check `in-process-registry`); quorum claims are dropped. Predicate, read and commit receipts move to v2.
- A `local-committed` read mode binds the registry instance's committed state. The registry denies `linearizable` with an explicit diagnostic, and the caller-supplied `read_index` input is removed: reads take the runtime itself.
- Coordination and CLI defaults (coordination command, job schedule, `raft run-fixture`, fixtures) use `local-committed`. Protected coordination operations are admitted with `local-committed` and denied with `local-stale` or `linearizable`. Coordination receipts replace the vacuous `read-index-bound` check with `committed-state-bound`; coordination receipts and status assertions move to v2.
- The in-process engine descriptor and its construction/switchover admission declare only local read modes and local committed-state currentness; they do not advertise linearizable reads or quorum evidence.
- The evidence-chain checkpoint mode becomes `local-checkpoint`; checkpoints move to v2.
- AGENTS.md, `docs/architecture.md` and `docs/runtime-reference.md` describe which paths use the replicated Raft and which use the in-process registry, and that neither calls Trellis.

## Impact

- **Files**: `src/raft/control`, `src/coordination`, `src/evidence/parts/chain`, `src/preserves/parts/rail`, the ledger artifact-kind table, the coordination/job/raft CLI modules, their tests, and the three documents.
- **Compatibility**: receipts and checkpoints with the v1 labels are rejected; no repository fixture, example, configuration or evidence ledger reads them back, so no legacy decode path is kept. Engine descriptor/admission identities change with truthful supported values; group manifest algorithm-contract bytes are unchanged.
- **Owner / consumer**: consensus and coordination. Replicating coordination state through `fabric_consistency::raft`, and moving the replicated Raft transition into `molten-core`, are not part of this change.
