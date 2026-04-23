# Verification

## Task Coverage

- [x] I1 Inventory the initial rollout scope for enforced lint policy, naming every crate root that will adopt the deny baseline in this change, naming deferred crates with reasons plus owners or follow-up change references, and capturing a baseline `cargo clippy` result for that scope before edits. [covers=tigerstyle-rollout.clippy-policy-rollout-scope-is-explicit,core.enforced-clippy-policy-verification-is-reviewable]
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/rollout-inventory.md`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/rollout-candidate-crates.tsv`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/aspen-time-baseline.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/canonical-clippy.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/notes.txt`

- [x] I2 Add a repository-root `clippy.toml` with the checked-in exported-API and complexity thresholds (`avoid-breaking-exported-api = false`, `cognitive-complexity-threshold = 15`, `too-many-arguments-threshold = 5`, `type-complexity-threshold = 200`) and treat those thresholds as deny-level policy for the rollout scope. [covers=core.repository-clippy-thresholds-travel-with-the-checkout,core.rollout-scope-warnings-and-threshold-overruns-fail]
- Evidence: `clippy.toml`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/source-diff.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/canonical-clippy.txt`

- [x] I3 Add the standardized crate-root lint policy block to every crate in the documented rollout scope, denying `missing_docs`, `clippy::all`, `clippy::pedantic`, and `clippy::undocumented_unsafe_blocks` while explicitly allowing `clippy::module_name_repetitions`, and keep the rollout-scope block text uniform unless a documented exception says otherwise. [covers=core.crate-roots-enforce-clippy-as-policy,core.rollout-scope-crate-policy-text-stays-uniform]
- Evidence: `crates/aspen-time/src/lib.rs`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/source-diff.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/rollout-inventory.md`

- [x] I4 Define and wire the rollout-scope rustdoc enforcement path so warnings become errors for the same crates, and document the canonical local/CI enforcement command semantics for that scope. [covers=core.rustdoc-warnings-are-enforced-as-policy,core.local-and-ci-lint-semantics-stay-identical,core.rollout-scope-warnings-and-threshold-overruns-fail]
- Evidence: `scripts/clippy-policy.sh`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/canonical-rustdoc.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/local-ci-parity.md`

- [x] I5 Add one canonical checked-in lint entrypoint for the rollout scope and ensure contributor guidance plus CI enforcement use that entrypoint or the same semantics it defines, without relying on ambient per-user setup. [covers=core.canonical-lint-entrypoint-is-checked-in-and-shared,core.canonical-lint-entrypoint-has-no-ambient-setup-assumptions]
- Evidence: `scripts/clippy-policy.sh`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/canonical-entrypoint.md`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/local-ci-parity.md`

- [x] I6 Constrain additional lint suppressions in the rollout scope so non-standard `#[allow(...)]` usage is item-scoped by default, broader only when justified, and never added as broad crate-wide escape hatches without explicit exception approval. [covers=core.additional-lint-exceptions-stay-narrow-and-justified,core.item-scoped-lint-allows-are-the-default]
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/lint-suppression-policy.md`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/source-diff.txt`

- [x] I7 Inventory rollout-scope `unsafe` sites, where present, and ensure the enforced policy plus saved evidence prove each site remains documented and reviewable. [covers=core.rollout-scope-unsafe-sites-are-inventoried-and-documented]
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/unsafe-inventory.md`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/unsafe-grep.txt`

- [x] V1 Save durable evidence under `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/` for the rollout inventory, deferred-crate list with owners or follow-up references, the baseline scope `cargo clippy` command, the final passing `cargo clippy` command for the same scope, rustdoc warning enforcement evidence for the same scope, local/CI command-parity evidence, canonical-entrypoint evidence, negative-proof evidence showing a representative violation fails under the canonical path, unsafe inventory evidence where applicable, a source diff artifact showing `clippy.toml` plus crate-root policy blocks, and `scripts/openspec-preflight.sh enforce-clippy-policy`; then map every checked task verbatim in `verification.md`. [covers=tigerstyle-rollout.clippy-policy-rollout-scope-is-explicit,core.crate-roots-enforce-clippy-as-policy,core.repository-clippy-thresholds-travel-with-the-checkout,core.rollout-scope-warnings-and-threshold-overruns-fail,core.rustdoc-warnings-are-enforced-as-policy,core.local-and-ci-lint-semantics-stay-identical,core.canonical-lint-entrypoint-is-checked-in-and-shared,core.canonical-lint-entrypoint-has-no-ambient-setup-assumptions,core.additional-lint-exceptions-stay-narrow-and-justified,core.item-scoped-lint-allows-are-the-default,core.rollout-scope-crate-policy-text-stays-uniform,core.rollout-scope-unsafe-sites-are-inventoried-and-documented,core.negative-proof-shows-enforcement-bites,core.enforced-clippy-policy-verification-is-reviewable]
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/rollout-inventory.md`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/aspen-time-baseline.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/canonical-clippy.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/canonical-rustdoc.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/local-ci-parity.md`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/canonical-entrypoint.md`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/negative-proof.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/unsafe-inventory.md`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/source-diff.txt`
- Evidence: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/openspec-preflight.txt`

## Verification Commands

- Command: `./scripts/clippy-policy.sh clippy`
  - Artifact: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/canonical-clippy.txt`
- Command: `./scripts/clippy-policy.sh rustdoc`
  - Artifact: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/canonical-rustdoc.txt`
- Command: `TMPDIR=/run/user/$UID/tmp env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo clippy --manifest-path openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/negative-proof-module/Cargo.toml -- -D warnings`
  - Artifact: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/negative-proof.txt`
- Command: `scripts/openspec-preflight.sh enforce-clippy-policy`
  - Artifact: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/openspec-preflight.txt`

## Changed file evidence

- Changed file: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/tasks.md`
- Changed file: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/verification.md`
- Changed file: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/source-diff.txt`
- Changed file: `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/openspec-preflight.txt`
