## Why

Every change that alters proof, evidence, gates, or mutation behavior should carry the same review questions: what is proved, what is out of scope, what remains trusted, what positive and negative evidence exists, what refs bind the proof, and which command regenerates it. A checklist makes those questions visible before implementation is archived.

## What Changes

- Add a proof checklist requirement for relevant Cairn changes.
- Require tasks or evidence notes for positive coverage, negative coverage, traceability updates, Hegel RS properties when core invariants change, and release-review commands.
- Require explicit exemptions for documentation-only or non-executable claims.
- Document the checklist in contributor workflow.

## Impact

- **Files**: Cairn project/spec workflow docs, templates or examples, validation guidance.
- **Testing**: fixture changes with complete and incomplete checklists, plus Hegel RS properties where checklist parsing is pure core logic.
