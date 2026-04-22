Evidence-ID: extend-no-std-foundation-and-wire.root-verification
Artifact-Type: claim-to-artifact-index

# Verification Evidence

This file is the root claim-to-artifact index for `extend-no-std-foundation-and-wire`.
Durable proof for this change must live under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`.

## Evidence Freshness Rule

- Only artifacts generated under `openspec/changes/extend-no-std-foundation-and-wire/evidence/` and indexed from this file satisfy this change's verification claims.
- Archived evidence from earlier no-std changes may be cited as baseline context only.
- Archived evidence MUST NOT substitute for final proof of the storage, trait, client-api, or protocol seams changed here.

## Evidence Naming Convention

Planned artifact families for this change:

- storage seam and UI/boundary artifacts: `evidence/storage-seam-*.txt`, `evidence/storage-seam-*.md`, saved stderr snapshots, and source-audit outputs
- core foundation and dependency-boundary artifacts: `evidence/core-foundation-*.txt`, `evidence/core-foundation-*.md`, dependency graphs, feature graphs, and target-build transcripts
- wire dependency artifacts: `evidence/wire-dependency-*.txt`, `evidence/wire-dependency-*.md`, dependency graphs, feature graphs, and target-build transcripts
- wire compatibility artifacts: `evidence/wire-compatibility-*.txt`, `evidence/wire-compatibility-*.md`, `evidence/client-rpc-postcard-baseline.json`, and test transcripts comparing current encodings against that baseline
- traceability plans: `evidence/verification-plan.md`, `evidence/*-verification.md`

## Planned Task Coverage

No tasks are checked yet. During implementation this section must copy each checked task from `tasks.md` exactly and cite repo-relative evidence paths.

Planned verification-plan anchors:

- `V1` → `openspec/changes/extend-no-std-foundation-and-wire/evidence/storage-seam-verification.md`
- `V2` → `openspec/changes/extend-no-std-foundation-and-wire/evidence/core-foundation-verification.md`
- `V3` → `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-dependency-verification.md`
- `V4` → `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md`
- `V5` → `openspec/changes/extend-no-std-foundation-and-wire/verification.md` with supporting plan context in `openspec/changes/extend-no-std-foundation-and-wire/evidence/verification-plan.md`

## Planned Scenario Coverage

Implementation phase must map every changed scenario from:

- `openspec/changes/extend-no-std-foundation-and-wire/specs/core/spec.md`
- `openspec/changes/extend-no-std-foundation-and-wire/specs/architecture-modularity/spec.md`

to concrete artifacts under this change's `evidence/` directory.

Required artifact categories include:

- compile-fail/UI proof for shell-only imports such as `SM_KV_TABLE`
- alloc-only target-build proof for `aspen-storage-types`, `aspen-traits`, `aspen-client-api`, `aspen-coordination-protocol`, `aspen-jobs-protocol`, and `aspen-forge-protocol`
- dependency and feature-resolution graphs for the narrowed alloc-only boundary, including `cargo tree -p aspen-traits -e features -i aspen-cluster-types`
- deterministic source-audit artifacts proving forbidden leaf/wire helpers stay out of production modules
- representative consumer compile transcripts for shell/runtime users
- deterministic postcard-baseline artifacts for `ClientRpcRequest` and `ClientRpcResponse`, with at least one variant-keyed default-production encoding for every request/response enum variant

## Notes

Before any task is checked complete, update this file from plan-only status to a real claim-to-artifact index and keep it synchronized with the per-topic evidence files under `evidence/`.
