## Why

Issue #52: tracked evidence and archived change evidence carry home paths, the host name `aspen1.local`, scratch paths and tailnet addresses. The earlier #52 decision keeps that history unchanged (it is already public and hash-bound, and `AGENTS.md` preserves archives and historical evidence), but nothing stops new evidence from repeating it. Epoch-dated archives are rejected only by a local pre-commit hook, not in CI.

## What Changes

- A fail-closed scanner, `scripts/evidence-publication-guard.py`, rejects host-specific material in new or modified files under `evidence/`, `release/`, change and archive evidence, and home or scratch paths under `docs/`, with file/line diagnostics and redaction placeholders.
- Files that held such material at `fcbf60a3d` are grandfathered by a committed, sorted baseline binding each path to its BLAKE3; any edit drops the exemption. Reviewed exemptions use an allowlist row or, in docs, an inline marker.
- A pre-commit hook checks staged files; the `evidence-publication` flake check runs the scanner's fixture tests and the whole-tree scan.
- The `epoch-archive-dates` flake check rejects any `1970-01-01-*` archive outside the 88 recorded historical names.
- CONTRIBUTING.md, SECURITY.md and the #52 entry in `docs/deferred-decisions.md` document redaction and the guard.

## Impact

- **Files**: the scanner and its tests, fixtures under `tests/fixtures/evidence-publication/`, `tools/repository-hygiene/` (baseline, allowlist, epoch archive list), `.pre-commit-config.yaml`, `flake.nix`, and the three documents.
- **Testing**: unit tests over the fixture root (new, redacted, grandfathered, edited grandfathered, allowlisted, scope, container, stale-baseline and fail-closed cases); whole-tree scan; both flake checks, each with a negative run.
- **Non-goals**: no existing evidence, archive or the root zip is rewritten, moved or deleted; Cairn's own `archive_date()` fallback is not changed here.
