## Why

Independent audits of open issues found locally reachable gaps in documentation and repository tooling:

- #27: `tools/check-license-boundary.rs` accepts a selected dependency revision found *anywhere* in `THIRD_PARTY_LICENSES.md`, so a historical fixture row can satisfy a stale current-dependency row; its self-tests do not cover the two mismatches the issue observed. The check also ran only inside the manual `dependency-audit` app, and its `contains` marker for `docs/distributed-system-fabric.md` still named "Molten" after the public rename, so the checker failed on the current tree.
- #29: `docs/source-sensitive-pages.md` points its "README first workflow" row at `docs/README.md`, not the root README, and no CI check validates documentation links or the README quick-start commands.
- #4: the atlas links its answered rate-limit question only to a review page, its edges carry no packet payloads, the HTML twin has no hover/pin detail, the `world` component cites a nonexistent `src/world_commit/` path, and the `world → prolly` edge is not backed by source (no world module uses the Prolly map).
- #30: SECURITY.md cites a 2026-11-13 verification of the private reporting channel that is not retained, and the pilot tracker still says maintainer verification is needed.
- #2: `docs/deferred-decisions.md` says committed events lack a cause field, but `TurnCommitted` now carries one.
- #10: the deferred decision lacks the requested need assessment.
- #25 AC5: the README names cross-host limits without the owning issue links.

## What Changes

- Notice revisions bind to their own row: each `notice-revision` rule requires the revision `Cargo.toml` selects for that dependency to appear in the single `Cargo packages` row naming it; conflicting selections fail; self-tests cover the two observed mismatches and an unchanged historical fixture keeping its older revision. Policy rules cover all ten directly selected Git dependencies with notice rows, and the renamed documentation marker is corrected. A `license-boundary` flake check compiles and runs the checker. r[molten.project.license_boundary.notice_revisions]
- A stdlib `scripts/check-doc-links.py` validates local links and anchors in `README.md`, `CONTRIBUTING.md`, `SECURITY.md` and `docs/**/*.md`, skipping external URLs, with fixtures for valid and broken links; the `doc-links` flake check runs it and its tests. r[molten.project.documentation_links.local_targets]
- `scripts/readme-quickstart.py` extracts the README "Getting started" fixture and cleanup blocks, substitutes the packaged `molten` binary for `cargo run --locked --`, runs them in a fresh directory and requires the results the README states; the `readme-quickstart` flake check runs it against the package that existing checks already build. r[molten.project.documentation_links.quickstart_commands]
- Atlas edges name their source payload type and fields; answered questions must link the issue or Cairn change that answered them; the HTML twin shows payloads on hover/focus with a pin toggle; the `world` source path and edge are corrected. r[molten.project.system_atlas.payloads_and_tracking]
- Documentation corrections for #2, #10, #25 and #30, with a retained read-only private-reporting observation.

## Impact

- **Files**: `tools/check-license-boundary.rs`, `license-policy.tsv`, `scripts/{check-doc-links,readme-quickstart,generate-atlas}.py` and their tests, `tests/fixtures/doc-links/`, `flake.nix`, `docs/{atlas.ncl,atlas.md,atlas.html,source-sensitive-pages.md,deferred-decisions.md,external-pilot-tracker.md,release-status.md,open-issue-triage.md}`, `README.md`, `SECURITY.md`.
- **Testing**: checker self-tests and a mutated-tree negative run; Python unit tests for the link checker, quick-start runner and atlas generator; the touched flake checks.
- **Non-goals**: no relicensing or legal conclusion; no semantic review implied by a link pass; no verbatim cold `cargo run` quick-start execution; no tree-clock experiment or discovery provider; no vulnerability report is submitted; archived material is not rewritten.
