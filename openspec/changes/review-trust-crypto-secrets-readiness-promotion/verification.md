# Verification

## Task Coverage

- Decision/policy/docs task: package-scoped promotion recorded in policy, inventory, manifest, and OpenSpec spec.
  - Evidence: `docs/crate-extraction/policy.ncl`
  - Evidence: `docs/crate-extraction.md`
  - Evidence: `docs/crate-extraction/trust-crypto-secrets.md`
  - Evidence: `openspec/specs/trust-crypto-secrets-extraction/spec.md`
- Verification task: readiness, dependency, serialization, compatibility, and hygiene checks captured below.
  - Evidence: `evidence/trust-crypto-secrets-readiness-decision.md`

## Command Evidence

- Readiness checker: `scripts/check-crate-extraction-readiness.rs --candidate-family trust-crypto-secrets`
  - Evidence: `evidence/trust-crypto-secrets-readiness.json`
  - Evidence: `evidence/trust-crypto-secrets-readiness.md`
- Reusable crate checks: `cargo check` for `aspen-crypto`, `aspen-trust`, and `aspen-secrets-core` default/no-default surfaces.
  - Evidence: `evidence/trust-crypto-secrets-crate-checks.txt`
- Serialization contracts: targeted `serialization_contract` tests for `aspen-trust` and `aspen-secrets-core`.
  - Evidence: `evidence/trust-crypto-secrets-serialization-tests.txt`
- Dependency boundary graphs and forbidden runtime guards.
  - Evidence: `evidence/trust-crypto-secrets-dependency-graphs.txt`
  - Evidence: `evidence/trust-crypto-secrets-forbidden-boundary.txt`
- Runtime compatibility checks for non-promoted service/adapter surfaces.
  - Evidence: `evidence/trust-crypto-secrets-compatibility.txt`
- Source guard and decision summary.
  - Evidence: `evidence/trust-crypto-secrets-source-guards.txt`
  - Evidence: `evidence/trust-crypto-secrets-readiness-decision.md`
- OpenSpec/helper, markdown, and diff checks.
  - Evidence: `evidence/openspec-helper-verify.txt`
  - Evidence: `evidence/openspec-validate.txt`
  - Evidence: `evidence/markdownlint.txt`
  - Evidence: `evidence/git-diff-check.txt`
