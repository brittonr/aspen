# Verification: review-foundational-types-public-api

## Implementation Evidence

- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/foundational-types.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `openspec/changes/review-foundational-types-public-api/fixtures/downstream-foundational-types/Cargo.toml`
- Changed file: `openspec/changes/review-foundational-types-public-api/fixtures/downstream-foundational-types/src/lib.rs`
- Changed file: `openspec/changes/review-foundational-types-public-api/specs/foundational-types-extraction/spec.md`
- Changed file: `openspec/changes/review-foundational-types-public-api/proposal.md`
- Changed file: `openspec/changes/review-foundational-types-public-api/design.md`
- Changed file: `openspec/changes/review-foundational-types-public-api/tasks.md`
- Changed file: `openspec/changes/review-foundational-types-public-api/verification.md`

- Public API ownership and canonical imports are recorded in `docs/crate-extraction/foundational-types.md`.
- The extraction inventory and Nickel policy promote the foundational family to `extraction-ready-in-workspace` while keeping publishable/repo-split states blocked on license/publication policy.
- `fixtures/downstream-foundational-types` is an independent downstream fixture that imports foundational crates directly with `default-features = false` and no root `aspen` dependency.
- Evidence files under `evidence/` cover downstream metadata, forbidden runtime dependency boundary, compatibility checks, and readiness checker output.

## Task Coverage

- [x] Create proposal, design, delta spec, and task rail for `review-foundational-types-public-api`.
  - Evidence: `openspec/changes/review-foundational-types-public-api/proposal.md`
  - Evidence: `openspec/changes/review-foundational-types-public-api/design.md`
  - Evidence: `openspec/changes/review-foundational-types-public-api/specs/foundational-types-extraction/spec.md`
  - Evidence: `openspec/changes/review-foundational-types-public-api/tasks.md`
- [x] Inventory public APIs and compatibility shims for all foundational crates.
  - Evidence: `docs/crate-extraction/foundational-types.md`
  - Evidence: `docs/crate-extraction.md`
  - Evidence: `docs/crate-extraction/policy.ncl`
- [x] Add or update downstream fixture and negative boundary checks for the reviewed surface.
  - Evidence: `openspec/changes/review-foundational-types-public-api/fixtures/downstream-foundational-types/Cargo.toml`
  - Evidence: `openspec/changes/review-foundational-types-public-api/fixtures/downstream-foundational-types/src/lib.rs`
  - Evidence: `openspec/changes/review-foundational-types-public-api/evidence/foundational-types-downstream-metadata.json`
  - Evidence: `openspec/changes/review-foundational-types-public-api/evidence/foundational-types-forbidden-boundary.txt`
- [x] Run no-std, extraction-readiness, and representative consumer checks; update manifests/docs with raise/no-raise evidence.
  - Evidence: `openspec/changes/review-foundational-types-public-api/evidence/foundational-types-compatibility.txt`
  - Evidence: `openspec/changes/review-foundational-types-public-api/evidence/foundational-types-readiness.md`
  - Evidence: `openspec/changes/review-foundational-types-public-api/evidence/foundational-types-readiness.json`
- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
  - Evidence: `openspec/changes/review-foundational-types-public-api/verification.md`
  - Evidence: `openspec/changes/review-foundational-types-public-api/evidence/foundational-types-readiness.md`
- [x] Sync/archive only after every implementation/evidence task is complete.
  - Evidence: `openspec/changes/review-foundational-types-public-api/tasks.md`
  - Evidence: `openspec/changes/review-foundational-types-public-api/verification.md`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | `scripts/check-crate-extraction-readiness.rs --candidate-family foundational-types ...` | pass | `evidence/foundational-types-readiness.md` | Covers policy/inventory/docs readiness contract. | Full `nix flake check` remains broader than this public API drain. |
| test | `cargo test --manifest-path openspec/changes/review-foundational-types-public-api/fixtures/downstream-foundational-types/Cargo.toml` | pass | `evidence/foundational-types-compatibility.txt` | Exercises direct downstream imports with no root Aspen dependency. | Add external published-crate fixture after license policy. |
| format | `nix run .#rustfmt` and `git diff --check` | pass | closeout transcript | Rust fixture and markdown/TOML diffs are formatting-clean. | Repo-wide lint is outside this docs/evidence slice. |

## Verification Commands

- Artifact: `openspec/changes/review-foundational-types-public-api/evidence/foundational-types-compatibility.txt`
- Artifact: `openspec/changes/review-foundational-types-public-api/evidence/foundational-types-forbidden-boundary.txt`
- Artifact: `openspec/changes/review-foundational-types-public-api/evidence/foundational-types-readiness.md`

```bash
cargo test --manifest-path openspec/changes/review-foundational-types-public-api/fixtures/downstream-foundational-types/Cargo.toml
cargo metadata --manifest-path openspec/changes/review-foundational-types-public-api/fixtures/downstream-foundational-types/Cargo.toml --format-version 1 > openspec/changes/review-foundational-types-public-api/evidence/foundational-types-downstream-metadata.json
cargo tree --manifest-path openspec/changes/review-foundational-types-public-api/fixtures/downstream-foundational-types/Cargo.toml -e normal
cargo check -p aspen-storage-types --no-default-features
cargo check -p aspen-traits --no-default-features
cargo check -p aspen-cluster-types --no-default-features
cargo check -p aspen-hlc --no-default-features
cargo check -p aspen-time --no-default-features
cargo check -p aspen-constants --no-default-features
python scripts/check-aspen-core-no-std-boundary.py --manifest-path crates/aspen-core/Cargo.toml --allowlist scripts/aspen-core-no-std-transitives.txt --output /tmp/aspen-core-no-std-current.txt --diff-output /tmp/aspen-core-no-std-diff.txt
scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family foundational-types --output-json openspec/changes/review-foundational-types-public-api/evidence/foundational-types-readiness.json --output-markdown openspec/changes/review-foundational-types-public-api/evidence/foundational-types-readiness.md
openspec validate review-foundational-types-public-api --strict
scripts/openspec-preflight.sh review-foundational-types-public-api
git diff --check
```
