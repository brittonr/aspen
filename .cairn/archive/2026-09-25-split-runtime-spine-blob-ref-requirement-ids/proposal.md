# Proposal: Split the runtime-spine blob-ref requirement ids

## Why

`cairn sync own-remote-assertions-per-session` is blocked. The Cairn merge preflight reports
`delta_merge.accepted_identity_cardinality` for four accepted blocks in `.cairn/specs/runtime-spine/spec.md`. Each
block carries more than one `r[...]` id before its first scenario:

| Accepted block | Ids |
| --- | --- |
| Blob-ref job submissions | 2 |
| Blob-ref worker fetch and verification | 4 |
| Blob-ref job status and receipt evidence | 3 |
| Blob-ref job DAG integration | 3 |

The blocks came from archive `2026-06-09-blob-ref-job-submission`, which predates the one-id-per-requirement rule.
In cairn@fde71b2, `parse_accepted_document` (`crates/cairn-core/src/verified/delta_merge.rs`, l.383-395) rejects these
blocks before any delta operation applies, and `merge_delta_spec` blocks on any diagnostic. So no delta against
`runtime-spine` can sync, including a MODIFIED repair. A scratch probe confirmed this. A MODIFIED delta that restated
only `molten.blob_ref_jobs.payload_model` with a single id still returned `blocked: true` with the same four reasons.

## What Changes

- This change edits the accepted spec `.cairn/specs/runtime-spine/spec.md` directly, under a bounded one-time
  exception (see design). That is the only way the accepted file can reach a form Cairn can merge into.
- The edit splits the four multi-id blocks into twelve requirements, one per existing id, and changes nothing else in
  the file. Every id, its requirement sentence, and every original scenario is kept. Ids that had no scenario of
  their own get one that restates their existing requirement sentence.
- Accepted requirement meaning does not change, so the `no-spec-delta` profile applies. The change carries no delta
  and syncs nothing. The accepted-spec file changes only by structure, as recorded in the design's mapping.

## Impact

- **Files**: `.cairn/specs/runtime-spine/spec.md` (lines of the four blob-ref blocks only), plus this change's
  artifacts and evidence.
- **Testing**: an identical requirement-id set before and after (`evidence/requirement-ids-{before,after}.txt`);
  `cairn validate --strict`; a `cairn sync own-remote-assertions-per-session` preview that is no longer blocked; the
  `inherited-tracey-debt` and `requirement-traceability-gate` Nix checks; every `molten.blob_ref_jobs.*` marker still
  resolves.

## Out of Scope

- Changing Cairn (`../cairn`). Letting a MODIFIED operation replace a multi-id accepted block is a Cairn follow-up.
- Any other accepted spec. None hit this preflight while this repair's successors synced.
- Behavior, code, or marker changes. No source file is edited.
