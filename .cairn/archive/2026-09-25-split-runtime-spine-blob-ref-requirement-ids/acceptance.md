# Change-local acceptance

a[split-runtime-spine-blob-ref-requirement-ids.identity] The requirement-id set of `.cairn/specs/runtime-spine/spec.md` is identical before and after the edit, and each of the four original blob-ref blocks is replaced by one single-id requirement per original id.
a[split-runtime-spine-blob-ref-requirement-ids.meaning] Every original requirement sentence and scenario step is kept verbatim, added scenarios only restate their own id's existing sentence, no text outside the four blocks changes, and the design records the id mapping and the bounded exception.
a[split-runtime-spine-blob-ref-requirement-ids.merge] `cairn validate --strict` passes, and a `cairn sync own-remote-assertions-per-session` preview no longer reports `delta_merge.accepted_identity_cardinality`.
a[split-runtime-spine-blob-ref-requirement-ids.traceability] Every `molten.blob_ref_jobs.*` marker still resolves, and the `inherited-tracey-debt` and `requirement-traceability-gate` Nix checks report no new dangling, uncovered, or classification drift caused by this edit.
