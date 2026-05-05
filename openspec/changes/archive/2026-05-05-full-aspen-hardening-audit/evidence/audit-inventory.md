# Full Aspen Hardening Audit Inventory
Status: captured (Phase 2 seed inventory).

## Collection policy
- Static source handles only; this pass did not read live cluster tickets, auth tokens, root tokens, bearer strings, private keys, passwords, or connection strings.
- Credential-like evidence must be represented by source paths, counts, permissions expectations, lengths, hashes, or `[REDACTED]` placeholders, never values.
- This inventory is a seed; Phase 3-5 tasks must refine each row into matrix entries and negative evidence.

## Parsed authority enums
- ClientRpcRequest_variants: 350
- CiRequest_variants: 25
- Operation_variants: 30
- Capability_variants: 30
- Audience_variants: 2

### Operation variants
`Read`, `Write`, `BatchRead`, `BatchWrite`, `Delete`, `Watch`, `ClusterAdmin`, `ShellExecute`, `SecretsRead`, `SecretsWrite`, `SecretsDelete`, `SecretsList`, `TransitEncrypt`, `TransitDecrypt`, `TransitSign`, `TransitVerify`, `TransitKeyManage`, `PkiIssue`, `PkiRevoke`, `PkiReadCa`, `PkiManage`, `SecretsAdmin`, `NetConnect`, `NetPublish`, `NetUnpublish`, `NetAdmin`, `FederationPull`, `FederationPush`, `SnixRead`, `SnixWrite`

### Capability variants
`Read`, `Write`, `Delete`, `Full`, `Watch`, `ClusterAdmin`, `Delegate`, `ShellExecute`, `SecretsRead`, `SecretsWrite`, `SecretsDelete`, `SecretsList`, `SecretsFull`, `TransitEncrypt`, `TransitDecrypt`, `TransitSign`, `TransitVerify`, `TransitKeyManage`, `PkiIssue`, `PkiRevoke`, `PkiReadCa`, `PkiManage`, `SecretsAdmin`, `NetConnect`, `NetPublish`, `NetAdmin`, `FederationPull`, `FederationPush`, `SnixRead`, `SnixWrite`

### CI request variants
`CiTriggerPipeline`, `CiGetStatus`, `CiGetRunReceipt`, `CiListRuns`, `CiCancelRun`, `CiWatchRepo`, `CiUnwatchRepo`, `CiListArtifacts`, `CiGetArtifact`, `CiGetJobLogs`, `CiSubscribeLogs`, `CiGetJobOutput`, `CiGetRefStatus`, `CacheQuery`, `CacheStats`, `CacheDownload`, `NixCacheGetPublicKey`, `SnixDirectoryGet`, `SnixDirectoryPut`, `SnixPathInfoGet`, `SnixPathInfoPut`, `CacheMigrationStart`, `CacheMigrationStatus`, `CacheMigrationCancel`, `CacheMigrationValidate`

## Audit surface table
| Domain | Asset | Source handles | Expected control | Current evidence | Next audit action |
| --- | --- | --- | --- | --- | --- |
| auth/request-classification | client RPC operations | `crates/aspen-client-api/src/messages/mod.rs:571`<br>`crates/aspen-client-api/src/messages/to_operation/mod.rs`<br>`crates/aspen-rpc-handlers/src/client.rs:353` | ClientRpcRequest::to_operation -> Operation; public exemptions explicit; auth gate before HandlerRegistry dispatch | enum counts + existing to_operation tests | Phase 3 variant matrix and fail-closed drift test |
| auth/request-classification | CI domain operations | `crates/aspen-client-api/src/messages/ci.rs:42` | CiRequest mappings must mirror security-sensitive ClientRpcRequest operations where applicable | CiRequest count and source handle | Phase 3 CiRequest operation matrix |
| auth/capabilities | capability tokens and operation matching | `crates/aspen-auth-core/src/capability.rs:113`<br>`crates/aspen-auth-core/src/capability.rs:853`<br>`crates/aspen-auth-core/src/token.rs:20` | Capability::authorizes over explicit Operation variants; Audience::Key presenter binding where used | Operation/Capability/Audience variant inventory | Phase 3 negative capability tests |
| dispatch/proxy | handler dispatch and proxy forwarding | `crates/aspen-rpc-core/src/handler.rs`<br>`crates/aspen-rpc-handlers/src/client.rs`<br>`crates/aspen-client-api/src/messages/to_operation/forge_ops.rs` | auth-before-dispatch; forwarded requests preserve verified tokens only when proxy policy permits | source handles from previous hardening slices | Phase 3 source-order/drift evidence |
| federation | federation credentials and pull/push scopes | `crates/aspen-federation/src/token_store.rs`<br>`crates/aspen-federation/src/sync/handler.rs`<br>`crates/aspen-client-api/src/messages/to_operation/forge_ops.rs` | FederationPull/FederationPush operations; proxy bearer proof requirements; stored credential lookup by cluster key | source handles and existing federation auth scope tests | Phase 3 federation matrix |
| storage/reserved-prefixes | Raft KV and reserved domain prefixes | `crates/aspen-raft-kv/src/lib.rs`<br>`crates/aspen-client-api/src/messages/to_operation/batch_ops.rs`<br>`crates/aspen-client-api/src/messages/batch.rs` | domain-specific caps or fail-closed ClusterAdmin for reserved prefixes across read/write/scan/watch/batch/conditional keys | recent SNIX reserved KV hardening commits + batch condition tests | Phase 3 complete prefix inventory |
| snix/supply-chain | SNIX store/cache/build/eval authority | `crates/aspen-snix/src`<br>`crates/aspen-snix-bridge/src`<br>`crates/aspen-ci-executor-nix/src`<br>`crates/aspen-nix-cache-gateway/src` | SnixRead/SnixWrite capabilities for DirectoryService/PathInfoService; build sandbox and cache gateway documented separately | crate source handles | Phase 5 sandbox/cache gateway audit |
| secrets/trust | cluster secrets, trust shares, SOPS material | `crates/aspen-trust/src`<br>`crates/aspen-secrets-core/src`<br>`crates/aspen-secrets/src`<br>`openspec/specs/trust-crypto-secrets-extraction/spec.md` | secrets remain in intended core/runtime boundaries; no operator output of secret payloads | existing trust/secrets extraction specs | Phase 4 redaction and boundary checks |
| tokens/tickets/filesystem | cluster tickets, root/bootstrap tokens, Iroh secret keys | `crates/aspen-ticket/src`<br>`crates/aspen-ci-executor-vm/src/config.rs:40`<br>`crates/aspen-cluster/src/endpoint_manager.rs:493` | owner-only permissions for local secret keys/token/ticket files; never print credential values | permission source handles; no real secret reads | Phase 4 permission and output tests |
| jobs/ci/execution | shell worker, VM worker, CI execution, deploy control plane | `crates/aspen-jobs-worker-shell/src/lib.rs:174`<br>`crates/aspen-ci-executor-vm/src/config.rs:26`<br>`crates/aspen-ci/src`<br>`crates/aspen-client-api/src/messages/to_operation/deploy_ops.rs` | ShellExecute tokens; VM cluster ticket injection; deploy requires ClusterAdmin; receipts identify run/artifact handles | source handles and deploy status auth tests | Phase 5 execution boundary evidence |
| dogfood/operator-evidence | dogfood/CI/deploy receipts and diagnostics | `crates/aspen-dogfood/src/receipt.rs`<br>`openspec/specs/dogfood-evidence/spec.md` | schema-versioned receipts with identifiers and failure summaries, no credential values | canonical dogfood evidence spec | Phase 4 synthetic redaction fixtures |
| network/transport | Iroh ALPN, node/client/federation/blob/snapshot transports | `crates/aspen-transport/src`<br>`crates/aspen-net/src/handler.rs`<br>`crates/aspen-rpc-core/src/handler.rs` | protected control plane remains Iroh/Raft/auth path; HTTP surfaces are data-plane/compatibility only | source handles and AGENTS.md project constraints | Phase 5 transport boundary audit |
| nix/build/supply-chain | flake inputs, vendored deps, native build paths, fallback subprocesses | `flake.nix`<br>`flake.lock`<br>`vendor/`<br>`crates/aspen-ci-executor-nix/src` | pinned inputs; explicit fallback feature gates; build artifact identity | flake lock + Tiger Style gate | Phase 5 supply-chain evidence |
| unsafe/feature-gates | unsafe/public unsafe API and feature-gated authority surfaces | `dylint.toml`<br>`Cargo.toml`<br>`crates/*/Cargo.toml` | Tiger Style public_unsafe_api deny; feature-gated deps recorded for audit commands | Tiger Style clean at previous commit; this inventory records feature coverage need | Phase 5 unsafe/feature inventory |

## Public/protected classification seed
- **Intentionally public:** bootstrap/challenge/health-style endpoints only when `to_operation` returns `None` by explicit test-backed rationale; exact list remains Phase 3 matrix work.
- **Capability-protected:** ordinary KV, Forge, coordination, job, CI, SNIX, federation, and domain operations via explicit `Operation` variants.
- **Presenter-bound:** key-bound delegated client tokens through authenticated Iroh remote identity.
- **Cluster-admin:** deploy lifecycle/status, membership, administrative maintenance, and fail-closed reserved-prefix mixed operations.
- **Federation-proxy delegation:** short-lived bearer-only proof with bounded delegation and pull/push-only child capabilities.
- **SNIX-specific:** DirectoryService/PathInfoService/store/cache access through `SnixRead`/`SnixWrite`, not generic `snix:` KV prefixes.
- **Filesystem-permission protected:** local secret keys, token/ticket outputs, VM-injected cluster tickets, and operator config files must be owner-only and never printed.
