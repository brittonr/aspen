# Verification

Evidence paths are relative to this change directory before archive. After archive, paths move under `openspec/changes/archive/2026-05-03-complete-aspen-trust-async-default-policy/`.

## Cargo checks

- Evidence: `evidence/aspen-trust-default-check.txt`
- Evidence: `evidence/aspen-trust-no-default-check.txt`
- Evidence: `evidence/aspen-trust-async-check.txt`
- Evidence: `evidence/aspen-trust-no-default-test.txt`
- Evidence: `evidence/aspen-trust-async-test.txt`
- Evidence: `evidence/aspen-raft-trust-check.txt`
- Evidence: `evidence/aspen-raft-types-check.txt`
- Evidence: `evidence/aspen-secrets-trust-check.txt`
- Evidence: `evidence/aspen-transport-trust-handler-check.txt`

## Dependency guards

- Evidence: `evidence/aspen-trust-dependency-graphs.txt`
- Evidence: `evidence/aspen-trust-default-forbidden-guard.txt`
- Evidence: `evidence/aspen-trust-async-positive-guard.txt`

## Readiness/OpenSpec/docs hygiene

- Evidence: `evidence/trust-crypto-secrets-readiness-command.txt`
- Evidence: `evidence/trust-crypto-secrets-readiness.json`
- Evidence: `evidence/trust-crypto-secrets-readiness.md`
- Evidence: `evidence/openspec-helper-verify.txt`
- Evidence: `evidence/openspec-validate.txt`
- Evidence: `evidence/markdownlint.txt`
- Evidence: `evidence/git-diff-check.txt`

## Task Coverage

- Gate async service modules and optional dependencies.
  - Evidence: `evidence/aspen-trust-no-default-check.txt`
  - Evidence: `evidence/aspen-trust-default-check.txt`
  - Evidence: `evidence/aspen-trust-async-check.txt`
  - Evidence: `evidence/aspen-trust-no-default-test.txt`
  - Evidence: `evidence/aspen-trust-async-test.txt`
  - Evidence: `evidence/aspen-trust-dependency-graphs.txt`
  - Evidence: `evidence/aspen-trust-default-forbidden-guard.txt`
  - Evidence: `evidence/aspen-trust-async-positive-guard.txt`
- Preserve runtime compatibility.
  - Evidence: `evidence/aspen-raft-trust-check.txt`
  - Evidence: `evidence/aspen-raft-types-check.txt`
  - Evidence: `evidence/aspen-secrets-trust-check.txt`
  - Evidence: `evidence/aspen-transport-trust-handler-check.txt`
  - Evidence: `evidence/trust-crypto-secrets-compatibility.txt`
- Add `aspen_trust` to policy/checker coverage.
  - Evidence: `evidence/trust-crypto-secrets-readiness.json`
  - Evidence: `evidence/trust-crypto-secrets-readiness.md`
  - Evidence: `evidence/trust-crypto-secrets-readiness-command.txt`
  - Evidence: `evidence/trust-crypto-secrets-downstream-metadata.json`
  - Evidence: `evidence/trust-crypto-secrets-forbidden-boundary.txt`
- Update docs/specs and hygiene evidence.
  - Evidence: `evidence/openspec-helper-verify.txt`
  - Evidence: `evidence/openspec-validate.txt`
  - Evidence: `evidence/markdownlint.txt`
  - Evidence: `evidence/git-diff-check.txt`
