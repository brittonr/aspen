# Verification

## Task Coverage

- Task: Map `trust-crypto-secrets` readiness checks to real reusable package candidates.
  - Evidence: `evidence/trust-crypto-secrets-checker-mapping.txt`
  - Evidence: `evidence/trust-crypto-secrets-readiness.json`
  - Evidence: `evidence/trust-crypto-secrets-readiness.md`

- Task: Split policy metadata for `aspen-crypto` and `aspen-secrets-core` while keeping readiness `workspace-internal`.
  - Evidence: `evidence/trust-crypto-secrets-readiness.json`
  - Evidence: `evidence/trust-crypto-secrets-forbidden-boundary.txt`

- Task: Refresh docs to remove the aggregate pseudo-candidate blocker and preserve remaining blockers.
  - Evidence: `docs/crate-extraction.md`
  - Evidence: `docs/crate-extraction/trust-crypto-secrets.md`

- Task: Capture checker, dependency, compatibility, OpenSpec, Markdown, and diff evidence.
  - Evidence: `evidence/trust-crypto-secrets-crate-checks.txt`
  - Evidence: `evidence/trust-crypto-secrets-dependency-graphs.txt`
  - Evidence: `evidence/trust-crypto-secrets-compatibility.txt`
  - Evidence: `evidence/openspec-helper-verify.txt`
  - Evidence: `evidence/openspec-validate.txt`
  - Evidence: `evidence/markdownlint.txt`
  - Evidence: `evidence/git-diff-check.txt`
