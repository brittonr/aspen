## Phase 1: Records and summaries

- [x] [serial] r[molten.coordination_control_plane_ux.apply_report] Add canonical coordination apply report records and ledger classification.
- [x] [parallel] r[molten.coordination_control_plane_ux.readonly_show] Extend read-only artifact summaries for generated coordination artifacts.

## Phase 2: CLI UX

- [x] [serial] r[molten.coordination_control_plane_ux.manifest_request_cli] Add manifest and request generation commands for canonical coordination records.
- [x] [serial] r[molten.coordination_control_plane_ux.apply_batch_cli] Add batch apply command that routes request artifacts through the control-plane runtime.
- [x] [parallel] r[molten.coordination_control_plane_ux.idempotent_replay] Preserve duplicate operation-id replay semantics in the CLI batch report.

## Phase 3: Coverage and docs

- [x] [serial] r[molten.coordination_control_plane_ux.cli_tests] Cover manifest/request/apply/show commands and duplicate request replay.
- [x] [parallel] r[molten.coordination_control_plane_ux.docs] Document the coordination control-plane UX and evidence-only trust boundary.
