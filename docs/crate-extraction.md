# Crate Extraction Readiness

Aspen stays a monorepo while extraction seams are hardened. A crate is not extraction-ready just because it is a workspace member: it must have a documented audience, owner, feature contract, dependency boundary, verification rails, and compatibility plan.

This document is the inventory and readiness contract. Detailed candidate manifests live under `docs/crate-extraction/`. Change-specific evidence lives under that change's own `openspec/changes/<change>/evidence/` directory while active, then moves with the change under `openspec/changes/archive/<date>-<change>/evidence/` when archived. Each evidence artifact is linked from the same change's `verification.md`.

## Readiness states

| State | Meaning | Allowed now |
| --- | --- | --- |
| `workspace-internal` | Workspace crate or family exists but is not proven reusable outside Aspen app assumptions. | yes |
| `extraction-ready-in-workspace` | Reusable boundary is proven inside the monorepo with standalone checks, downstream-style consumer proof, and compatibility evidence. | yes |
| `blocked-pending-license/publication-policy` | Technical rails pass, but human license/publication policy is unresolved. | yes |
| `publishable from monorepo` | Candidate can be externally published from this monorepo. | no, blocked until human license/publication decision |
| `future repository split candidate` | Candidate is ready for a later repository split. | no, blocked until human license/publication decision |

No candidate may use `publishable from monorepo` or `future repository split candidate` during `prepare-crate-extraction`.

## Canonical classes

| Class | Description | Default expectation |
| --- | --- | --- |
| `leaf type/helper` | Portable types, constants, pure helpers, or crypto helpers. | No root app, handler, binary, concrete transport, trust/secrets/SQL/coordination dependency unless that is the explicit purpose. |
| `protocol/wire` | Wire schemas, request/response enums, postcard/serde compatibility surfaces. | Stable serialization tests; minimal defaults; no runtime handler registry. |
| `storage/backend` | Embedded storage, cache, or persistence backend. | Storage dependencies allowed; app/bootstrap/network dependencies blocked by default. |
| `runtime adapter` | Concrete transport or runtime adapter around reusable contracts. | Runtime dependencies allowed only because the adapter owns them. |
| `service library` | Reusable service logic above leaf/protocol/storage layers. | Must depend on traits/contracts rather than root Aspen app bundles. |
| `binary shell` | CLI, TUI, node, web, bridge, gateway, dogfood, and test binaries. | Final consumers only; reusable behavior belongs in libraries. |

## Extraction-readiness contract

Every candidate manifest records:

- candidate name and family;
- intended external audience;
- public API owner;
- package description and documentation entrypoint;
- license and publish policy;
- repository/homepage policy;
- default features and optional features;
- public API stability or semver policy;
- internal Aspen dependencies and why each remains, moves, becomes feature-gated, or is removed;
- external dependencies and their role;
- binary/runtime dependencies, if any;
- standalone verification commands;
- positive examples and negative boundary checks;
- compatibility re-exports, dependency-key/package aliases, owner, tests, and removal plan;
- readiness state using the canonical labels above.

Before a candidate can be marked `extraction-ready-in-workspace`, evidence must prove:

1. default reusable features compile without root `aspen`, node binaries, handler bundles, dogfood, UI/TUI/web binaries, or app-runtime integration crates;
2. forbidden dependencies are absent through direct dependencies, transitive paths, representative workspace consumers, feature unification, and compatibility re-export paths;
3. positive examples use canonical new APIs;
4. negative checks prove app-only APIs stay behind named opt-in features or adapter crates;
5. downstream-style consumer proof does not depend on root package `aspen` or compatibility re-exports as the primary API;
6. Aspen compatibility consumers still compile or test through direct migration or documented temporary re-exports;
7. every checked OpenSpec task links durable evidence from `verification.md`.

## Policy and checker

Typed policy lives at `docs/crate-extraction/policy.ncl`. The deterministic checker is planned at `scripts/check-crate-extraction-readiness.rs` and is invoked as:

```bash
scripts/check-crate-extraction-readiness.rs \
  --policy docs/crate-extraction/policy.ncl \
  --inventory docs/crate-extraction.md \
  --manifest-dir docs/crate-extraction \
  --candidate-family redb-raft-kv \
  --output-json openspec/changes/<change>/evidence/dependency-boundary.json \
  --output-markdown openspec/changes/<change>/evidence/dependency-boundary.md
```

The checker must fail on unowned exceptions, missing required exception fields, forbidden readiness states before human license/publication decision, direct app-bundle dependencies, transitive app-bundle dependencies, representative-consumer leaks, and compatibility re-export leaks.

## First vertical target: Redb Raft KV

| Layer | Canonical path | Manifest | Current source | Readiness | Next action |
| --- | --- | --- | --- | --- | --- |
| KV operation/response types | `aspen-kv-types` | `docs/crate-extraction/aspen-kv-types.md` | `crates/aspen-kv-types` | `extraction-ready-in-workspace` | Publishable/repo-split blocked on license/publication policy. |
| OpenRaft app types | `aspen-raft-kv-types` | `docs/crate-extraction/aspen-raft-kv-types.md` | `crates/aspen-raft-kv-types` plus legacy `crates/aspen-raft-types` | `extraction-ready-in-workspace` | Legacy `aspen-raft-types` consumers tracked in transition table; publishable/repo-split blocked on license/publication policy. |
| Redb storage backend | `aspen-redb-storage` | `docs/crate-extraction/aspen-redb-storage.md` | `crates/aspen-redb-storage` | `extraction-ready-in-workspace` | `raft-storage` feature provides complete `RedbKvStorage`; Aspen-specific `SharedRedbStorage` stays in `aspen-raft`; publishable/repo-split blocked on license/publication policy. |
| Consensus KV facade | `aspen-raft-kv` | `docs/crate-extraction/aspen-raft-kv.md` | `crates/aspen-raft-kv` | `extraction-ready-in-workspace` | Redb-backed execution available via `aspen-redb-storage`; publishable/repo-split blocked on license/publication policy. |
| Iroh/IRPC adapter | `aspen-raft-network` | `docs/crate-extraction/aspen-raft-network.md` | `crates/aspen-raft-network` | `workspace-internal` | Transitive path through `aspen-transport`/`aspen-sharding` reaches app concerns; needs feature-gating in follow-up. |
| Aspen compatibility | `aspen-raft` compatibility paths | `docs/crate-extraction/aspen-raft-compat.md` | `crates/aspen-raft`, `crates/aspen-cluster`, handlers, binaries | `workspace-internal` | App integration shell by design; all compatibility re-exports verified (V7-V9). |

Mandatory first-slice rails:

- storage crate compile for default and Raft-storage feature sets;
- reusable KV type/facade compile without Aspen app bundles;
- iroh adapter compile only through named adapter feature or crate;
- feature-topology verification;
- direct, transitive, representative-consumer, and re-export dependency-boundary verification;
- positive downstream example using canonical new APIs;
- negative boundary checks proving app-only APIs stay behind opt-in features;
- atomic Redb log plus state-machine commit proof for moved storage path;
- crash-recovery or failure-injection proof that partial log/state commits are not observable;
- chain-integrity and snapshot-integrity tests after the move;
- CAS and lease/TTL regression tests;
- Aspen compatibility consumer proof for re-exported or migrated paths.

## Broader candidate inventory

| Family | Canonical class | Crates | Owner | Manifest | Readiness | Next action |
| --- | --- | --- | --- | --- | --- | --- |
| Foundational types/helpers | `leaf type/helper` | `aspen-constants`, `aspen-hlc`, `aspen-kv-types`, `aspen-storage-types`, `aspen-cluster-types`, `aspen-traits`, `aspen-time` | owner needed | `docs/crate-extraction/foundational-types.md` | `workspace-internal` | Keep alloc-safe defaults; split `SM_KV_TABLE` / `redb::TableDefinition`; prove `aspen-traits` transitive leaks absent. |
| Auth and tickets | `leaf type/helper` | `aspen-auth-core`, `aspen-auth`, `aspen-ticket`, `aspen-hooks-ticket` | owner needed | `docs/crate-extraction/auth-ticket.md` | `workspace-internal` | Keep runtime helpers behind explicit features. |
| Protocol/wire | `protocol/wire` | `aspen-client-api`, `aspen-forge-protocol`, `aspen-jobs-protocol`, `aspen-coordination-protocol` | owner needed | `docs/crate-extraction/protocol-wire.md` | `workspace-internal` | Preserve postcard baselines and alloc-safe serializers. |
| Redb Raft KV | `storage/backend` + `service library` + `runtime adapter` | `aspen-kv-types`, `aspen-redb-storage`, `aspen-raft-kv-types`, `aspen-raft-kv`, `aspen-raft-network`, `aspen-raft` | owner needed | per-layer manifests | 4/6 `extraction-ready-in-workspace` | Type/storage/facade layers ready; adapter/compat remain `workspace-internal`; publishable/repo-split blocked on license policy. |
| Iroh transport/RPC | `runtime adapter` | `aspen-transport`, `aspen-rpc-core`, `aspen-client` | owner needed | manifest not yet created | `workspace-internal` | Separate generic iroh RPC helpers from Aspen handler registry. |
| Coordination | `service library` | `aspen-coordination`, `aspen-coordination-protocol` | Aspen coordination maintainers | [`docs/crate-extraction/coordination.md`](crate-extraction/coordination.md) | `extraction-ready-in-workspace` | `aspen-core` removed; depends only on `aspen-kv-types`, `aspen-traits`, `aspen-constants`, `aspen-time`; protocol crate standalone; publishable/repo-split blocked on license policy. |
| Blob/castore/cache | `service library` + `runtime adapter` | `aspen-blob`, `aspen-castore`, `aspen-cache`, `aspen-exec-cache` | Aspen storage/cache maintainers | [`docs/crate-extraction/blob-castore-cache.md`](crate-extraction/blob-castore-cache.md) | `workspace-internal` | Add downstream fixtures and checker mutation evidence before marking extraction-ready. |
| Commit DAG / branches | `service library` | `aspen-commit-dag`, `aspen-kv-branch`, `aspen-dag` | owner needed | manifest not yet created | `workspace-internal` | Remove direct `aspen-raft` dependency where trait-based KV is enough. |
| Jobs and CI core | `service library` | `aspen-jobs-protocol`, `aspen-jobs`, `aspen-ci-core`, executors | owner needed | manifest not yet created | `workspace-internal` | Split schema/scheduler from cluster worker runtime. |
| Trust/crypto/secrets | `leaf type/helper` + `service library` | `aspen-trust`, `aspen-crypto`, parts of `aspen-secrets` | owner needed | manifest not yet created | `workspace-internal` | Keep pure crypto separate from transport/client secrets shell. |
| Config/plugin | `protocol/wire` | `aspen-nickel`, `aspen-plugin-api` | owner needed | manifest not yet created | `workspace-internal` | Document standalone examples and feature minima. |
| Testing harness | `service library` | `aspen-testing-core`, fixtures, madsim/network/patchbay crates | owner needed | manifest not yet created | `workspace-internal` | Split generic harness helpers from Aspen cluster boot helpers. |
| Binary shells | `binary shell` | `aspen-cli`, `aspen-tui`, `aspen-node`, bridges, gateways, web | owner needed | manifest not yet created | `workspace-internal` | Keep as final consumers, not reusable dependency roots. |

## Evidence discipline

Every checked task must appear verbatim in its change's `verification.md` with an `- Evidence:` line. Evidence must be repo-relative, checked in, and under that change directory unless it is a changed implementation file. Do not cite `/tmp` or chat-only summaries.
