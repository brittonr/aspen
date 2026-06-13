## Why

Molten has selected crates for a programmable distributed runtime, but the repo does not yet define how those crates compose. Without an explicit lifecycle change, implementation can drift into a tightly coupled runtime where networking, scripting, sandboxing, policy, and evidence surfaces depend on each other directly.

## What Changes

- Define a pure envelope spine with Serde DTOs as the shared boundary between all runtime adapters.
- Use Blake3 for canonical boundary hashing, Snafu for structured error boundaries, and Tracing for adapter/runtime observability.
- Keep Preserves canonical values and hashes at every communication, wire, storage, policy, and evidence boundary, using content references for large payload bytes.
- Route local runtime behavior through a Syndicate-backed actor/dataspace adapter.
- Use BEAM/OTP and Lunatic as non-normative references for actor lifecycle, supervision, mailboxes, links/monitors, scheduling, and Wasm hostcall ergonomics.
- Bridge peers with Iroh gossip for small envelopes, Iroh blobs for large content, and Iroh docs for replicated mutable document/state surfaces.
- Add execution adapters for sandboxed Wasmtime actors, deny-by-default WASI capabilities, WIT/component bindings, wasmparser inspection, and trusted Steel orchestration.
- Evaluate Nickel configuration into typed startup configuration instead of using it as a hot-path dependency.
- Use Nickel contracts for static declarative policy/config/schema gates and Steel contracts for explicitly reviewed dynamic predicates or trusted callables, with both enforced through Basalt before side effects occur.
- Add a Clap CLI surface and Hegel property-based test rail for broad runtime invariants.
- Add policy and evidence gates that use Trellis predicates plus Cairn/Octet receipts.
- Stage implementation so core types land before networking, execution, and policy adapters.

## Impact

This creates the architectural contract for Molten's first real runtime milestone. It does not require all adapters to be production-ready at once; it requires each adapter to attach to the same typed envelope and to preserve the pure-core/imperative-shell boundary.
