## Task Coverage

- [x] Owner/public API review change package and scope artifact created.
  - Evidence: `openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/evidence/jobs-ci-public-api-review-scope.md`
- [x] Jobs/CI extraction manifest and inventory record the active owner/public API review scope.
  - Evidence: `docs/crate-extraction/jobs-ci-core.md`
  - Evidence: `docs/crate-extraction.md`
- [x] Jobs/CI core extraction spec purpose and owner/public API review requirement are recorded.
  - Evidence: `openspec/specs/jobs-ci-core-extraction/spec.md`
  - Evidence: `openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/specs/jobs-ci-core-extraction/spec.md`
- [x] Fresh downstream metadata captured.
  - Evidence: `openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/evidence/jobs-ci-core-downstream-metadata.json`
- [x] Negative runtime-boundary evidence captured.
  - Evidence: `openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/evidence/jobs-ci-core-forbidden-boundary.txt`
- [x] Focused compatibility checks passed.
  - Evidence: `openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/evidence/jobs-ci-core-compatibility.txt`
- [x] Readiness checker passed with no failures or warnings.
  - Evidence: `openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/evidence/jobs-ci-core-readiness.md`
  - Evidence: `openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/evidence/jobs-ci-core-readiness.json`
- [x] Jobs/CI core readiness raised to `extraction-ready-in-workspace`; publishable/repo-split remains blocked on license/publication policy.
  - Evidence: `docs/crate-extraction.md`
  - Evidence: `docs/crate-extraction/jobs-ci-core.md`
  - Evidence: `docs/crate-extraction/policy.ncl`

## Verification Commands

```bash
cargo test -p aspen-jobs-core --no-fail-fast
cargo test -p aspen-ci-core --no-fail-fast
cargo test -p aspen-client-api request_metadata --no-fail-fast
cargo test -p aspen-ci-handler --lib --features forge,blob --no-fail-fast
cargo check -p aspen-ci-handler --features blob
scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family jobs-ci-core --output-json openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/evidence/jobs-ci-core-readiness.json --output-markdown openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/evidence/jobs-ci-core-readiness.md
python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify review-jobs-ci-public-api --json
openspec validate review-jobs-ci-public-api --json
markdownlint docs/**/*.md openspec/changes/archive/2026-05-02-review-jobs-ci-public-api/**/*.md openspec/specs/jobs-ci-core-extraction/spec.md
git diff --check
```
