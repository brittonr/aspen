## Context

Aspen is already organized as a large workspace with leaf crates, runtime crates, feature-gated handlers, and separate binaries. Recent no-std and alloc-safe work created useful seams (`aspen-core`, `aspen-core-shell`, `aspen-auth-core`, `aspen-hooks-ticket`, protocol crates, and storage-type cleanup). The remaining problem is extraction readiness: a crate can exist as a workspace member but still be hard to reuse because its default dependencies pull Aspen's app bundle, concrete iroh runtime setup, root binary integration, transitive default features, or Aspen-specific names.

The Redb + Raft KV path is the first vertical extraction target because it is independently valuable: a reusable embedded Redb state machine plus OpenRaft consensus KV library with optional iroh transport. Today the pieces are close but not cleanly layered:

- `crates/aspen-redb-storage` contains verified pure storage helpers, but its comment notes the concrete storage modules remain in `crates/aspen-raft` because they depend on Raft-specific types.
- `crates/aspen-raft` mixes reusable consensus/KV logic with Aspen auth, client API, transport, sharding, optional coordination/SQL/trust/secrets, iroh network implementation, node watchers, and app integration.
- `crates/aspen-raft-types` still depends on `aspen-core` and `aspen-trust`, which makes it less reusable as a neutral OpenRaft app type crate.
- `crates/aspen-raft-network` is a useful iroh/IRPC adapter, but should be optional relative to storage and consensus contracts.

## Goals / Non-Goals

**Goals:**

- Define extraction-readiness criteria before moving repositories or publishing packages.
- Make Redb + Raft KV the first proof target and split it into stable layers.
- Identify other crate families with strong reuse potential.
- Keep Aspen's dogfood node and existing consumer imports working while internals move.
- Add durable verification rails so readiness is measurable, not aspirational.

**Non-Goals:**

- Do not immediately create separate Git repositories.
- Do not publish crates as part of this preparation change.
- Do not change Aspen runtime networking policy; Aspen still uses iroh-only runtime networking.
- Do not support arbitrary non-iroh transports in Aspen. Reusable layers may be transport-neutral; shipped Aspen adapters remain iroh-based.
- Do not preserve compatibility re-exports forever; they require exit criteria.

## Decisions

### 1. Prepare extraction inside the workspace first

**Choice:** Treat extraction as a workspace-internal seam hardening effort before repository split or publication.

**Rationale:** This keeps feedback fast, avoids versioning churn, and lets existing dogfood tests prove Aspen still works. A crate is not extraction-ready until it has standalone checks from the workspace root and a downstream-style consumer proof that does not depend on root `aspen`, handler bundles, dogfood crates, binaries, or compatibility re-exports as the primary API. That downstream proof must be either a checked-in repo-local consumer fixture or a baseline clone/worktree with sibling path dependencies explicitly wired. The guard is a saved `cargo metadata --manifest-path <consumer>/Cargo.toml --format-version 1` artifact plus a deterministic assertion that the consumer does not depend on package `aspen`, that each candidate package `manifest_path` resolves under the intended checkout, and that no dependency path resolves through a sibling live Aspen checkout.

**Alternative:** Split repositories first. Rejected because dependency cycles and app coupling would become harder to fix after path dependencies are replaced with published versions.

### 2. Use per-candidate extraction manifests

**Choice:** Store the inventory summary in `docs/crate-extraction.md` and store detailed per-candidate manifests under `docs/crate-extraction/<crate-or-family>.md` when a candidate moves beyond rough inventory.

Each manifest records:

- candidate name and family;
- intended external audience;
- public API owner;
- package description and documentation entrypoint;
- license and publish policy;
- repository/homepage policy;
- default features and optional features;
- public API stability or semver policy;
- internal Aspen dependencies and why each remains, moves, or becomes feature-gated;
- external dependencies and their role;
- binary/runtime dependencies, if any;
- standalone verification commands;
- positive examples and negative boundary checks;
- compatibility re-exports and removal plan;
- readiness state using the canonical labels: `workspace-internal`, `extraction-ready-in-workspace`, `blocked-pending-license/publication-policy`, `publishable from monorepo`, or `future repository split candidate`.

Until license and publication policy are resolved, no candidate may be marked `publishable from monorepo` or `future repository split candidate`; those labels are invalid for this change. Any task attempting to mark a candidate publishable or repo-split-ready must stop for human decision on licensing and publication policy.

All evidence generated for a manifest or readiness claim must be saved under `openspec/changes/prepare-crate-extraction/evidence/` and linked from `openspec/changes/prepare-crate-extraction/verification.md` before the related task can be checked complete.

**Rationale:** A table alone is not enough once code starts moving. The manifest is the durable place where API ownership, release metadata, and dependency decisions are reviewed.

**Alternative:** Encode all fields in Cargo manifests only. Rejected because Cargo metadata cannot express dependency-removal plans, API owner, downstream proof model, or temporary re-export exit criteria.

### 3. Use a crate-family model for Redb + Raft KV

**Choice:** Prepare the reusable KV stack as a family of crates rather than one large catch-all crate:

- `aspen-kv-types` for reusable KV command and response types.
- `aspen-raft-kv-types` as the new reusable OpenRaft `TypeConfig`, membership metadata, app data, and error crate without Aspen app dependencies.
- `aspen-redb-storage` for Redb-backed log/state-machine storage, snapshots, chain integrity, leases, CAS, and single-transaction storage behavior.
- `aspen-raft-kv` as the reusable consensus KV node facade and `KeyValueStore` / `ClusterController` implementation crate.
- `aspen-raft-network` as the explicit iroh/IRPC adapter crate for the reusable KV stack.
- Existing `aspen-raft` remains the Aspen compatibility and integration crate until callers migrate.

Initial module/file ownership map:

| Current source | Target layer | Target crate/path |
| --- | --- | --- |
| `crates/aspen-kv-types` | KV operation/response types | `aspen-kv-types` |
| `crates/aspen-raft-types` reusable OpenRaft config/app data | Raft app types | `aspen-raft-kv-types` |
| `crates/aspen-raft/src/storage_shared/*` | Redb shared log/state-machine storage | `aspen-redb-storage` |
| `crates/aspen-raft/src/storage/redb_store.rs` | Redb storage adapter | `aspen-redb-storage` |
| `crates/aspen-raft/src/storage_validation.rs` | Redb storage validation | `aspen-redb-storage` |
| `crates/aspen-raft/src/integrity.rs` and `crates/aspen-raft/src/verified/*` storage helpers | Verified storage/integrity helpers | `aspen-redb-storage` where storage-specific, otherwise `aspen-raft-kv-types`/`aspen-raft-kv` |
| `crates/aspen-raft/src/node/*` reusable KV paths | Consensus node facade | `aspen-raft-kv` |
| `crates/aspen-raft/src/network/*` and `crates/aspen-raft-network` | Iroh/IRPC adapter | `aspen-raft-network` |
| `crates/aspen-raft` Aspen bootstrap/watchers/trust/secrets/SQL/coordination wiring | Aspen integration/compatibility | `aspen-raft`, `aspen-cluster`, handlers, binaries |

Initial compatibility path targets:

- `aspen_raft::types::*` → `aspen_raft_kv_types::*` for reusable app types.
- `aspen_raft::storage_shared::*` and `aspen_raft::storage::redb_store::*` → `aspen_redb_storage::*` for Redb storage types.
- `aspen_raft::node::RaftNode` reusable KV construction paths → `aspen_raft_kv::*`; Aspen-specific bootstrap wrappers stay in `aspen_raft` / `aspen_cluster`.
- `aspen_raft::network::*` iroh adapter paths → `aspen_raft_network::*` where the adapter is reusable, with Aspen-specific wiring staying in integration crates.

**Rationale:** External users should be able to choose storage only, consensus without iroh, or full iroh transport. Aspen can then compose the same layers with trust/secrets/SQL/coordination features.

**Alternative:** Publish current `aspen-raft` as-is. Rejected because default users would inherit too much Aspen-specific surface area and feature coupling.

### 4. Resolve first-slice boundary choices now

**Choice:** For the first extraction slice, keep Aspen's vendored `openraft` dependency and expose Redb storage primarily through OpenRaft storage traits plus the reusable KV facade. `openraft` is not hidden as an Aspen app detail for the Raft layers: `aspen-raft-kv-types`, `aspen-redb-storage`, and `aspen-raft-kv` must mark any public OpenRaft trait/type exposure in their manifests. `openraft` also must not become a covert path for root `aspen`, handler bundles, trust/secrets/SQL/coordination, or concrete iroh transport defaults. Do not add a separate lower-level embedded KV API in this change.

**Rationale:** The first goal is a clean reusable replicated KV stack without destabilizing consensus dependency management. Supporting upstream OpenRaft compatibility or a lower-level embedded KV API can be future work after the storage and facade seams are proven. Making OpenRaft exposure explicit avoids the ambiguous middle ground where external consumers accidentally rely on Aspen-specific `TypeConfig` aliases without knowing whether they are stable public API.

**Alternative:** Require upstream OpenRaft compatibility first. Rejected for this change because it expands scope into dependency upgrade policy. Alternative lower-level Redb KV API is deferred because it would need its own API design and tests.

The Redb Raft KV family uses exactly one manifest file per target layer for this change:

- `docs/crate-extraction/aspen-kv-types.md`
- `docs/crate-extraction/aspen-raft-kv-types.md`
- `docs/crate-extraction/aspen-redb-storage.md`
- `docs/crate-extraction/aspen-raft-kv.md`
- `docs/crate-extraction/aspen-raft-network.md`
- `docs/crate-extraction/aspen-raft-compat.md`

Each manifest records owner, canonical crate or compatibility path, readiness state, dependency policy class, representative consumers/re-exporters, and verification rails. Layer consolidation is out of scope for this change; any future consolidation requires a design update before implementation starts.

The Redb Raft KV manifests must name these mandatory first-slice rails before any layer can be marked ready:

- storage crate compile for default and Raft-storage feature sets;
- reusable KV types/facade compile without Aspen app bundles;
- iroh adapter compile only through its named adapter feature or crate;
- feature-topology verification for default and named reusable features;
- dependency-boundary verification, including direct, transitive, representative workspace-consumer, and re-export leak checks;
- positive downstream example using canonical new APIs;
- negative boundary checks proving app-only APIs stay behind opt-in features;
- atomic Redb log plus state-machine commit proof for the moved storage path;
- crash-recovery or failure-injection proof that partial log/state commits are not observable;
- chain-integrity and snapshot-integrity tests after the move;
- CAS and lease/TTL regression tests;
- downstream-style consumer proof using canonical new APIs, not compatibility re-exports;
- Aspen compatibility consumer proof for re-exported or migrated paths.

### 5. Keep functional core plus imperative shell as the split rule

**Choice:** Reusable crates own deterministic logic, types, protocol encoding, storage algorithms, and traits. Binary/runtime crates own CLI parsing, node bootstrap, task spawning, filesystem layout, environment variables, logging setup, and external services.

**Rationale:** This matches Aspen's verified modules, madsim goals, and Tiger Style rule that logic should be testable without standing up the world.

**Alternative:** Split by directory ownership only. Rejected because directory moves without API/dependency cleanup do not improve reuse.

### 6. Verify transitive and re-export dependency leaks, not just direct deps

**Choice:** Extraction gates must inspect candidate crates, their claimed reusable feature sets, and representative workspace consumers/re-exporters. The checker reads a typed Nickel policy at `docs/crate-extraction/policy.ncl` that defines candidate categories, forbidden crate classes, allowed dependency categories, feature-gated exception rules, feature sets to test, representative workspace consumers per candidate/family, and readiness-state restrictions.

Initial policy classes:

- **forbidden by default for reusable library candidates**: root `aspen`, node binaries, handler bundles, dogfood crates, UI/TUI/web binaries, bridge/gateway binaries, and app-runtime integration crates.
- **forbidden unless the candidate manifest explicitly marks them as the candidate purpose or named feature**: trust, secrets, SQL, coordination, concrete iroh transport, RPC handler registry, sharding, proxy, and Aspen cluster bootstrap.
- **allowed with manifest explanation**: leaf type crates, protocol crates, pure verified helpers, OpenRaft, Redb, serialization, metrics, tracing, and runtime crates required by the candidate's explicit layer.
- **representative consumer rule**: every candidate/family must list workspace consumers/re-exporters that can unify features and re-enable defaults.
- **readiness-state rule**: `publishable from monorepo` and `future repository split candidate` are invalid until license and publication policy are resolved.
- **exception rule**: every exception must name the candidate, feature set, dependency path, owner, and reason in the manifest; unowned exceptions fail readiness.

The checker must fail if forbidden app/runtime crates are reachable through feature unification, transitive defaults, or compatibility re-export paths.

**Rationale:** Prior no-std work showed that disabling defaults on a leaf crate is insufficient when another dependency re-enables them. Extraction readiness needs the same style of inverse `cargo tree` proofs and source audits used by the alloc-safe boundary work.

**Alternative:** Only inspect direct dependencies in the candidate manifest. Rejected because it misses the known default-feature leak mode.

### 7. Track compatibility re-exports with exit criteria

**Choice:** Every compatibility re-export introduced by extraction must be listed in the candidate manifest with old path, new path, reason, tests covering both paths, migration owner, and removal criterion. When preserving import paths is more important than preserving package names, use dependency-key/package aliasing (for example the existing `aspen-core = { package = "aspen-core-shell", ... }` pattern) and record that alias in the manifest alongside any re-exports. A re-export is removable after all in-repo consumers use the new API and the reusable downstream example no longer imports through the compatibility path.

**Rationale:** Temporary aliases help Aspen keep moving, but unmanaged aliases become permanent public API clutter.

**Alternative:** Leave re-exports undocumented and remove later opportunistically. Rejected because reviewers need to know which public paths are canonical.

### 8. Inventory candidates by family and readiness

**Choice:** Create a checked-in extraction inventory, then promote candidates from easiest to hardest. The inventory in `docs/crate-extraction.md` must record, for every row, candidate family, canonical class (`leaf type/helper`, `protocol/wire`, `storage/backend`, `runtime adapter`, `service library`, or `binary shell`), crates, owner or `owner needed`, manifest link or `manifest not yet created`, readiness state, and next action.

Initial candidate map:

| Family | Canonical class | Crates | Owner | Manifest | Readiness | Next action |
| --- | --- | --- | --- | --- | --- | --- |
| Foundational types/helpers | leaf type/helper | `aspen-constants`, `aspen-hlc`, `aspen-kv-types`, `aspen-storage-types`, `aspen-cluster-types`, `aspen-traits`, `aspen-time` | owner needed | manifest not yet created | workspace-internal | Keep alloc-safe defaults, document semver policy, and track `aspen-storage-types` `SM_KV_TABLE` / `redb::TableDefinition` cleanup plus `aspen-traits` transitive default-feature leak checks |
| Auth and tickets | leaf type/helper | `aspen-auth-core`, `aspen-auth`, `aspen-ticket`, `aspen-hooks-ticket` | owner needed | manifest not yet created | workspace-internal | Keep runtime helpers behind explicit features |
| Protocol/wire | protocol/wire | `aspen-client-api`, `aspen-forge-protocol`, `aspen-jobs-protocol`, `aspen-coordination-protocol` | owner needed | manifest not yet created | workspace-internal | Preserve postcard baselines and alloc-safe serializers |
| Redb Raft KV | storage/backend + service library + runtime adapter | `aspen-kv-types`, `aspen-redb-storage`, `aspen-raft-kv-types`, `aspen-raft-kv`, `aspen-raft-network`, `aspen-raft` | owner needed | per-layer manifests under `docs/crate-extraction/aspen-*.md` | workspace-internal | Move concrete Redb storage out of `aspen-raft` and hide Aspen app features behind adapters |
| Iroh transport/RPC | runtime adapter | `aspen-transport`, `aspen-rpc-core`, `aspen-client` | owner needed | manifest not yet created | workspace-internal | Separate generic iroh RPC helpers from Aspen handler registry |
| Coordination | service library | `aspen-coordination`, protocol crate | owner needed | manifest not yet created | workspace-internal | Ensure it depends only on reusable KV traits/types plus time injection |
| Blob/castore/cache | service library | `aspen-blob`, `aspen-castore`, `aspen-cache`, `aspen-exec-cache` | owner needed | manifest not yet created | workspace-internal | Separate Aspen client integration from storage/cache core APIs |
| Commit DAG / branches | service library | `aspen-commit-dag`, `aspen-kv-branch`, `aspen-dag` | owner needed | manifest not yet created | workspace-internal | Remove direct `aspen-raft` dependency where trait-based KV is enough |
| Jobs and CI core | service library | `aspen-jobs-protocol`, `aspen-jobs`, `aspen-ci-core`, executors | owner needed | manifest not yet created | workspace-internal | Split core schema/scheduler from Aspen cluster worker runtime |
| Trust/crypto/secrets | leaf type/helper + service library | `aspen-trust`, `aspen-crypto`, parts of `aspen-secrets` | owner needed | manifest not yet created | workspace-internal | Keep pure crypto separate from Aspen client/transport secrets shell |
| Config/plugin | protocol/wire | `aspen-nickel`, `aspen-plugin-api` | owner needed | manifest not yet created | workspace-internal | Document standalone examples and feature minima |
| Testing harness | service library | `aspen-testing-core`, fixtures, madsim/network/patchbay crates | owner needed | manifest not yet created | workspace-internal | Split generic harness helpers from Aspen cluster boot helpers |
| Binary shells | binary shell | `aspen-cli`, `aspen-tui`, `aspen-node`, bridges/gateways/web | owner needed | manifest not yet created | workspace-internal | Keep them as final consumers, not reusable dependency roots |

## Risks / Trade-offs

**API churn** → Mitigate with compatibility re-exports and manifest-tracked removal criteria, but prefer clean APIs over permanent legacy paths.

**Feature graph regressions** → Mitigate with deterministic dependency-boundary scripts, inverse dependency proofs, representative consumer checks, and saved evidence for each candidate.

**Over-splitting** → Mitigate by extracting crate families around real reuse stories. Do not split crates that only make sense inside Aspen.

**Transport coupling** → Mitigate by keeping iroh adapters explicit while preserving Aspen's iroh-only runtime rule for node communication.

**Storage safety regressions** → Mitigate by carrying forward the single-fsync atomic log+state proof, chain-integrity, snapshot-integrity, crash-recovery, and madsim rails during the Redb storage move.

**Licensing/publishing friction** → Mitigate by recording license and release-readiness fields in candidate manifests before external publication rather than during repository splitting.

## Migration Plan

1. Inventory current workspace crates, direct Aspen dependencies, default features, binaries, and public APIs.
2. Add `docs/crate-extraction.md` plus detailed manifests for the Redb Raft KV family and any candidate marked ready.
3. Add extraction gates for direct/transitive/re-export dependency leaks and downstream-style consumer proof.
4. Harden leaf and protocol crates first so the reusable stack has clean dependencies.
5. Move concrete Redb storage modules from `aspen-raft` into `aspen-redb-storage` behind an explicit Raft storage feature.
6. Introduce a reusable Redb Raft KV facade and migrate Aspen integration code to consume it.
7. Split or feature-gate iroh transport as an adapter layer.
8. Convert root binaries into thin consumers of reusable library APIs.
9. Only after gates pass, decide whether to publish crates, split repos, or keep a monorepo with publishable workspace packages.

Rollback is straightforward during preparation: keep old `aspen-raft` compatibility re-exports until the new facade is proven, then migrate callers incrementally. If storage movement regresses atomicity or crash-recovery proof, revert `aspen-raft` to the previous storage module path and keep only manifest/checker work.

## Open Questions

- License strategy: keep AGPL for all crates, dual-license selected reusable libraries, or separate policy by crate family?
- Publication strategy: publish from the monorepo first, split repositories later, or keep all crates monorepo-owned indefinitely?
