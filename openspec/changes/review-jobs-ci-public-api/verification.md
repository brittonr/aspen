## Task Coverage

- [x] Owner/public API review change package and scope artifact created.
  - Evidence: `openspec/changes/review-jobs-ci-public-api/evidence/jobs-ci-public-api-review-scope.md`
- [x] Jobs/CI extraction manifest and inventory record the active owner/public API review scope.
  - Evidence: `docs/crate-extraction/jobs-ci-core.md`
  - Evidence: `docs/crate-extraction.md`
- [x] Jobs/CI core extraction spec purpose and owner/public API review requirement are recorded.
  - Evidence: `openspec/specs/jobs-ci-core-extraction/spec.md`
  - Evidence: `openspec/changes/review-jobs-ci-public-api/specs/jobs-ci-core-extraction/spec.md`

## Verification Commands

Pending for this active change:

```bash
python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify review-jobs-ci-public-api --json
markdownlint docs/**/*.md openspec/changes/review-jobs-ci-public-api/**/*.md
```
