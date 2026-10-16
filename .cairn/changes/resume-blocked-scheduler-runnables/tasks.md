# F09 tasks

Implementation is authorized under the bounded completion campaign. Checked tasks
record implemented and exercised work; required acceptance blockers remain open.

- [x] [serial] Record the smallest baseline from existing core scheduler tests and `live_scheduler_wake_shell_routes_only_admitted_wake_transitions`. Cite r[molten.audit_f09.validation].
- [x] [serial] Add the `Wake -> Block -> Wake` reproduction to normal core tests with positive and duplicate-phase controls. Cite r[molten.audit_f09.resume].
- [x] [serial] Add full-active-limit, full-ready-queue, stale-generation, and sequence-overflow tests with complete rejected-state equality. Cite r[molten.audit_f09.queue] and r[molten.audit_f09.ordering].
- [x] [serial] Coordinate F10 Ready admission ownership before core edits without a circular lifecycle dependency. Cite r[molten.audit_f09.queue].
- [x] [serial] Implement pure blocked-resume classification, checked enqueue updates, supplied priority, and wait-account reset. Cite r[molten.audit_f09.resume] and r[molten.audit_f09.ordering].
- [x] [serial] Remove the shell charge for an existing blocked occurrence while preserving the extension envelope for new work. Cite r[molten.audit_f09.resume].
- [ ] [serial] Add positive and negative live/simulation adapter tests for effect order, overload, stale callbacks, and state preservation. Cite r[molten.audit_f09.validation].
- [x] [serial] Review F11 occurrence compatibility and record Wake, replay-divergence, and canonical receipt decisions in fabric-time docs. Cite r[molten.audit_f09.ordering] and r[molten.audit_f09.validation].
- [x] [serial] Run the focused baseline and new tests after edits, including scheduler capacity and replay fixtures. Cite r[molten.audit_f09.validation].
- [ ] [serial] Run focused Octet checks, the existing strict Octet gate, and Clippy with `-D warnings` across required targets and features. Cite r[molten.audit_f09.validation].
- [ ] [serial] Run required workspace tests, relevant Nix checks, and the repository Nix gate without weakening existing scope. Cite r[molten.audit_f09.validation].
- [x] [serial] Run native `.cairn/` validation and required change gates with tracked evidence and explicit executed-versus-static limits. Cite r[molten.audit_f09.validation].

## 2026-10-15 acceptance attempt

Fresh execution is recorded in
[`evidence/2026-10-15-verification.md`](evidence/2026-10-15-verification.md).
Five new adapter regressions passed for both admitted profile kinds, including
observed live thread wakes. Task 11 remains open: these tests do not implement or
prove the missing production scheduler/capacity/effect composition or independent
simulation effect dispatcher. Required strict Octet acceptance denied the run;
the compatible-feature and required Nix rails are not complete. No sync or
archive is authorized by these partial results.
