# Token and Ticket Debug Redaction Evidence

Generated: 2026-05-05T02:20:02Z

## Scope

Focused Phase 4 remediation for debug/output surfaces that can carry token, ticket, or secret-like material. The slice used synthetic fixtures only.

Audited and hardened source handles:

- `crates/aspen-auth-core/src/token.rs`
  - `CapabilityToken` no longer derives `Debug`.
  - Custom `Debug` keeps routing/audit metadata but redacts reconstructable token material: nonce presence only, proof presence only, facts count only, signature placeholder.
  - Regression: `capability_token_debug_redacts_reconstructable_material`.
- `crates/aspen-client/src/ticket.rs`
  - `AspenClientTicket` no longer derives `Debug`.
  - Custom `Debug` reports `has_auth_token` without printing optional auth-token bytes.
  - Regression: `client_ticket_debug_redacts_auth_token_bytes`.
- `crates/aspen-ticket/src/signed.rs`
  - `SignedAspenClusterTicket` no longer derives `Debug`.
  - Custom `Debug` reports issuer, timestamps, cluster id, and bootstrap peer count while redacting nonce and signature material.
  - Regression: `signed_ticket_debug_redacts_replay_and_signature_material`.

## Finding

Derived `Debug` for these security-sensitive structures could print raw synthetic bytes for:

- token `nonce`, `proof`, `facts`, and `signature` fields;
- client ticket `auth_token` bytes;
- signed ticket `nonce` and `signature` fields.

Even when fields are not long-lived credentials, debug output can be copied into logs, receipts, errors, or test failures. This slice removes the derived debug path and adds exact negative regressions.

## Verification

Focused tests passed:

```sh
rustfmt crates/aspen-auth-core/src/token.rs crates/aspen-client/src/ticket.rs crates/aspen-ticket/src/signed.rs
cargo test -p aspen-auth-core capability_token_debug_redacts_reconstructable_material -- --nocapture
cargo test -p aspen-client client_ticket_debug_redacts_auth_token_bytes -- --nocapture
cargo test -p aspen-ticket signed_ticket_debug_redacts_replay_and_signature_material -- --nocapture
cargo check -p aspen-auth-core -p aspen-client -p aspen-ticket
scripts/tigerstyle-check.sh
openspec validate full-aspen-hardening-audit --strict --json
python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify full-aspen-hardening-audit --json || true
git diff --check
python -m json.tool openspec/changes/full-aspen-hardening-audit/evidence/token-ticket-redaction.json >/dev/null
```

Observed result: focused redaction tests, touched-crate check, Tiger Style, strict OpenSpec validation, diff whitespace check, and JSON validation passed. The OpenSpec helper reported only the expected incomplete-umbrella warning (`11 done / 11 todo`).

## Non-goals

- This evidence does not claim the whole Phase 4 token/ticket persistence audit is complete.
- Serialization and round-trip behavior intentionally remain unchanged; the redaction boundary is human-readable `Debug` output.
