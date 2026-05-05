# Dogfood receipt and diagnostic redaction audit

Generated: 2026-05-05T10:06:08Z

## Scope

This slice audited Aspen dogfood receipt and diagnostic paths that operators use after CI, deploy, Forge push, and receipt publication failures. The audit used synthetic marker fixtures only and did not read or preserve live credential-bearing files.

Source handles reviewed:

- `crates/aspen-dogfood/src/receipt.rs` — canonical receipt schema and artifact fields.
- `crates/aspen-dogfood/src/main.rs` — receipt writer, receipt publication, `receipts show`, `receipts diagnose`, CI run artifact, cluster receipt artifact, and failure summary construction.
- `crates/aspen-dogfood/src/forge.rs` — Forge remote URL construction and git push stdout/stderr capture.
- `crates/aspen-dogfood/src/ci.rs` — CI run polling and run-id artifact usage.
- `crates/aspen-dogfood/src/deploy.rs` — deploy status polling and failure classification.
- `crates/aspen-dogfood/src/cluster.rs` and `crates/aspen-dogfood/src/federation.rs` — ticket preview use around client RPC error targets and federated clone URLs.

## Findings

Receipt artifacts already record operator-safe identifiers for follow-up:

- CI build stage artifacts use `DogfoodArtifactKind::CiRun` with `store_id` set to the CI run id via `ci_run_artifact`.
- Published receipt artifacts use `DogfoodArtifactKind::Receipt` with `store_id` set to the cluster receipt key via `cluster_receipt_artifact`.
- The receipt schema stores artifact kind/name/store id/blob id/digest/size/path metadata rather than scraping node logs.
- `receipts diagnose` points operators at the CI run artifact and receipt commands instead of requiring broad log capture.

Concrete leakage risks found and remediated:

1. `cmd_start_single` logged the first bytes of a live cluster ticket. It now logs the existing redacted ticket preview (`[REDACTED ticket; bytes=N]`) instead.
2. Git push failures can include `aspen://.../...` remotes in git stdout/stderr. A shared dogfood redactor now replaces the credential segment with `<cluster-ticket>` while preserving the repo/run identifier path for diagnosis.
3. Dogfood failure summaries are persisted into receipts. They now sanitize error strings before storage.
4. `receipts show` and `receipts diagnose` now sanitize persisted failure messages before printing, covering older or externally produced receipts that may contain raw `aspen://.../...` values.

## Regression coverage

Synthetic marker pattern used in tests: `synthetic-dogfood-ticket-marker-0123456789`.

Added regressions:

- `error::tests::redacts_aspen_remote_credentials_without_removing_identifiers`
- `error::tests::redacts_trailing_aspen_remote_credentials`
- `tests::dogfood_failure_summary_redacts_aspen_remote_credentials`
- `tests::diagnose_receipt_redacts_legacy_failure_messages`

Expected behavior:

- The synthetic marker and marker prefix are absent from redacted output.
- Operator-safe identifiers remain visible, e.g. `aspen://<cluster-ticket>/repo-123` and `aspen://<cluster-ticket>/fed:node:repo-123`.
- CI run ids and cluster receipt keys remain as artifact identifiers in receipts.

## Verification

Commands run:

```sh
rustfmt crates/aspen-dogfood/src/error.rs crates/aspen-dogfood/src/main.rs crates/aspen-dogfood/src/forge.rs
cargo test -p aspen-dogfood credentials -- --nocapture
cargo test -p aspen-dogfood diagnose_receipt_redacts_legacy_failure_messages -- --nocapture
python -m json.tool openspec/changes/full-aspen-hardening-audit/evidence/dogfood-receipt-redaction.json >/dev/null
cargo check -p aspen-dogfood
scripts/tigerstyle-check.sh
openspec validate full-aspen-hardening-audit --strict --json
python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify full-aspen-hardening-audit --json || true
git diff --check
```

OpenSpec helper verification reported the expected active-umbrella warning after this slice: `tasks incomplete: {'done': 14, 'todo': 8, 'in_progress': 0}`.
