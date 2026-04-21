# Verification Evidence

This file is the claim-to-artifact index for `no-std-aspen-core`.
Durable evidence lives under `openspec/changes/no-std-aspen-core/evidence/`.
No tasks are checked yet; this file defines the evidence plan that implementation must fill in.

## Evidence Naming Convention

- compile slices: `evidence/compile-<slice>.txt`
- default feature proof: `evidence/core-default-features.txt`
- smoke consumer proof: `evidence/smoke-manifest.txt`, `evidence/smoke-source.txt`, `evidence/compile-smoke.txt`
- representative consumer feature proofs: `evidence/cluster-core-features.txt`, `evidence/cli-core-features.txt`, `evidence/feature-claims.json`
- dependency audits: `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json`, `evidence/deps-allowlist-diff.txt`, `evidence/deps-transitive-review-<crate>.md`
- boundary inventories and source audits: `evidence/baseline-surface-inventory.md`, `evidence/surface-inventory.md`, `evidence/export-map.md`, `evidence/source-audit.txt`
- compile-fail artifacts: `evidence/compile-ui.txt`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr`
- regression artifacts: `evidence/regression-<topic>.txt`
- docs review notes: saved under `evidence/` with a `docs-*.md` or `docs-*.txt` name

## Task Coverage

No tasks are checked yet.
When a task is checked in `tasks.md`, copy its text here verbatim and cite the exact evidence paths.

## Scenario Coverage

| Scenario | Planned evidence |
| --- | --- |
| `specs/core/spec.md` → `Bare dependency uses alloc-only default` | `evidence/core-default-features.txt`, `evidence/feature-claims.json` |
| `specs/core/spec.md` → `Alloc-only build succeeds` | `evidence/compile-no-default.txt`, `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json` |
| `specs/core/spec.md` → `Bare-default downstream consumer remains supported` | `evidence/smoke-manifest.txt`, `evidence/smoke-source.txt`, `evidence/compile-smoke.txt`, `evidence/feature-claims.json` |
| `specs/core/spec.md` → `Alloc-only build rejects shell imports` | `evidence/compile-ui.txt`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr` |
| `specs/core/spec.md` → `Compile-fail verification is reviewable` | `evidence/compile-ui.txt`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr` |
| `specs/core/spec.md` → `Std-dependent helpers require explicit opt-in` | `evidence/compile-std.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt`, `evidence/export-map.md` |
| `specs/core/spec.md` → `Std-gated shell APIs keep current public paths` | `evidence/export-map.md`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr` |
| `specs/core/spec.md` → `Module-family boundary matches documented inventory` | `evidence/baseline-surface-inventory.md`, `evidence/surface-inventory.md`, `evidence/export-map.md` |
| `specs/core/spec.md` → `Representative std consumers remain supported` | `evidence/compile-cluster.txt`, `evidence/cluster-core-features.txt`, `evidence/compile-cli.txt`, `evidence/cli-core-features.txt` |
| `specs/core/spec.md` → `Compile-slice verification is reviewable` | `evidence/compile-default.txt`, `evidence/compile-no-default.txt`, `evidence/compile-sql.txt`, `evidence/compile-std.txt`, `evidence/compile-std-sql.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt`, `evidence/compile-smoke.txt`, `evidence/compile-cluster.txt`, `evidence/compile-cli.txt` |
| `specs/core/spec.md` → `Verified function purity` | `evidence/source-audit.txt`, `evidence/regression-<topic>.txt` |
| `specs/core/spec.md` → `Shell APIs do not leak into alloc-only core` | `evidence/export-map.md`, `evidence/source-audit.txt` |
| `specs/core/spec.md` → `Pure time-dependent logic uses no-std-safe inputs` | `evidence/source-audit.txt`, `evidence/regression-<topic>.txt` |
| `specs/core/spec.md` → `Pure logic accepts explicit randomness and configuration inputs` | `evidence/source-audit.txt`, `evidence/regression-<topic>.txt` |
| `specs/core/spec.md` → `Refactored pure logic keeps regression coverage` | `evidence/regression-<topic>.txt` |
| `specs/architecture-modularity/spec.md` → `Runtime shells depend outward on core` | `evidence/surface-inventory.md`, `evidence/export-map.md`, `evidence/source-audit.txt` |
| `specs/architecture-modularity/spec.md` → `Acyclic boundary proof is reviewable` | `evidence/surface-inventory.md`, `evidence/export-map.md`, `evidence/source-audit.txt` |
| `specs/architecture-modularity/spec.md` → `Pure consumers avoid runtime shells` | `evidence/compile-no-default.txt`, `evidence/compile-smoke.txt`, `evidence/feature-claims.json` |
| `specs/architecture-modularity/spec.md` → `Alloc-only core excludes runtime shells` | `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json`, `evidence/deps-allowlist-diff.txt`, `evidence/deps-transitive-review-<crate>.md` |
| `specs/architecture-modularity/spec.md` → `Std compatibility is an explicit opt-in` | `evidence/core-default-features.txt`, `evidence/compile-std.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt`, `evidence/compile-sql.txt`, `evidence/compile-std-sql.txt` |
| `specs/architecture-modularity/spec.md` → `Dependency boundary is checked deterministically` | `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json`, `evidence/deps-allowlist-diff.txt`, `evidence/deps-transitive-review-<crate>.md` |
| `specs/architecture-modularity/spec.md` → `Feature-topology verification is reviewable` | `evidence/core-default-features.txt`, `evidence/compile-default.txt`, `evidence/compile-no-default.txt`, `evidence/compile-sql.txt`, `evidence/compile-std.txt`, `evidence/compile-std-sql.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt` |

## Review Scope Snapshot

No implementation evidence yet.
Add changed-file paths and any saved diff artifacts once tasks begin landing.

## Verification Commands

No commands have been run for this change yet.
Populate this section with exact commands and saved outputs as evidence lands.
