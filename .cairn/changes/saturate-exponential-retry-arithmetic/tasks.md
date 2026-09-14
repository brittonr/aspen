# F12 tasks

Checked tasks have current execution evidence. Unchecked tasks remain open. This package does not grant implementation permission.

- [x] [serial] Run `retry_plans_are_bounded_and_jitter_explicit` and the smallest existing retry fixture tests before core edits. Cite r[molten.audit_f12.validation]. Evidence: `evidence/core-validation.md`.
- [x] [serial] Move the base-two, attempt-63, maximum-128 reproduction into normal repository tests with named inputs and ordinary-growth controls. Cite r[molten.audit_f12.saturation]. Evidence: `evidence/core-validation.md`.
- [x] [serial] Add high-bit-loss, integer-width, large admitted attempt, and exhausted-budget tests before arithmetic changes. Cite r[molten.audit_f12.bounds]. Evidence: `evidence/core-validation.md`.
- [x] [serial] Implement bounded pure exponential saturation through checked multiplication or a pre-shift overflow check. Cite r[molten.audit_f12.saturation] and r[molten.audit_f12.bounds]. Evidence: `evidence/core-validation.md`.
- [x] [serial] Preserve fixed-delay outputs and existing jitter, generation, domain, and deadline admission with positive and negative tests. Cite r[molten.audit_f12.compatibility]. Evidence: `evidence/core-validation.md`.
- [x] [serial] Add shell and adapter tests for correct plans, error translation, no retry effect after denial, and consumer-state preservation. Cite r[molten.audit_f12.compatibility]. Evidence: `evidence/delivery/validation.md`, `evidence/fixture-replay/validation.md`, and `evidence/domain-binding/validation.md`.
- [x] [serial] Record the replay/cohort decision and test valid replay plus visible divergence for historical wrapped delays. Cite r[molten.audit_f12.validation]. Evidence: `evidence/fixture-replay/validation.md` and the domain-bound correction in `evidence/domain-binding/validation.md`.
- [x] [serial] Update fabric-time docs and canonical receipt fixtures with corrected-delay semantics and the fixed delivery-profile non-claim. Cite r[molten.audit_f12.validation]. Evidence: `evidence/fixture-replay/validation.md` and `evidence/domain-binding/validation.md`.
- [x] [serial] Repeat focused core and adapter tests after edits, including fixed delivery controls and deadline-overflow rejection. Cite r[molten.audit_f12.compatibility] and r[molten.audit_f12.validation]. Evidence: `evidence/fixture-replay/validation.md` and `evidence/domain-binding/validation.md`.
- [ ] [serial] Run focused Octet checks, the existing strict Octet gate, and Clippy with `-D warnings` across required targets and features. Cite r[molten.audit_f12.validation].
- [ ] [serial] Run required workspace tests, relevant Nix checks, and the repository Nix gate without weakening existing checks. Cite r[molten.audit_f12.validation].
- [ ] [serial] Run native `.cairn/` validation and required gates with tracked evidence limited to executed planner and adapter observations. Cite r[molten.audit_f12.validation].
