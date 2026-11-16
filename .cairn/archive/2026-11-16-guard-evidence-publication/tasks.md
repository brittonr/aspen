## Phase 1: Implementation

- [x] [serial] Add the fail-closed publication scanner with home, scratch, `.local`/observed host, RFC 1918, CGNAT/tailnet and opaque-container rules, docs path-only scope, the hosted-runner exclusion, redaction placeholders, the reviewed allowlist and docs inline marker. r[molten.project.evidence_publication.host_material]
- [x] [serial] Generate the BLAKE3-bound baseline from `git ls-files` (1,068 files: 636 `evidence/`, 425 `.cairn/archive/`, 4 `.cairn/changes/`, 3 `docs/`); make regeneration keep only unchanged or moved-identical rows and make `--all` reject stale rows. r[molten.project.evidence_publication.host_material]
- [x] [serial] Wire the `evidence-publication-guard` pre-commit hook and the `evidence-publication` flake check; add `b3sum` to the development shell. r[molten.project.evidence_publication.host_material]
- [x] [serial] Record the 88 historical epoch-dated archives and add the `epoch-archive-dates` flake check. r[molten.project.evidence_publication.archive_dates]
- [x] [serial] Document redaction and the guard in CONTRIBUTING.md, SECURITY.md and the #52 entry of `docs/deferred-decisions.md`. r[molten.project.evidence_publication.host_material] r[molten.project.evidence_publication.archive_dates]

## Phase 2: Verification

- [x] [serial] Cover new, redacted, grandfathered, one-byte-edited grandfathered, moved-bytes, allowlisted and unreviewed docs, inline marker outside docs, scope, hosted-runner, decimal-versus-IPv4, container, stale-baseline and fail-closed cases. r[molten.project.evidence_publication.host_material]
- [x] [serial] Run the whole-tree scan, both flake checks, a negative build of each check, and the pre-commit hook on an offending and a redacted file. r[molten.project.evidence_publication.host_material] r[molten.project.evidence_publication.archive_dates]

## Verification (2026-11-16)

Worktree at `fcbf60a3d` plus this change: `python3 scripts/test_evidence_publication_guard.py` (16 passed); `python3 scripts/evidence-publication-guard.py --all` (4,286 scanned, 1,068 grandfathered, 0 rejected, 0 stale); `nix build --no-link --option builders '' path:<worktree>#checks.x86_64-linux.{evidence-publication,epoch-archive-dates}` (both built). Negative builds of both checks with `overrideAttrs { src = <scratch copy>; }`: an added evidence file with a home path, `aspen1.local`, `100.100.7.9` and `192.168.4.2` failed with four `path:line:column` findings; an added `1970-01-01-new-unrecorded` archive and a renamed recorded archive failed `epoch-archive-dates`, naming both. Pre-commit 4.5.1 (`nix shell --inputs-from . nixpkgs#pre-commit`): `pre-commit run evidence-publication-guard --files <probe>` failed on the offending fixture copy and passed on the redacted copy, and passed on a grandfathered doc and container. A scratch git repository exercised `--write-baseline`: bootstrap, an archived-change move kept its row, and a new offending file was refused. `tools/tracey/inherited_debt_guard.rs` passed (0 dangling).
