## Why

`crates/aspen-hooks-ticket` still exposes concrete `iroh::EndpointAddr` values, ambient wall-clock helpers, and `std`-bound error/serialization choices in a crate that should be a lightweight shared ticket surface. That keeps a small ticket crate on the runtime side of Aspen's no-std/std boundary and forces consumers to pull transport/runtime baggage even when they only need to parse or hold hook tickets.

## What Changes

- Make `crates/aspen-hooks-ticket` alloc-safe by default with explicit `std` convenience helpers instead of unconditional runtime dependencies.
- **BREAKING** Replace the public `AspenHookTicket.bootstrap_peers` payload from concrete iroh endpoint addresses to transport-neutral `NodeAddress` values; previously serialized hook tickets will need regeneration after the schema change.
- Move expiry math into pure helper APIs that accept explicit `now_secs`, while keeping optional wall-clock convenience wrappers only for runtime builds.
- Replace `anyhow`-style ticket errors with a crate-local error type so shared ticket parsing/validation remains alloc-safe.
- Remove the silent empty-payload serializer fallback from `AspenHookTicket` so impossible encoding failures fail loudly instead of minting misleading tickets.
- Update runtime consumers (`crates/aspen-hooks`, `crates/aspen-cli`) to opt into conversion helpers explicitly and surface invalid bootstrap-peer conversion errors instead of assuming iroh-native ticket payloads.

## Non-Goals

- Refactoring `crates/aspen-ticket` or signed cluster tickets in the same change.
- Reworking hook-trigger RPC semantics or authentication rules beyond the intentional ticket-schema change above.
- Removing iroh from runtime consumers that already own connection setup.
- Adding separate release-note or CLI migration UX beyond the shared ticket crate's explicit decode failures when legacy hook tickets are used.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `architecture-modularity`: extend the alloc-safe leaf-crate boundary so `aspen-hooks-ticket` defaults to transport-neutral, alloc-safe ticket data and only runtime consumers opt into wall-clock or iroh conversion helpers.
- `ticket-encoding`: require hook ticket encoding to fail loudly instead of substituting an empty payload on impossible serializer errors.

## Impact

- **Files**: `crates/aspen-hooks-ticket/`, `crates/aspen-hooks/`, `crates/aspen-cli/`, and OpenSpec artifacts under `openspec/changes/alloc-safe-hooks-ticket/`.
- **APIs**: `AspenHookTicket` stores alloc-safe bootstrap addresses and exposes pure expiry helpers; runtime wall-clock and iroh conversion paths become explicit opt-ins.
- **Dependencies**: default `aspen-hooks-ticket` resolution stops pulling `iroh`; runtime crates add explicit direct opt-ins for any needed address conversion.
- **Testing**: targeted `cargo test`/`cargo check`/`cargo tree` rails plus explicit positive and negative regression coverage for expiry and bootstrap-peer conversion.
