## Context

Aspen has several shareable ticket and signed-identity formats that are serialized into compact strings or byte payloads for transport. A recent audit found a bad failure pattern in a few of those encoders: if serialization fails, the code substitutes `Vec::new()` via `unwrap_or_default()` and continues.

That is dangerous because these payloads are not best-effort logs. They are authority-bearing wire formats. Silent fallback moves the failure away from the source and can produce strings that look structurally valid but decode to garbage or to an empty payload.

`AutomergeSyncTicket` is the clearest case because its constructor wraps `CapabilityToken::encode()`, which is explicitly fallible and enforces `MAX_TOKEN_SIZE`.

## Goals / Non-Goals

**Goals:**

- Make ticket and signed-identity encoding failures visible at the point where the ticket is created or serialized.
- Prevent any shareable ticket string from being minted from empty fallback bytes.
- Preserve ergonomic APIs where encoding is genuinely infallible, while avoiding silent data loss in trait-constrained code.
- Add regression tests for oversized or otherwise invalid inputs.

**Non-Goals:**

- Redesigning every ticket format in the repo.
- Changing the on-wire encoding itself.
- Adding backward-compatibility shims for already-broken empty-payload strings.

## Decisions

### D1: Constructors and serializers that can fail should return `Result`

When a wrapper type depends on a fallible inner encoder, the outer API should not hide that fact.

**Decision:** convert helper APIs such as `AutomergeSyncTicket::new` and any similar wrapper serializer into fallible APIs that return `Result` with the underlying encoding error.

This keeps the failure close to the call site and avoids shipping a ticket object that is already invalid.

### D2: Trait-imposed `to_bytes() -> Vec<u8>` paths must fail loudly, not silently

Some types implement third-party traits such as `iroh_tickets::Ticket`, where `to_bytes()` cannot return a `Result`.

**Decision:** for those trait-imposed encoders, replace `unwrap_or_default()` with a fail-fast strategy that preserves context, typically an `expect(...)` documenting why serialization is intended to be infallible.

An explicit crash is safer than emitting an authority-bearing empty payload that may later be misdiagnosed as a decode error or bad user input.

### D3: Keep impossible-path claims narrow and tested

Several current comments assert that postcard serialization is infallible for certain structs. Those claims should stay only where the type really is bounded and directly serializable.

**Decision:** pair each fail-fast `expect(...)` path with tests that exercise normal round trips and, where possible, move genuinely fallible work into earlier `Result`-returning constructors.

### D4: Audit externally shared formats before internal helpers

The highest-risk cases are values that are copied into URLs, tickets, handshake payloads, or persisted identities.

**Decision:** prioritize `AutomergeSyncTicket`, client tickets, signed cluster tickets, and federation identity documents before cleaning up internal convenience serializers that do not escape the process.

## Risks / Trade-offs

- **[Risk]** Making constructors or serializers fallible will touch call sites and tests. -> Mitigation: keep the error types narrow and start with the externally shared formats that already have explicit decode APIs.
- **[Risk]** Replacing silent fallback with `expect(...)` can crash in edge cases. -> Mitigation: reserve it for trait-imposed encoders where empty fallback is worse than aborting and document the invariants clearly.
- **[Risk]** Some callers may rely on the old `String`-returning API shape. -> Mitigation: use staged refactors where constructors become fallible first, then the remaining infallible helpers can wrap the validated value.
