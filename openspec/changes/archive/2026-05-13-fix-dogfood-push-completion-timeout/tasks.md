## Phase 1: Snapshot push fix

- [x] [serial] Add an OpenSpec baseline for dogfood push-completion timeout repair.
- [x] [depends:baseline] Implement a bounded single-commit source snapshot workspace for dogfood-local pushes.
- [x] [depends:implementation] Add unit coverage for snapshot workspace command shape and push workspace selection.
- [x] [depends:tests] Run focused dogfood tests, formatting, OpenSpec validation, and whitespace checks.
- [x] [depends:verification] Run focused `push-check` and record redacted receipt evidence or the new bounded blocker. ✅ evidence: `evidence/push-check-snapshot.md`, receipt `dogfood-20260513T171735Z`
