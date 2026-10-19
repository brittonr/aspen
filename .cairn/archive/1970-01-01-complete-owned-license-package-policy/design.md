## Context

Cargo.toml lists molten, molten-core, molten-node-host, and molten-release-policy as workspace members. Each inherits AGPL-3.0-or-later. license-policy.tsv has a strict workspace-package-set rule but omits molten-node-host. tools/check-license-boundary.rs validates workspace coverage before reading individual rule artifacts and reports a missing package rule deterministically.

## Decisions

### Decision: Complete the existing policy rather than weaken its checker

**Choice:** Add one package rule for crates/molten-node-host/Cargo.toml in workspace order. Keep the checker and all package metadata unchanged. Add explicit complete-workspace coverage to r[molten.project.license_boundary.final_validation].

**Rationale:** The existing coverage guard catches plausible license drift in a newly extracted owned package. Removing strictness, ignoring the missing member, or scanning dependency metadata as if it were owned would conceal rather than repair the defect. The ordered TSV row has no runtime allocation or execution effect.

### Decision: Preserve grant history and acquisition boundaries

**Choice:** Update only existing README licensing prose to name owned cores, contracts, schemas, fixtures, templates, documentation, and policy; record complete workspace enforcement and unchanged prior grants. Preserve third-party terms and exact dependency-pin license assertions, including vm-cohort-core's historical MIT OR Apache-2.0 snapshot and the pinned Artifact license receipts.

**Rationale:** The existing root grant already covers owned source; this change closes an enforcement omission, not a dependency relicensing or publication event. Current authenticated SSH and private Radicle source coordinates remain untouched. A declaration does not make those exact source revisions anonymously obtainable.

## Verification

The parent runs tools/check-license-boundary.rs against this worktree after all source preparation. Its existing self-check covers omitted workspace package rules and explicit/inherited expressions. No new permanent test or checker modification is needed. The parent also performs final Cairn validation and review before any sync/archive claim. This worker performs only the required preimplementation lifecycle gates; builds, tests, lint, formatters, and broad validation are explicitly excluded.

## Risks / Trade-offs

- Complete workspace coverage does not enumerate every standalone check or pilot crate, but inspected owned check/pilot grants already select AGPL and remain covered by the root owned-source declaration.
- Earlier permissive grants remain irrevocable for previously granted bytes; current metadata cannot revoke them.
- Source-acquisition credentials, unavailable revisions, runtime behavior, legal compliance, and release eligibility are outside the proof of a passing license-boundary audit.
