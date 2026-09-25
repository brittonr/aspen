# Design: Split the runtime-spine blob-ref requirement ids

## Context

Accepted specs change only through a synced delta. This repository's precedent for repairing an accepted marker form
is the archived `restore-tracey-requirement-root` change. Its MODIFIED delta restated four `aspen.cas.*` requirements
whose ids sat in their headings. That worked because each of those accepted blocks still carried exactly one id.

The four blob-ref blocks in `.cairn/specs/runtime-spine/spec.md` carry 2, 4, 3, and 3 ids. In cairn@fde71b2,
`parse_accepted_document` (`crates/cairn-core/src/verified/delta_merge.rs`, l.383-395) pushes
`delta_merge.accepted_identity_cardinality` for each such block, and `merge_delta_spec` (l.122-135) blocks the merge on
any diagnostic before `apply_operations` runs. The delta route is therefore closed for the whole `runtime-spine`
spec. A scratch probe showed this: a copied `.cairn/` tree with a probe change whose MODIFIED delta restated only
`molten.blob_ref_jobs.payload_model` returned `blocked: true` and the same four `accepted_identity_cardinality`
reasons.

## Decisions

### Decision: Hand-split the accepted blocks under a bounded exception

**Choice:** Edit `.cairn/specs/runtime-spine/spec.md` directly. Replace the four blob-ref blocks with twelve
single-id requirements and change nothing else. The change declares the `no-spec-delta` profile and states
explicitly that it edits an accepted spec under this exception.

**Rationale:** No delta, including a repair, can currently touch `runtime-spine`. The split restores the one-id form
that the Cairn merge preflight requires, so the next delta against this spec can sync.

**Exception record** (workspace rule: urgent one-time work only through a bounded exception):

- **Reason:** cairn@fde71b2 `delta_merge.rs` `parse_accepted_document` (l.383-395) rejects legacy multi-id accepted
  blocks before any operation applies, so no delta, including a repair, can touch `runtime-spine`. The scratch probe
  above confirms this.
- **Recurrence decision:** This is a one-time migration of legacy content written before the one-id rule. Future
  repairs go through deltas once Cairn can replace a multi-id accepted block.
- **Revisit trigger:** Cairn gains the ability for a MODIFIED operation to replace a multi-id accepted block by any
  of its ids. This is a Cairn follow-up; this change does not edit `../cairn`.
- **Owner:** the Aspen lifecycle owner.
- **Capability non-claim:** No requirement meaning changes and no new obligations are added. The exception does not
  authorize other direct edits to accepted specs.

### Decision: One requirement per existing id, statement by statement

**Choice:** Each original block lists one requirement sentence per id, so each id keeps its own original sentence
verbatim. Each original scenario goes to the id whose sentence it exercises. Four ids had no scenario of their own
(`provenance_policy`, `retention_pins`, `local_tests`, `property_tests`). Each gets one scenario that restates only its
own requirement sentence. The first id of each block keeps the original heading. The others get headings that
paraphrase their sentence.

**Rationale:** The statements are separable, so no shared obligation needs restating. Scenarios that restate
existing sentences keep each block well-formed without adding obligations.

#### Mapping

| Original block | Id | New heading | Scenario source |
| --- | --- | --- | --- |
| Blob-ref job submissions | `payload_model` | Blob-ref job submissions | "Content-ref-only submission" (verbatim) |
| Blob-ref job submissions | `no_inline_large_bytes` | Blob-ref job submissions reject inline large bytes | "Inline large content is denied" (verbatim) |
| Blob-ref worker fetch and verification | `local_worker` | Blob-ref worker fetch and verification | "Verified local worker execution" (verbatim) |
| Blob-ref worker fetch and verification | `content_verification` | Blob-ref content is verified before execution | "Missing or tampered content ref" (verbatim) |
| Blob-ref worker fetch and verification | `provenance_policy` | Blob-ref executables carry provenance and policy refs | restates its own sentence |
| Blob-ref worker fetch and verification | `retention_pins` | Blob-ref content refs are pinned while active | restates its own sentence (the verbatim local-worker scenario also pins active refs) |
| Blob-ref job status and receipt evidence | `status_assertions` | Blob-ref job status and receipt evidence | "Status lifecycle evidence" (verbatim) |
| Blob-ref job status and receipt evidence | `receipts` | Blob-ref job receipts bind lifecycle evidence | "Receipt replay identity" steps, verbatim, titled "Receipt binds lifecycle evidence" |
| Blob-ref job status and receipt evidence | `replay_integration` | Blob-ref job evidence enters replay identity | "Receipt replay identity" (verbatim) |
| Blob-ref job DAG integration | `job_dag_integration` | Blob-ref job DAG integration | "CLI and ledger integration" and "Evidence only" (verbatim) |
| Blob-ref job DAG integration | `local_tests` | Blob-ref local job lifecycle is tested | restates its own sentence |
| Blob-ref job DAG integration | `property_tests` | Blob-ref invariants have property coverage | restates its own sentence |

(Ids abbreviated: each is `molten.blob_ref_jobs.<id>`.) The "Receipt replay identity" scenario exercises both
`receipts` and `replay_integration`, so both keep its steps. The exact before and after text is in
`evidence/original-blob-ref-blocks.md` and `evidence/split-blob-ref-blocks.md`.

## No-spec classification

Accepted requirement meaning does not change. The accepted file changes only in structure: the id set is identical
before and after, and every requirement sentence and original scenario step is kept verbatim. Semantic review
inputs: this design's mapping, the two evidence block files, the `.cairn/specs/runtime-spine/spec.md` diff, and the
id-set files.

## Failure behavior

- If an id is lost, duplicated, or added, the before and after id-set files differ and the change fails.
- If a marker stops resolving, the `inherited-tracey-debt` guard reports it as dangling.
- If the split leaves another malformed block, the Cairn sync preview for `own-remote-assertions-per-session` stays
  blocked, and strict validation fails.

## Risks / Trade-offs

- A direct accepted-spec edit bypasses the delta audit trail. The exception record, the before and after block
  evidence, and the review receipt stand in for it.
- The new headings are paraphrases. A reader who searched by an old heading finds only the first id of each block
  under it.
