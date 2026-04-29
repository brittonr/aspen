# Next Decomposition Wave Selection Baseline

Source: `docs/crate-extraction.md`, `docs/crate-extraction/*.md`, `docs/crate-extraction/policy.ncl`, and workspace crate inventory from `crates/*` on 2026-04-29.

## Completed or already-proven families

| Family | Status | Evidence source | Next action |
| --- | --- | --- | --- |
| Redb Raft KV | Partly extraction-ready: `aspen-kv-types`, `aspen-raft-kv-types`, `aspen-redb-storage`, and `aspen-raft-kv`; adapter/compat shells remain workspace-internal. | `docs/crate-extraction.md`; per-layer manifests under `docs/crate-extraction/aspen-*.md`. | Keep publish/repo split blocked on license/publication policy; finish `aspen-raft-network` adapter and compatibility shells later. |
| Coordination | `extraction-ready-in-workspace` for `aspen-coordination` and `aspen-coordination-protocol`. | `docs/crate-extraction/coordination.md`; policy entries `aspen_coordination`, `aspen_coordination_protocol`. | Keep publication blocked on license/publication policy. |
| Protocol/wire | Workspace-internal; scoped by protocol/wire manifest and policy entries for client/forge/jobs/coordination protocols. | `docs/crate-extraction/protocol-wire.md`; policy entries `aspen_client_api`, `aspen_forge_protocol`, `aspen_jobs_protocol`, `aspen_coordination_protocol`. | Finish downstream fixture and compatibility rails before readiness raises. |
| Transport/RPC | Workspace-internal; generic helpers/RPC dispatch are separated enough for targeted downstream evidence, but runtime adapters remain explicit. | `docs/crate-extraction/transport-rpc.md`; policy entries `aspen_transport`, `aspen_rpc_core`. | Finish downstream and compatibility evidence before readiness raises. |
| Blob/castore/cache | Workspace-internal with backend-purpose exceptions documented. | `docs/crate-extraction/blob-castore-cache.md`; policy entries `aspen_blob`, `aspen_castore`, `aspen_cache`, `aspen_exec_cache`. | Add downstream fixtures and checker mutation evidence. |
| KV branch/commit DAG | Workspace-internal with branch/DAG manifests. | `docs/crate-extraction/kv-branch-commit-dag.md`; policy entries `aspen_commit_dag`, `aspen_kv_branch`. | Verify downstream fixture and compatibility consumers before readiness raises. |

## Selected next five families

| Order | Family | Selected crates | Why next | First blocker |
| --- | --- | --- | --- | --- |
| 1 | `foundational-types` | `aspen-storage-types`, `aspen-traits`, `aspen-cluster-types`, `aspen-hlc`, `aspen-time`, `aspen-constants` | Foundational type/trait seams unblock later families and keep reusable defaults free of runtime leakage. | Move `SM_KV_TABLE` / `redb::TableDefinition` out of portable storage types; then split or prove narrower KV capability traits in `aspen-traits`. |
| 2 | `auth-ticket` | `aspen-auth-core`, `aspen-auth`, `aspen-ticket`, `aspen-hooks-ticket` | Protects portable auth/ticket wire surfaces and keeps runtime HMAC/verifier/revocation shells opt-in. | Inventory portable consumers still importing through runtime shells; migrate or document compatibility re-exports; add token/ticket serialization proof. |
| 3 | `jobs-ci-core` | `aspen-jobs`, `aspen-jobs-protocol`, `aspen-jobs-guest`, `aspen-jobs-worker-*`, `aspen-ci-core`, `aspen-ci`, `aspen-ci-executor-*` | Jobs/CI is central to Aspen self-hosting and crosses scheduler, run-state, artifact, Nickel config, and executor boundaries. | Separate scheduler/config/run-state contracts from worker/executor runtime shells and concrete process/Nix execution. |
| 4 | `trust-crypto-secrets` | `aspen-trust`, `aspen-crypto`, reusable pure/state-machine surfaces in `aspen-secrets`, with `aspen-secrets-handler` as runtime consumer | Security-sensitive pure logic should be deterministic, testable, and separated from Raft/Iroh/secrets-service shells. | Isolate Shamir/GF/HKDF/share-chain, trust reconfiguration, and decryption-key-selection logic behind explicit deterministic inputs/outputs. |
| 5 | `testing-harness` | `aspen-testing-core`, `aspen-testing`, `aspen-testing-fixtures`, `aspen-testing-madsim`, `aspen-testing-network`, `aspen-testing-patchbay` | Future extraction evidence needs reusable simulation/workload/assertion helpers that do not smuggle Aspen app bootstrap assumptions. | Inventory reusable simulation/workload/assertion helpers separately from cluster bootstrap, node config, concrete transport, and binary shell helpers. |

## Deferred candidates

| Candidate | Reason deferred | Compatibility note |
| --- | --- | --- |
| Config/plugin APIs (`aspen-nickel`, `aspen-plugin-api`) | Important, but jobs/CI exercises Nickel config first and should identify the stable config/plugin seam before standalone plugin/config extraction. | Treat as likely next-wave candidate after jobs/CI core has manifest evidence. |
| Binary shells (`aspen-cli`, `aspen-tui`, `aspen-node`, bridges, gateways, forge web, dogfood) | Final consumers by design, not reusable dependency roots. | Keep auditing that shells stay thin and do not become dependencies of reusable crates. |

## Selection constraints

- Publication and repository split remain blocked until human license/publication policy is decided.
- No selected family may advance beyond `workspace-internal` or `extraction-ready-in-workspace` in this change.
- Any out-of-order implementation must record the bypassed prerequisite and the temporary compatibility guard it relies on.
- `aspen-jobs-protocol` is included only as jobs/CI domain-schema compatibility; generic wire compatibility remains governed by the archived protocol/wire work.
