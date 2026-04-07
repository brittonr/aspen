## Why

Several ticket and identity serialization paths silently replace serialization failures with empty byte vectors. That hides the real error and can produce shareable strings that look valid but cannot be used.

The clearest reachable case is `AutomergeSyncTicket`:

- `crates/aspen-automerge/src/ticket.rs:35` uses `token.encode().unwrap_or_default()`
- `crates/aspen-automerge/src/ticket.rs:42` uses `postcard::to_stdvec(self).unwrap_or_default()`

`CapabilityToken::encode()` is explicitly fallible: it returns `AuthError::TokenTooLarge` when the encoded token exceeds `MAX_TOKEN_SIZE`, and it can also fail on postcard encoding errors. Today `AutomergeSyncTicket::new()` drops that failure, stores an empty token, and still returns a ticket object.

The same fallback pattern appears in other ticket-like types:

- `crates/aspen-client/src/ticket.rs:104`
- `crates/aspen-ticket/src/signed.rs:260`
- `crates/aspen-federation/src/identity.rs:190`

## What We Know

- `AutomergeSyncTicket::new()` has no error return, so callers cannot tell when token encoding failed.
- `AutomergeSyncTicket::serialize()` returns a string even if postcard serialization fails; the failure collapses to the prefix plus base64 of an empty payload.
- `capability_token()` later fails when the recipient tries to decode the empty token bytes, which moves the error far from the source and makes ticket generation look successful.
- Similar `unwrap_or_default()` serialization fallbacks exist in other ticket and signed-object code, suggesting a cross-cutting pattern instead of a one-off mistake.

## What Changes

- Replace silent `unwrap_or_default()` fallbacks in ticket/identity serialization with explicit `Result`-returning constructors or serializers where feasible.
- For trait-imposed `to_bytes() -> Vec<u8>` paths, centralize the failure handling and log/panic with context instead of minting empty payloads.
- Add regression tests covering oversized capability tokens and postcard serialization failures for ticket wrappers.
- Audit other `postcard::to_stdvec(...).unwrap_or_default()` and `to_allocvec(...).unwrap_or_default()` call sites in externally shared formats.

## Scope

- **In scope**: Ticket, signed identity, and other externally shared serialization paths that currently hide encoding failures.
- **Out of scope**: Internal-only debug formatting or best-effort UI display helpers.

## Evidence

```text
crates/aspen-automerge/src/ticket.rs:35  token: token.encode().unwrap_or_default(),
crates/aspen-automerge/src/ticket.rs:42  let bytes = postcard::to_stdvec(self).unwrap_or_default();

crates/aspen-auth/src/token.rs:82-89
pub fn encode(&self) -> Result<Vec<u8>, AuthError> {
    let bytes = postcard::to_allocvec(self)...?;
    if bytes.len() > MAX_TOKEN_SIZE as usize {
        return Err(AuthError::TokenTooLarge { ... });
    }
}
```
