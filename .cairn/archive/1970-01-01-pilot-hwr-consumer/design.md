# Design: Molten-owned standalone Component Model boundary

## Context

The root Molten runtime still owns its existing component shell and production admission/receipt rails. Historical extracted-runtime evidence did not prove the corrected public component API. The separate `adopt-shared-wasm-component-runtime` change covers eventual parity and production cutover, not this pilot.

## Decisions

### Decision: Isolate the consumer in an existing pilot convention

**Choice:** Use `pilots/hwr-consumer/Cargo.toml` with its own empty `[workspace]`, like the existing schema-migration pilot. Depend only on BLAKE3, WAT encoding, and the external runtime core/shell. Do not import the root Molten crate or touch root dependency/lock/Nix plans.

**Rationale:** A parent can run a real consumer command without rebuilding or migrating the product host. No engine handles or runtime dependencies enter pure Molten cores.

### Decision: Keep actor semantics and fixture trust in Molten

**Choice:** Define package `molten:component-runtime@1.0.0`, world `actor`, root export `actor-turn`, and an explicit synthetic eight-byte turn carrier. Input is little-endian `(actor-id:u32, counter:u32)`; the guest preserves the actor and increments the counter. WIT and guest sources are owned here. The exact demonstration maps `(7,41)` to `(7,42)`, bytes `070000002a000000`.

**Rationale:** The producer sees only a generic bounded byte carrier. Product semantics, WIT/schema identity, output validation, and trust are consumer-owned. This synthetic contract is not canonical Preserves or the production actor ABI. Fixture `MaterializationFacts` are local declarations, not externally trusted Mantle materialization. The shell checks component class, byte identity, and exact names; the caller still owns WIT/world trust.

### Decision: Exercise the corrected component API with finite resources

**Choice:** Admit a pilot-owned Nickel policy wire document through `profile::document::admit`, then call `BoundedRuntime::build(&admitted.policy, admitted.limits, Strategy::Cranelift)`. Encode actual Component Model binary bytes with pinned `wat`, measure them using BLAKE3, and pass `Artifact { bytes, facts }`, empty descriptors, positive `Budget`, and `Some(&service)` to `ComponentInvocation::prepare`. Invoke `actor-turn` through `run(export, input, host, binder)`; the binder denies all imports.

**Rationale:** No defaults, optional-deadline bypass, core-module fallback, shim API, or ambient WASI are involved. Policy admits 10,000 fuel, eight input/output bytes, one fixed memory page, finite stack, instance/import/export/concurrency limits. A consumer-owned driver advances the exact engine every 10 ms under a 1,000-tick budget, then is stopped and joined. The binary checks exact output, positive consumed fuel, malformed-turn `GuestDenied`, oversized-input `InputLimitExceeded`, and absent-service `DeadlineDenied`.

The frozen extracted document parser requires the Molten schema, package, actor world, production scope marker, and Nickel marker. `profile.ncl` is a pilot-specific source, not a production policy migration. These parser markers do not elevate loose fixture bytes into production evidence. The WIT identity is remeasured and checked independently in this consumer.

### Decision: Keep publication and evidence gates honest

**Choice:** Retain the historical immutable published revision as temporary working input in both producer manifest entries and `source.ncl`. Replacement must be a corrected, reviewed immutable external revision. No path dependency or placeholder revision is delivered.

**Rationale:** The current published pin lacks the corrected API; this source is not runnable against it. The producer must first resolve bounded component-result lifting, including post-return cleanup behavior, and provide its final candidate. Execution and lifecycle tasks remain unchecked until observed under that candidate. Development-only local Cargo patches are not publication evidence or durable dependencies.

## Risks / Trade-offs

- Producer API availability, bounded lifting correctness, remote Git access, and final immutable publication are prerequisites to smoke evidence, not conclusions of source inspection.
- Epoch timing is finite but is not a latency or deterministic scheduling claim; fuel observation is reported, not hardcoded to an incidental compiler instruction count.
- Handwritten canonical ABI guest glue is intentionally restricted to the one admitted input allocation and fixed output layout. It is a pilot fixture, not a general allocator or production guest SDK.
- The local WIT/schema declaration cannot independently establish artifact provenance, release authority, actor persistence, replay, or policy correctness.

## Operation and evidence state

Run from the repository root after coupled pin replacement:

```sh
cargo run --manifest-path pilots/hwr-consumer/Cargo.toml
```

The exact payload, normalized denials, fuel observation, and source revision must be recorded after the command actually runs. At source handoff no builds, tests, formatters, linters, production execution, or publication are claimed. The governing runtime document owns the repeatable command, maintenance owner, policy projection maintenance, and non-claims.
