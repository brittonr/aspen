# Verification: split the runtime-spine blob-ref requirement ids

Base: `e9ece6c69` (integration head `dca8f7135` plus the wasm and simulation sync commits).

## Identity

- `requirement-ids-before.txt` (from the pre-edit file) and `requirement-ids-after.txt` (from the edited file) are
  byte-identical: 476 ids, 476 unique, both with BLAKE3
  `e0149fdfc782a7e6be80052d30f99ea804bac0f0e84c7e18049ac566111191a5`.
- The ids in `original-blob-ref-blocks.md` and `split-blob-ref-blocks.md` are the same 12
  `molten.blob_ref_jobs.*` ids. The edited file starts with the unchanged pre-block prefix and ends with the unchanged
  post-block suffix, so only the four blocks changed.
- The design's mapping table records which block, sentence, and scenario each id came from.

## Merge (negative, then positive)

- `own-remote-sync-preview-before-split.json`: a scratch `.cairn/` copy with the pre-edit spec returns
  `blocked: true` with 4 `delta_merge.accepted_identity_cardinality` reasons.
- `own-remote-sync-preview-after-split.json`: the edited tree returns `blocked: false`, with one operation
  (`added:molten.runtime_spine.remote_assertion_ownership`).
- `cairn validate --strict` passes (`validate-strict.json`: `valid: true`, `issues: []`).
- The proposal, design, and tasks gates exit 0 with `issues: []`, and the no-spec review receipt validates
  (`cairn review validate`: `valid: true`).

## Traceability

- `blob-ref-marker-resolution.txt`: all 9 distinct `molten.blob_ref_jobs.*` ids that carry `r[impl]` or
  `r[verify]` markers resolve to an id in the edited spec. The other 3 (`job_dag_integration`, `replay_integration`,
  `status_assertions`) have no marker and stay in the inherited baseline, as before.
- The `inherited-tracey-debt` guard over the staged tree reports `requirements=2790 referenced=863 uncovered=1927
  baseline_entries=1924 dangling=19`. None of the dangling ids is a `blob_ref_jobs` id; all 19 belong to unsynced
  changes. The check stops at the guard while those 19 remain. To exercise the steps after the guard, the check's
  exact build script ran with only the guard exit made non-fatal. The classifier output then matched
  `inherited-debt-classification.{tsv,md}` (`verdict=pass`, 1924 classified). The baseline, classification, and all
  five runtime-spine repair manifests exported, diffed, and hash-checked unchanged. No binding needed regeneration.
- `requirement-traceability-gate` was built from this change's staged tree (`nix build
  .#checks.x86_64-linux.requirement-traceability-gate`). The successor commit that syncs
  `own-remote-assertions-per-session` records its result.

## Claim boundary

This shows that the edit is structural: the same ids, the same sentences, and the same scenario steps. It does not
show that the blob-ref requirements are implemented or verified beyond the markers that already existed.
