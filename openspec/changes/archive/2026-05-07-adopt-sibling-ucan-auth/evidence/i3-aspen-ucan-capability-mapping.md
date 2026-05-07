# I3 Aspen capability/operation to UCAN mapping

- Change: `adopt-sibling-ucan-auth`
- Task: Write the Aspen capability/operation to UCAN ability/resource mapping table.
- Started: 2026-05-06T23:29:42Z
- Completed: 2026-05-06T23:31:36Z
- Status: captured
- Sources inspected:
  - `crates/aspen-auth-core/src/capability.rs` (`Capability`, `Operation`, `authorizes`, `contains`, `Display`)
  - `../ucan/crates/ucan-core/src/lib.rs` (`CapabilityView`, resource/ability validation)
  - `../ucan/src/verified.rs` / `../ucan/examples/issue_and_verify.rs` (resource prefix + ability segment/wildcard authorization)

## Mapping conventions

UCAN capability fields are `{ with: <resource>, can: <ability> }`.

Aspen should use these conventions for the first UCAN-backed adapter:

- Resource namespace: `aspen:<domain>:<scope>`.
- Empty Aspen prefixes become domain roots such as `aspen:kv:` or `aspen:cache:`; UCAN resource prefix matching then preserves current Aspen empty-prefix-as-all behavior within that domain.
- Ability namespace: `aspen/<domain>/<verb>` or `aspen/<domain>/*` for existing `Full`/admin variants. UCAN ability syntax allows slash-separated segments and `*` wildcard segments; avoid colons in abilities.
- Existing Aspen `Capability` and `Operation` remain the public API. This table defines adapter conversion and is not a new operator-facing token grammar unless a later task explicitly promotes it.
- Prefix/glob edge cases remain Aspen-local before or after UCAN authorization where UCAN only supplies resource-prefix and segment-wildcard ability checks.

## Core KV and cluster capabilities

| Aspen capability | Aspen operation(s) | UCAN resource | UCAN ability | Notes |
| --- | --- | --- | --- | --- |
| `Read { prefix }` | `Read { key }`, `BatchRead { keys }` | `aspen:kv:<prefix>` | `aspen/kv/read` | Every requested key maps to `aspen:kv:<key>` and must be authorized. |
| `Write { prefix }` | `Write { key, .. }`, `BatchWrite { keys }` write entries | `aspen:kv:<prefix>` | `aspen/kv/write` | Batch keys must all pass. |
| `Delete { prefix }` | `Delete { key }`, `BatchWrite { keys }` delete entries | `aspen:kv:<prefix>` | `aspen/kv/delete` | Batch delete keys must all pass. |
| `Full { prefix }` | `Read`, `BatchRead`, `Write`, `BatchWrite`, `Delete`, `Watch` | `aspen:kv:<prefix>` | `aspen/kv/*` | Preserves existing read/write/delete/watch convenience. |
| `Watch { prefix }` | `Watch { key_prefix }` | `aspen:kv:<prefix>` | `aspen/kv/watch` | Request resource is `aspen:kv:<key_prefix>`. |
| `ClusterAdmin` | `ClusterAdmin { action }` | `aspen:cluster:` | `aspen/cluster/admin` | `action` remains audit metadata, not part of UCAN authorization. |
| `Delegate` | token delegation/attenuation, not an `Operation` variant | `aspen:auth:delegate` | `aspen/auth/delegate` | Must be enforced in Aspen adapter around child-token issuance/proof attenuation; sibling UCAN proof chains do not by themselves preserve Aspen's legacy `Delegate` gate. |

## Runtime shell and secrets capabilities

| Aspen capability | Aspen operation(s) | UCAN resource | UCAN ability | Notes |
| --- | --- | --- | --- | --- |
| `ShellExecute { command_pattern, working_dir }` | `ShellExecute { command, working_dir }` | `aspen:shell:<working-dir-or-global>:<command-pattern>` | `aspen/shell/execute` | UCAN cannot express Aspen's current command glob + optional working-dir semantics alone; adapter must keep Aspen-local `authorizes_shell_command`/directory checks after UCAN broad admission. |
| `SecretsRead { mount, prefix }` | `SecretsRead { mount, path }` | `aspen:secrets:<mount>:<prefix>` | `aspen/secrets/read` | Mount equality + path prefix remain encoded in resource string. |
| `SecretsWrite { mount, prefix }` | `SecretsWrite { mount, path }` | `aspen:secrets:<mount>:<prefix>` | `aspen/secrets/write` |  |
| `SecretsDelete { mount, prefix }` | `SecretsDelete { mount, path }` | `aspen:secrets:<mount>:<prefix>` | `aspen/secrets/delete` |  |
| `SecretsList { mount, prefix }` | `SecretsList { mount, path }` | `aspen:secrets:<mount>:<prefix>` | `aspen/secrets/list` |  |
| `SecretsFull { mount, prefix }` | read/write/delete/list for mount+path | `aspen:secrets:<mount>:<prefix>` | `aspen/secrets/*` | Preserves existing full secrets access, but not secrets admin. |
| `SecretsAdmin` | `SecretsAdmin`, all transit/PKI/secrets ops currently covered by admin | `aspen:secrets:` | `aspen/secrets/admin` plus adapter-expanded admin implication | Existing `SecretsAdmin` authorizes secrets, transit, and PKI admin-class operations. Adapter must either expand into multiple UCAN capabilities or preserve Aspen-local admin implication. |

## Secrets sub-engines, net, CI/jobs, data services

| Aspen capability family | Aspen operation family | UCAN resource | UCAN ability | Notes |
| --- | --- | --- | --- | --- |
| `TransitEncrypt/Decrypt/Sign/Verify/KeyManage { key_prefix }` | matching `Transit* { key_name }` | `aspen:transit:<key-prefix>` | `aspen/transit/{encrypt,decrypt,sign,verify,manage}` | `SecretsAdmin` implication remains adapter-owned. |
| `PkiIssue { role_prefix }` | `PkiIssue { role }` | `aspen:pki:<role-prefix>` | `aspen/pki/issue` |  |
| `PkiRevoke`, `PkiReadCa`, `PkiManage` | matching PKI ops; manage also covers issue/revoke/read-ca | `aspen:pki:` | `aspen/pki/{revoke,read-ca,manage}` | `PkiManage` implication across PKI verbs remains adapter-owned or expands to multiple UCAN capabilities. |
| `NetConnect { service_prefix }` | `NetConnect { service, port }` | `aspen:net:<service-prefix>` | `aspen/net/connect` | Port is request metadata, not scoped by current capability. |
| `NetPublish { service_prefix }` | `NetPublish`, `NetUnpublish` | `aspen:net:<service-prefix>` | `aspen/net/publish` | Existing publish covers unpublish; preserve as adapter implication or add `aspen/net/unpublish` expansion. |
| `NetAdmin` | all net ops + net admin | `aspen:net:` | `aspen/net/admin` plus adapter-expanded admin implication |  |
| `CiRead/CiWrite { resource_prefix }` | `CiRead/CiWrite { resource }` | `aspen:ci:<resource-prefix>` | `aspen/ci/{read,write}` |  |
| `JobsRead/JobsWrite { resource_prefix }` | `JobsRead/JobsWrite { resource }` | `aspen:jobs:<resource-prefix>` | `aspen/jobs/{read,write}` |  |
| `BlobRead/BlobWrite { resource_prefix }` | `BlobRead/BlobWrite { resource }` | `aspen:blob:<resource-prefix>` | `aspen/blob/{read,write}` |  |
| `DocsRead/DocsWrite { resource_prefix }` | `DocsRead/DocsWrite { resource }` | `aspen:docs:<resource-prefix>` | `aspen/docs/{read,write}` |  |
| `HooksRead/HooksWrite { resource_prefix }` | `HooksRead/HooksWrite { resource }` | `aspen:hooks:<resource-prefix>` | `aspen/hooks/{read,write}` | `HooksWrite` covers trigger/mutate as today. |
| `KvMetadataRead/KvMetadataWrite { resource_prefix }` | matching KV metadata ops | `aspen:kvmeta:<resource-prefix>` | `aspen/kvmeta/{read,write}` |  |
| `CoordinationRead/CoordinationWrite { resource_prefix }` | matching coordination ops | `aspen:coordination:<resource-prefix>` | `aspen/coordination/{read,write}` |  |
| `SqlRead { resource_prefix }` | `SqlRead { resource }` | `aspen:sql:<resource-prefix>` | `aspen/sql/read` | No write ability exists in current Aspen auth-core. |
| `ObservabilityRead/ObservabilityWrite { resource_prefix }` | matching observability ops | `aspen:observability:<resource-prefix>` | `aspen/observability/{read,write}` |  |
| `AutomergeRead/AutomergeWrite { resource_prefix }` | matching automerge ops | `aspen:automerge:<resource-prefix>` | `aspen/automerge/{read,write}` |  |
| `FederationPull/FederationPush { repo_prefix }` | `FederationPull/FederationPush { fed_id }` | `aspen:federation:<repo-prefix>` | `aspen/federation/{pull,push}` | Preserves federation credential admission with UCAN mechanics. |
| `CacheRead/CacheWrite { resource_prefix }` | matching binary-cache ops | `aspen:cache:<resource-prefix>` | `aspen/cache/{read,write}` |  |
| `SnixRead/SnixWrite { resource_prefix }` | matching SNIX ops | `aspen:snix:<resource-prefix>` | `aspen/snix/{read,write}` | Resource examples remain `dir:<digest>` / `pathinfo:<digest>` after namespace prefix. |

## Unsupported or intentionally Aspen-local semantics

| Surface | Decision | Reason |
| --- | --- | --- |
| Command globs in `ShellExecute` | Aspen-local post-filter | UCAN ability/resource matching does not model Aspen's command glob grammar or optional working-directory constraint. |
| Admin implication sets (`SecretsAdmin`, `PkiManage`, `NetAdmin`, `Full`, `SecretsFull`) | Adapter-expanded or Aspen-local implication | UCAN can represent wildcard ability segments, but Aspen's cross-family implications need explicit expansion or a compatibility post-check. |
| `Delegate` authorization | Aspen-local issuance gate plus UCAN proof-chain attenuation | Sibling UCAN supports proof chains/delegation, but Aspen's legacy `Delegate` bit is an authorization policy that must be checked before issuing child tokens. |
| Batch operations | Adapter-level all-items check | UCAN verifies one resource/ability request at a time; Aspen batch semantics require every key/resource in the batch to pass. |
| Action/audit fields (`ClusterAdmin.action`, `NetAdmin.action`, `SecretsAdmin.action`, ports, write values) | Audit metadata, not UCAN resource scope | Current Aspen auth does not scope by those fields. Preserve logging/redaction separately. |
| Remote DID/proof/revocation/replay backends | Runtime `aspen-auth` integration, not mapping table | Sibling `ucan` exposes hooks only; Aspen must wire real backends in later tasks. |

## Implementation notes for adapter tasks

- Provide deterministic `Capability -> Vec<ucan::CapabilityDocument>` conversion because some Aspen variants expand to multiple UCAN abilities or need local implication metadata.
- Provide deterministic `Operation -> Vec<(resource, ability)>` conversion because batch operations expand into one request per key/resource.
- Keep `Capability::contains` compatibility fixtures: UCAN wildcard ability containment should be checked against Aspen's existing prefix, mount, shell, and admin implication semantics.
- Validate generated UCAN resource/ability strings through `ucan-core` before runtime issuance.

## Verification IDs touched

- `ucan-auth-integration.adapter-preserves-aspen-boundary`
- `ucan-auth-integration.sibling-source-of-truth`
- `ucan-auth-integration.capability-mapping`
- `ucan-auth-integration.explicit-policy-backends`
