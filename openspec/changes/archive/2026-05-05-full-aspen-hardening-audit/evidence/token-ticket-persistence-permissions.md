# Phase 4 Token/Ticket Persistence and Permission Audit

Generated: 2026-05-05T02:30:22Z

## Scope

Audited credential-bearing token/ticket generation and persistence paths using source inspection only. No live credential files were read and no credential values were preserved.

## Source handles

- `crates/aspen-cli/src/bin/aspen-cli/commands/token.rs` — capability token generation/delegation emits bearer token text to stdout; inspection output keeps metadata only.
- `crates/aspen-cluster/src/endpoint_manager.rs` — node secret-key persistence uses `OpenOptionsExt::mode(0o600)` and writes with restrictive permissions.
- `crates/aspen-docs/src/store/initialization.rs` — persistent iroh-docs namespace/author secrets generated under the docs store.
- `crates/aspen-hooks-ticket/src/lib.rs` — hook trigger ticket struct can contain default payload and optional auth token.
- `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs` — hook URL creation serializes the hook ticket for explicit operator output.

## Findings and remediation

1. Persistent docs namespace/author secrets were generated via plain `std::fs::write`, inheriting the process umask and not repairing an existing wide-mode file. This was remediated by adding `write_secret_hex_file`, which uses `OpenOptionsExt::mode(0o600)` on Unix and then explicitly sets the resulting file mode to `0600` after writing. Regression tests cover new files and existing wide-mode files.
2. `AspenHookTicket` derived `Debug`, exposing optional auth-token bytes and default payload templates if a ticket was accidentally logged or included in test output. This was remediated with a custom `Debug` implementation that exposes only safe metadata (`cluster_id`, event type, counts, booleans, expiry, priority) and redacts payload/auth-token/relay URL material. A regression uses synthetic markers only.
3. Token generation/delegation CLI output intentionally emits the token as the command product; no file persistence path was found in `token.rs`. Token inspection output renders metadata and boolean nonce/proof presence, not token bytes.
4. Cluster endpoint secret-key persistence already uses owner-only permissions.

## Verification commands

- `rustfmt crates/aspen-hooks-ticket/src/lib.rs crates/aspen-docs/src/store/initialization.rs crates/aspen-docs/src/importer.rs`
- `cargo test -p aspen-hooks-ticket hook_ticket_debug_redacts_payload_and_auth_token -- --nocapture`
- `cargo test -p aspen-docs generated_persistent -- --nocapture`

## Secret handling

All fixtures use synthetic marker strings or repeated bytes. Evidence intentionally names credential fields but contains no credential values.
