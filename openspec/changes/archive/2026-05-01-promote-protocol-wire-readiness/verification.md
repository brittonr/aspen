# Verification Evidence

## Implementation Evidence

- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/protocol-wire.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `openspec/specs/architecture-modularity/spec.md`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/.openspec.yaml`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/proposal.md`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/design.md`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/tasks.md`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/verification.md`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/specs/architecture-modularity/spec.md`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/Cargo.toml`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/Cargo.lock`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/src/lib.rs`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-cargo-checks.txt`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i3-client-api-compatibility-tests.txt`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-test.txt`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-metadata.json`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-forbidden-grep.txt`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-readiness.json`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-readiness.md`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-readiness-command.txt`

- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-validate.json`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/helper-verify.json`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/implementation-diff.patch`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/git-diff-check.txt`
- Changed file: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-drain-audit.txt`

## Task Coverage

- [x] V1 Promote protocol/wire readiness in inventory, manifest, and policy while preserving publication blocks. [covers=architecture-modularity.protocol-wire-readiness.no-publication-claim]
  - Evidence: `docs/crate-extraction.md`, `docs/crate-extraction/protocol-wire.md`, `docs/crate-extraction/policy.ncl`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-readiness.md`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-cargo-checks.txt`
- [x] V2 Generate fresh downstream, compatibility, and dependency-boundary evidence for protocol/wire. [covers=architecture-modularity.protocol-wire-readiness.runtime-boundary]
  - Evidence: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/Cargo.toml`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/Cargo.lock`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/src/lib.rs`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-test.txt`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-metadata.json`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-forbidden-grep.txt`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i3-client-api-compatibility-tests.txt`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-readiness.md`

## Verification Commands

### `CARGO_TARGET_DIR=/tmp/aspen-protocol-wire-checks cargo check -p aspen-client-api && ...`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-cargo-checks.txt`

### `CARGO_TARGET_DIR=/tmp/aspen-protocol-wire-client-test cargo test -p aspen-client-api`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i3-client-api-compatibility-tests.txt`

### `CARGO_TARGET_DIR=/tmp/aspen-protocol-wire-fixture cargo test --manifest-path openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/Cargo.toml`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-test.txt`

### `cargo metadata --manifest-path openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/Cargo.toml --format-version 1`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-metadata.json`

### `fixture source and protocol cargo tree negative boundary scan`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-forbidden-grep.txt`

### `scripts/check-crate-extraction-readiness.rs --candidate-family protocol-wire ...`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-readiness.md`
- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-readiness.json`

### `openspec validate architecture-modularity --json`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-validate.json`

### `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify promote-protocol-wire-readiness --json`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/helper-verify.json`

### `git diff --cached --check`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/git-diff-check.txt`

### `scripts/openspec-preflight.sh openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-preflight.txt`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | `openspec validate architecture-modularity --json` and protocol `cargo check` commands | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-validate.json`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-cargo-checks.txt` | Validates the active OpenSpec package and checks reusable protocol crate feature sets. | Re-run after archive as `openspec validate architecture-modularity --json`. |
| test | `cargo test -p aspen-client-api` and downstream fixture `cargo test --manifest-path ...` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i3-client-api-compatibility-tests.txt`, `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-test.txt` | Exercises client wire compatibility and downstream canonical protocol imports. | Re-run tests if protocol schema files change. |
| format | `git diff --cached --check` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/git-diff-check.txt` | This slice changes docs, Nickel policy, fixture code, and evidence; whitespace diff-check is the relevant formatting gate before commit. | Re-run before commit after final evidence regeneration. |
| helper verify | `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify promote-protocol-wire-readiness --json` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/helper-verify.json` | Confirms task/spec mechanics before archive. | Drain audit after archive. |
| preflight | `scripts/openspec-preflight.sh openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-preflight.txt` | Ensures checked tasks have durable evidence and changed-file entries. | Re-run after staging final evidence. |

## Verification Matrix

| Rail | Command | Status | Artifact |
| --- | --- | --- | --- |
| protocol checks | `cargo check` default/no-default protocol crates | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-cargo-checks.txt` |
| compatibility tests | `CARGO_TARGET_DIR=/tmp/aspen-protocol-wire-client-test cargo test -p aspen-client-api` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i3-client-api-compatibility-tests.txt` |
| downstream fixture | `CARGO_TARGET_DIR=/tmp/aspen-protocol-wire-fixture cargo test --manifest-path openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/Cargo.toml` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-test.txt` |
| metadata | `cargo metadata --manifest-path openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/fixtures/downstream-protocol-wire/Cargo.toml --format-version 1` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-metadata.json` |
| negative boundary | fixture source and protocol `cargo tree -e normal` scan | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/i5-downstream-protocol-wire-forbidden-grep.txt` |
| readiness checker | `scripts/check-crate-extraction-readiness.rs --candidate-family protocol-wire ...` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/protocol-wire-readiness.md` |

| OpenSpec validate | `openspec validate architecture-modularity --json` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-validate.json` |
| helper verify | `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify promote-protocol-wire-readiness --json` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/helper-verify.json` |
| implementation diff | `git diff --cached -- ...` | captured | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/implementation-diff.patch` |
| preflight | `scripts/openspec-preflight.sh openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness` | pass | `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-preflight.txt` |

### `scripts/openspec-drain-audit.sh --archive openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness`

- Artifact: `openspec/changes/archive/2026-05-01-promote-protocol-wire-readiness/evidence/openspec-drain-audit.txt`
