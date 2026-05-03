## Task Coverage

- [x] Trust/crypto/secrets owner/public API review package and scope artifact created.
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-public-api-review-scope.md`
- [x] Trust/crypto/secrets manifest and inventory record the review decision and remaining blockers.
  - Evidence: `docs/crate-extraction/trust-crypto-secrets.md`
  - Evidence: `docs/crate-extraction.md`
- [x] Trust/crypto/secrets extraction spec includes owner/public API readiness review gate.
  - Evidence: `openspec/specs/trust-crypto-secrets-extraction/spec.md`
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/specs/trust-crypto-secrets-extraction/spec.md`
- [x] Fresh crate checks passed for the candidate reusable and compatibility surfaces.
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-crate-checks.txt`
- [x] Fresh dependency graphs captured for reusable and runtime/compatibility surfaces.
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-dependency-graphs.txt`
- [x] Source/readiness guards captured for checker pseudo-candidate coverage and runtime boundary references.
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-source-guards.txt`
- [x] Downstream metadata, negative boundary, and compatibility evidence captured for the current aggregate review.
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-downstream-metadata.json`
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-forbidden-boundary.txt`
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-compatibility.txt`
- [x] Readiness checker output captured and reviewed.
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-readiness.md`
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-readiness.json`
  - Decision: keep `trust-crypto-secrets` at `workspace-internal`; checker evidence passes only with a warning that direct package dependency checks are deferred, so promotion remains blocked by checker pseudo-candidate coverage and public API stability policy.
- [x] OpenSpec validation/helper verification, markdownlint, and diff checks passed.
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/openspec-helper-verify.txt`
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/openspec-validate.txt`
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/markdownlint.txt`
  - Evidence: `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/git-diff-check.txt`

## Verification Commands

```bash
cargo check -p aspen-crypto --no-default-features
cargo check -p aspen-secrets-core --no-default-features
cargo check -p aspen-trust
cargo check -p aspen-secrets --no-default-features
cargo check -p aspen-secrets-handler --no-default-features
cargo tree -p aspen-crypto --no-default-features -e normal --depth 2
cargo tree -p aspen-secrets-core --no-default-features -e normal --depth 2
cargo tree -p aspen-trust -e normal --depth 1
cargo tree -p aspen-secrets --no-default-features -e normal --depth 1
cargo tree -p aspen-secrets-handler --no-default-features -e normal --depth 1
scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family trust-crypto-secrets --output-json openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-readiness.json --output-markdown openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/trust-crypto-secrets-readiness.md
python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify review-trust-crypto-secrets-public-api --json
openspec validate review-trust-crypto-secrets-public-api --json
markdownlint docs/crate-extraction.md docs/crate-extraction/trust-crypto-secrets.md openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/**/*.md openspec/specs/trust-crypto-secrets-extraction/spec.md
git diff --check
```
