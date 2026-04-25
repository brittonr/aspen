# Redb Raft KV Layer Review

Reviewed before any Redb storage code movement or compatibility migration.

## Proposed split

| Layer | Canonical path | Manifest | Decision |
| --- | --- | --- | --- |
| KV operation/response types | `aspen-kv-types` | `docs/crate-extraction/aspen-kv-types.md` | Keep current crate. |
| OpenRaft app types | `aspen-raft-kv-types` | `docs/crate-extraction/aspen-raft-kv-types.md` | Split/rename from current `aspen-raft-types`; remove default Aspen core/trust coupling. |
| Redb storage backend | `aspen-redb-storage` | `docs/crate-extraction/aspen-redb-storage.md` | Move concrete Redb/OpenRaft storage from `aspen-raft` behind `raft-storage`. |
| Consensus KV facade | `aspen-raft-kv` | `docs/crate-extraction/aspen-raft-kv.md` | New reusable facade without Aspen binary configuration. |
| Iroh/IRPC adapter | `aspen-raft-network` | `docs/crate-extraction/aspen-raft-network.md` | Keep as explicit adapter. |
| Aspen compatibility | `aspen-raft` | `docs/crate-extraction/aspen-raft-compat.md` | Keep as app/runtime shell and compatibility bridge. |

## OpenRaft boundary review

- `openraft` remains vendored at Aspen's pinned 0.10 dependency for this slice.
- `aspen-raft-kv-types`, `aspen-redb-storage` with `raft-storage`, and `aspen-raft-kv` must treat OpenRaft trait/type exposure as public or manifest-tracked implementation API.
- `openraft` exposure must not require root `aspen`, Aspen bootstrap, handler registries, dogfood defaults, trust, secrets, SQL, coordination, or concrete iroh endpoint construction by default.
- Upstream OpenRaft compatibility beyond the vendored version is deferred.

## Deviation status

No deviation from the approved proposal/design split is required before implementation. If implementation later needs consolidation, package renaming, or a different OpenRaft exposure model, the design and manifests must be updated with owner, rationale, and revised verification rails before code moves.
