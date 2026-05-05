# Full Aspen Hardening Threat Model

Status: captured (Phase 2 seed threat model).

## Actors

- **Unauthenticated network caller:** can reach public Iroh client/federation entrypoints and malformed protocol inputs.
- **Authenticated delegated client:** holds a scoped token, possibly key-bound to an Iroh identity.
- **Cluster/operator administrator:** can bootstrap clusters and create root/bootstrap/delegated tokens; mistakes here can leak bearer authority.
- **Federated peer/proxy caller:** can request cross-cluster operations and may present stale, generic, or overbroad bearer material.
- **Worker/job payload author:** can submit CI, shell, VM, or build jobs and attempt command/sandbox escape.
- **Compromised or buggy node/component:** can emit logs, receipts, errors, or metrics containing sensitive material.
- **Supply-chain attacker:** can target flake inputs, vendored deps, build scripts, fallback subprocesses, or cache artifacts.

## Assets

- Capability tokens, root/bootstrap tokens, federation credentials, and presenter identity bindings.
- Cluster tickets, Iroh secret keys, SOPS/trust shares, and secret payloads.
- Raft-backed state, reserved KV prefixes, SNIX DirectoryService/PathInfoService content, Forge repos, CI/job/deploy state.
- Dogfood/CI/deploy receipts, logs, metrics, and operator diagnostics.
- Build artifacts, Nix/SNIX cache entries, flake pins, vendored dependencies, and VM/shell execution environments.

## Trust boundaries and controls

| Boundary | Primary abuse case | Expected control | Phase 2 inventory handle | Later evidence gate |
| --- | --- | --- | --- | --- |
| Client RPC decode -> auth gate -> handler dispatch | Invoke protected handler before auth or with stale public exemption | `to_operation` classification, auth-before-dispatch, presenter binding | `audit-inventory.md` auth/request-classification | Phase 3 source-order + negative tests |
| Batch/conditional KV -> reserved prefixes | Disclose/mutate reserved SNIX/secrets/admin state through generic operations | all protected keys represented or fail-closed admin/domain op | storage/reserved-prefixes | Phase 3 reserved-prefix fixtures |
| Proxy/federation forwarding | Forward key-bound/generic/root token or strip verified delegated token | proxy eligibility policy and token preservation | dispatch/proxy + federation rows | Phase 3 proxy/federation negative tests |
| Token/ticket filesystem | Leak local bearer/ticket/private-key material through output or weak perms | owner-only writes; path/length/hash only in logs | tokens/tickets/filesystem row | Phase 4 permission/redaction checks |
| Receipts/logs/errors/metrics | Durable credential leakage in evidence | synthetic redaction fixtures; no real-secret reads | dogfood/operator-evidence row | Phase 4 redaction regressions |
| Jobs/CI/build execution | command escape, missing ShellExecute cap, VM ticket leakage | command caps, sandbox/isolation, ticket redaction | jobs/ci/execution row | Phase 5 sandbox evidence |
| SNIX/cache/build supply chain | generic auth to store mutation or unpinned artifact trust | SnixRead/SnixWrite, pinned inputs, artifact IDs | snix/supply-chain + nix/build rows | Phase 5 store/cache/build checks |
| Network/transport | HTTP/raw transport bypass of Iroh/Raft/auth control plane | Iroh ALPN/Raft for protected control plane; documented data-plane exceptions | network/transport row | Phase 5 transport audit |

## Audit acceptance rules

1. Public endpoints are accepted only with explicit source handles and rationale.
2. Protected endpoints are accepted only with a matrix row, expected `Operation`, capability shape, and negative bypass evidence or a tracked finding.
3. Credential-bearing surfaces are accepted only with synthetic redaction evidence; real secret files are never read for value.
4. Execution and build boundaries are accepted only with command/sandbox/artifact identity evidence.
5. High-risk gaps become focused child OpenSpecs or direct verified remediation commits before umbrella archive.
