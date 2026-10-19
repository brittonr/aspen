# F10 tasks

Implementation is authorized under the bounded completion campaign. Checked tasks
record implemented and exercised work; required acceptance blockers remain open.

- [x] [serial] Run the smallest existing core scheduler, capacity, and shell tests before core edits and retain the baseline result. Cite r[molten.audit_f10.validation].
- [x] [serial] Move the queue-bound-one A/B Yield reproduction into normal core tests with a successful Yield control. Cite r[molten.audit_f10.queue].
- [x] [serial] Add both overload policies, invalid phases, stale generations, and enqueue overflow with complete rejected-state equality. Cite r[molten.audit_f10.atomicity].
- [x] [serial] Define shared pure Ready admission for new Wake, blocked Wake, and Yield with F09 integration ownership. Cite r[molten.audit_f10.shared_admission].
- [x] [serial] Implement checked admission before phase, count, and enqueue mutation without new active charges for existing occurrences. Cite r[molten.audit_f10.queue] and r[molten.audit_f10.atomicity].
- [x] [serial] Adapt shell capacity effects to accepted decisions and preserve Running ownership on rejected Yield. Cite r[molten.audit_f10.atomicity].
- [x] [serial] Add positive and negative live/simulation adapter tests for action translation, effect order, capacity, and denied-state preservation. Cite r[molten.audit_f10.validation].
- [x] [serial] Record compatibility, overload caller behavior, replay divergence, and bounded receipt decisions in fabric-time docs. Cite r[molten.audit_f10.validation].
- [x] [serial] Repeat focused tests after edits and add replay controls for unchanged valid schedules and rejected over-capacity history. Cite r[molten.audit_f10.validation].
- [x] [serial] Run focused Octet checks, the existing strict Octet gate, and Clippy with `-D warnings` across required targets and features. Cite r[molten.audit_f10.validation].
- [ ] [serial] Run required workspace tests, relevant Nix checks, and the repository Nix gate without weakening existing checks. Cite r[molten.audit_f10.validation].
- [x] [serial] Run native `.cairn/` validation and required gates with tracked evidence and explicit limits on adapter claims. Cite r[molten.audit_f10.validation].

## 2026-10-15 acceptance attempt

Fresh execution is recorded in
[`evidence/2026-10-15-verification.md`](evidence/2026-10-15-verification.md).
Five new adapter regressions passed for both admitted profile kinds, including
observed live thread wakes. Task 11 remains open: these tests do not implement or
prove the missing production scheduler/capacity/effect composition or independent
simulation effect dispatcher. Required strict Octet acceptance denied the run;
the compatible-feature and required Nix rails are not complete. No sync or
archive is authorized by these partial results.

## 2026-10-17 final bounded attempt

Attempt 20 passed the focused core/shell suites, both clean Octet producers,
the unchanged strict source gate, workspace and compatible-feature tests,
all required Clippy configurations, formatting, and the actual both-profile CLI.
The owning runtime and independent simulation delivery from attempts 18–19 are
now exercised. Required Nix acceptance remains open; no sync or archive is
authorized by these partial results. See
[`evidence/2026-10-17-final-acceptance.md`](evidence/2026-10-17-final-acceptance.md).

