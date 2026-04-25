## Why

Aspen already contains several reusable libraries, but the reusable seams are still hard to consume because some crates carry Aspen application identity, runtime transport, or binary integration dependencies. Preparing explicit extraction boundaries lets Aspen keep dogfooding its full platform while making core pieces reusable by other Rust projects.

## What Changes

- Define a reusable-crate extraction contract: dependency hygiene, public API ownership, feature topology, docs, tests, and release metadata.
- Make the Redb + OpenRaft KV stack the first vertical extraction target, with storage, consensus, transport, and Aspen-node integration split into separate layers.
- Inventory other high-value extraction candidates and classify each as a leaf library, protocol library, runtime adapter, service library, or binary shell, covering foundational type crates, auth/ticket crates, transport/RPC crates, coordination primitives, blob/castore/cache crates, commit DAG/KV branch crates, CI core/executor crates, jobs protocol/executor crates, trust/crypto crates, plugin API, Nickel config support, testing harness crates, binaries, and explicitly `crates/aspen-storage-types`, `crates/aspen-client-api`, `crates/aspen-cluster-types`, and `crates/aspen-traits`.
- Add verification rails so a crate is not considered extractable until it builds and tests outside Aspen's app bundle assumptions.
- Keep Aspen's existing workspace and dogfood binaries working through compatibility re-exports while boundaries are prepared.

First Redb Raft KV layer map:

- **KV types**: current `crates/aspen-kv-types` remains the reusable KV operation/response type crate.
- **Raft types**: current `crates/aspen-raft-types` becomes new reusable `aspen-raft-kv-types`.
- **storage**: current `crates/aspen-redb-storage` absorbs reusable Redb modules now under `crates/aspen-raft/src/storage*`.
- **consensus node facade**: new reusable `aspen-raft-kv` owns generic replicated KV node APIs.
- **transport adapter**: current `crates/aspen-raft-network` remains the explicit iroh/IRPC adapter.
- **Aspen integration**: current `crates/aspen-raft`, `crates/aspen-cluster`, `crates/aspen-core-shell`, root `aspen-node`, handlers, and dogfood binaries stay as app/runtime shells over those libraries.

## Non-goals

- Do not split Aspen into separate Git repositories in this change.
- Do not publish crates externally in this change.
- Do not redesign unrelated Aspen subsystems while preparing extraction seams.
- Do not add non-iroh networking to Aspen's runtime; reusable layers may be transport-neutral, but Aspen node communication remains iroh-only.
- Do not strand existing Aspen consumers during import-path migration; either migrate all affected callers directly or keep temporary compatibility re-exports with a documented removal plan.

## Completion state

This change should end with extraction contracts, a checked-in inventory, typed dependency policy, six canonical Redb Raft KV manifests (`docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, and `docs/crate-extraction/aspen-raft-compat.md`), three broader family manifest stubs (`docs/crate-extraction/foundational-types.md`, `docs/crate-extraction/auth-ticket.md`, and `docs/crate-extraction/protocol-wire.md`), service/runtime inventory follow-up entries that may explicitly say `manifest not yet created`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/verification.md`, and workspace-internal extraction readiness evidence for the first Redb Raft KV slice. It does not mark any crate publishable or repository-split-ready until license and publication policy are decided.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `architecture-modularity`: Add requirements for publishable reusable crate boundaries, reusable Redb + Raft KV layering, binary/library separation, and extraction-candidate inventory.

## Impact

- **Code**: Primarily `crates/aspen-raft`, `crates/aspen-redb-storage`, `crates/aspen-raft-types`, `crates/aspen-raft-network`, `crates/aspen-core-shell`, `crates/aspen-traits`, root `src/bin/aspen_node`, and candidate crates called out by the inventory.
- **APIs**: New or renamed public facades may be introduced for reusable crates; existing Aspen paths should keep temporary re-exports during migration.
- **Dependencies**: Reusable library defaults should avoid Aspen app bundles, trust/secrets/SQL/coordination, binary-only code, and concrete iroh transport unless explicitly enabled.
- **Testing**: Each extraction target needs standalone compile/test rails, isolated-consumer proof outside Aspen app bundles, dependency graph assertions that include transitive/re-export leak checks, positive usage examples, and negative tests proving app-only APIs stay behind opt-in features.

## Task Plan

- **Baseline**: Capture workspace inventory and Redb Raft KV coupling evidence before moving code.
- **Contract and gates**: Add `docs/crate-extraction.md`, per-layer Redb Raft KV manifests, typed Nickel policy, and deterministic readiness checker.
- **First vertical slice**: Create reusable KV/Raft type, storage, facade, and iroh-adapter boundaries while retaining Aspen compatibility re-exports.
- **Broader inventory**: Add family manifest stubs for foundational, auth/ticket, and protocol/wire crates, then record service/runtime follow-ups.
- **Verification**: Save compile, feature-topology, dependency-boundary, downstream-consumer, Redb atomicity, positive/negative test, release-readiness, and compatibility evidence before checking tasks complete.

## Verification Rails

Initial evidence paths and commands are part of the implementation checklist, but durable traceability lives in `openspec/changes/archive/2026-04-25-prepare-crate-extraction/verification.md`. No implementation or verification item can be accepted until `verification.md` links the related evidence artifact.

- `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/extraction-inventory-baseline.md`: `cargo metadata --format-version 1`-derived crate inventory and classification baseline.
- `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/redb-raft-kv-coupling-baseline.md`: source/module map plus `cargo tree` evidence for `aspen-redb-storage`, `aspen-raft-types`, `aspen-raft`, and `aspen-raft-network`.
- `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/feature-matrix.md`: standalone `cargo check` matrix for `aspen-kv-types`, `aspen-redb-storage`, `aspen-raft-kv-types`, `aspen-raft-kv`, `aspen-raft-network`, and Aspen compatibility consumers across default and named reusable features.
- `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.json` and `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.md`: readiness-checker output proving reusable defaults do not pull app bundles, handlers, dogfood, UI, trust, secrets, SQL, coordination, or forbidden defaults except through documented opt-in features.
- `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/downstream-consumer-metadata.json`: `cargo metadata --manifest-path <consumer>/Cargo.toml --format-version 1` for a checked-in repo-local consumer or explicitly wired baseline clone/worktree that does not depend on root package `aspen` or compatibility re-exports as its primary API.
- `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/redb-atomicity.md`: post-move Redb single-transaction log/state proof plus crash-recovery, failure-injection, or madsim evidence.
- `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-*.md`: Aspen node, cluster, CLI, dogfood, handler, bridge, gateway, web, and TUI compatibility compile/test transcripts.

Readiness checker invocation:

```bash
scripts/check-crate-extraction-readiness.rs \
  --policy docs/crate-extraction/policy.ncl \
  --inventory docs/crate-extraction.md \
  --manifest-dir docs/crate-extraction \
  --candidate-family redb-raft-kv \
  --output-json openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.json \
  --output-markdown openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.md
```

The checker reads `docs/crate-extraction/policy.ncl`, the inventory, and candidate manifests; evaluates direct, transitive, representative-consumer, and re-export dependency paths; rejects incomplete exception metadata and forbidden readiness states; exits non-zero on boundary failure; and writes deterministic JSON plus markdown summaries for review.
