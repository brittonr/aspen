## Phase 1: Source preparation

- [x] [serial] Complete the molten-node-host package rule in license-policy.tsv without changing grants, pins, generated plans, or checker semantics. r[molten.project.license_boundary.final_validation]
- [x] [serial] Clarify owned-source scope, prospective grant history, and complete workspace enforcement in the existing README licensing section. r[molten.project.license_boundary.final_validation]

## Phase 2: Parent final verification

- [x] [serial] Parent runs tools/check-license-boundary.rs from the dedicated worktree and records the actual result; worker deliberately runs no builds, tests, lint, formatters, or broad validation. r[molten.project.license_boundary.final_validation]
- [x] [serial] Parent performs final Cairn validation and review before deciding sync/archive; no completion, publication, or anonymous-acquisition claim is authorized by source preparation. r[molten.project.license_boundary.final_validation]
