# Tasks: operator-context-profiles

## Phase 1: Context model and pure expansion

- [ ] [serial] r[molten.operator_workflow.context_profile.artifact] Define the canonical operator context profile artifact and validation model.
- [ ] [serial] r[molten.operator_workflow.context_profile.expansion] Implement a pure expansion core from context profile plus operation requirements to explicit command refs.
- [ ] [serial] r[molten.operator_workflow.context_profile.overrides] Define deterministic CLI/profile merge and override-denial rules.

## Phase 2: Representative CLI integration

- [ ] [parallel] r[molten.operator_workflow.context_profile.expansion] Add `--context-profile` support to a small representative set of read-only and mutating commands without bypassing existing command cores.
- [ ] [parallel] r[molten.operator_workflow.context_profile.evidence_only] Bind profile refs and expanded refs into command receipts and diagnostics while preserving downstream gates.
- [ ] [parallel] r[molten.operator_workflow.context_profile.artifact] Document profile schema, operation scopes, and examples.

## Phase 3: Tests and validation

- [ ] [parallel] r[molten.operator_workflow.context_profile.artifact] Add positive and negative context profile fixtures.
- [ ] [parallel] r[molten.operator_workflow.context_profile.expansion] Add tests that valid profiles expand to the same command-core inputs as explicit refs.
- [ ] [parallel] r[molten.operator_workflow.context_profile.overrides] Add negative tests for conflicting overrides and missing required refs.
- [ ] [parallel] r[molten.operator_workflow.context_profile.evidence_only] Add negative tests that profile-only evidence cannot authorize mutation.
- [ ] [serial] r[molten.operator_workflow.context_profile.artifact] Run focused operator-workflow tests and Cairn proposal/design/tasks/spec gates.
