# Phase 4 CLI/log/output redaction audit

Generated: 2026-05-05T02:41:00Z

## Scope

Audited representative operator-facing output surfaces for credential leakage using source inspection and synthetic secret-like fixtures only. No live cluster tickets, auth tokens, private keys, passwords, or connection strings were read.

Covered surfaces:

- CLI/web operator output: `crates/aspen-forge-web/src/templates.rs`, `crates/aspen-forge-web/src/routes.rs`, `crates/aspen-fuse/src/main.rs`, `crates/aspen-fuse/src/bin/aspen-cluster-virtiofs-server.rs`, `crates/aspen-fuse/src/bin/aspen-ci-workspace-server.rs`.
- Logs/errors: `crates/aspen-dogfood/src/{cluster,forge,ci,deploy,error,main}.rs`, `crates/aspen-client/src/watch.rs`, `crates/aspen-raft/src/log_subscriber.rs`, `crates/aspen-transport/src/log_subscriber/connection.rs`.
- Test/debug fixtures: existing Phase 4 redaction tests for capability tokens, client tickets, signed cluster tickets, and hook tickets.
- Docs/evidence: OpenSpec evidence and Markdown references were searched for credential-bearing terms; matches were contextual and did not preserve credential values.

## Finding and remediation

`aspen-forge-web` rendered the live `RepoOverviewParams::ticket` value directly into the repository clone command:

- before: `git clone aspen://{ticket}/{repo_id} {repo_name}`
- after: `git clone aspen://<cluster-ticket>/{repo_id} {repo_name}`

Repository overview pages are shareable/screenshot-prone operator output, so the rendered clone command now preserves the command shape while replacing the credential-bearing cluster ticket with a placeholder. A regression uses the synthetic marker `aspen-ticket-synthetic-secret-marker-0123456789` and asserts the marker is absent from rendered HTML while the placeholder remains.

## Other audited observations

- Dogfood error/receipt paths already route ticket labels through `ticket_preview`, which emits `[REDACTED ticket; bytes=N]` and has tests for both short and long synthetic tickets.
- Fuse cluster-ticket command paths log only `ticket_bytes` / `ticket_len`, and source tests guard against logging the full argument value.
- Dogfood `git` remote URLs still contain tickets internally to perform clone/push operations, but inspected error/log paths use redacted `ticket_preview` labels. The dedicated dogfood/CI/deploy receipt task remains open for a deeper artifact/run identifier audit.
- Log-subscriber auth paths include challenge/result diagnostics but do not include cluster tickets or token strings in the inspected messages.

## Verification commands

- `rustfmt crates/aspen-forge-web/src/templates.rs crates/aspen-forge-web/src/routes.rs`
- `cargo test -p aspen-forge-web repo_overview_clone_command_redacts_ticket_value -- --nocapture`
- `search_files` source scans for `aspen://{...ticket}`, `ticket_preview`, `ticket_len`, `ticket_bytes`, `auth_token: Some`, auth-result debug formatting, and synthetic markers.

## Credential handling

All fixtures were synthetic. Evidence records source paths, command names, and byte-length/redaction placeholders only; it does not include credential values.
