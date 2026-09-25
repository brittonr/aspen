# Tasks: Split the runtime-spine blob-ref requirement ids

## Phase 1: Implementation

- [x] [serial] Record the Cairn preflight blocker and the scratch MODIFIED-delta probe in the design exception record. a[split-runtime-spine-blob-ref-requirement-ids.meaning]
- [x] [serial] Split the four blob-ref blocks into one requirement per existing id and record the mapping. a[split-runtime-spine-blob-ref-requirement-ids.identity] a[split-runtime-spine-blob-ref-requirement-ids.meaning]

## Phase 2: Validation

- [x] [serial] Positive: the before and after requirement-id sets are identical and the diff touches only the four blocks. a[split-runtime-spine-blob-ref-requirement-ids.identity]
- [x] [serial] Negative: the pre-edit sync preview for `own-remote-assertions-per-session` is blocked by `accepted_identity_cardinality`, and the post-edit preview is not. a[split-runtime-spine-blob-ref-requirement-ids.merge]
- [x] [serial] Run `cairn validate --strict` after the edit. a[split-runtime-spine-blob-ref-requirement-ids.merge]
- [x] [serial] Confirm every `molten.blob_ref_jobs.*` marker resolves, then build `inherited-tracey-debt` and `requirement-traceability-gate`. a[split-runtime-spine-blob-ref-requirement-ids.traceability]
