# Human / oracle escalation checkpoint

Repeated omission findings must stop automatic checkbox advancement until an explicit escalation decision is recorded.

Trigger:

- Same class/scope omission appears after one attempted fix in `openspec_gate`, `done-review`, or deterministic preflight output.
- Review says task status cannot be re-verified from supplied evidence.
- Review says required transcript/evidence was claimed but not attached.

Action:

1. Do not check additional implementation or verification tasks.
2. Capture the exact failing review/gate transcript under `openspec/changes/prepare-crate-extraction/evidence/`.
3. Add or update the deterministic task/evidence rail that would have caught the omission.
4. Ask a human/oracle reviewer to confirm whether the revised rail closes the omission before treating the affected status as verified.

Current application:

- The tasks gate reported repeated review-scope omission around missing gate transcripts and unverifiable checked-task evidence.
- `tasks.md` V11 now requires actual `openspec_gate tasks` transcript synchronization before intermediate or final task-status claims.
- `verification.md` links `openspec/changes/prepare-crate-extraction/evidence/openspec-gate-tasks-i8.txt` for the original captured gate transcript.
- The V2 checkpoint gate rejected a premature checked V2 status; the corrected task state unchecks V2, keeps its evidence prepared-only, and records `openspec/changes/prepare-crate-extraction/evidence/openspec-gate-tasks-v2.txt` plus `openspec/changes/prepare-crate-extraction/evidence/openspec-gate-tasks-v2-corrected.txt`.
- Required confirmation before future status claims: a later gate or reviewer must accept that `verification.md`, preflight, and gate transcripts are synchronized for every checked task before additional task boxes are checked.
