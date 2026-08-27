## Context

Molten already identifies code artifacts and runtime profiles. Mantle owns build outputs and build evidence. Neither layer currently owns a product-neutral immutable executable-extent mapping mechanism.

The world protocol needs only an optional typed reference and adapter. It must not absorb build, storage, mapping, or runtime authority.

## Decisions

### Decision: Keep four identity domains separate

**Choice:** Model semantic code identity, built artifact identity, executable-extent manifest identity, and live mapping identity as distinct nominal types.

A world commit binds the artifact and extent manifest. Live mapping observations stay detached.

**Rationale:** Equal source meaning, equal build bytes, equal layout, and equal host mappings are different claims.

### Decision: Consume a product-neutral extent mechanism

**Choice:** Pin the reviewed `executable-extent` project by immutable source revision. Use its pure layout and W^X admission core and its positive and negative conformance vectors.

Molten owns runtime-profile compatibility, current execution authority, and adapter orchestration.

**Rationale:** Extent layout and mapping safety are reusable mechanisms. Runtime admission remains product-specific.

### Decision: Accept only exact producer bundles

**Choice:** The extent world-root profile binds one Mantle bundle identity, source artifact identity, target triple, executable format, ABI cohort, page-size profile, ordered extent descriptors, closure, and producer receipt references.

Molten remeasures all bytes and validates exact manifest parity. Producer success alone does not grant mapping.

**Rationale:** A path or producer callback cannot establish content, layout, or current authority.

### Decision: Enforce W^X through explicit states

**Choice:** Allowed states are absent, materialized writable-nonexecutable staging, sealed read-only, mapped read-only, mapped executable-read-only, and unmapped.

No transition permits writable and executable access at the same time. Executable activation requires sealed bytes and a fresh read-back observation.

**Rationale:** W^X must be a checked transition contract, not an adapter convention.

### Decision: Use capability-relative handles and stable objects

**Choice:** The shell opens admitted roots through capability handles, materializes through Durable File Publication or an equivalent reviewed mechanism, verifies by handle, and maps the verified object without reopening an ambient path.

**Rationale:** Path re-resolution can substitute bytes between verification and mapping.

### Decision: Keep fallback explicit

**Choice:** A runtime may support an ordinary-artifact profile that interprets or compiles through its existing path. It cannot satisfy policy that requires executable extents.

Unsupported target, format, ABI, page size, relocation model, or runtime cohort fails before mapping.

**Rationale:** Silent fallback would erase the stronger profile's meaning.

### Decision: Keep execution and retention authority outside extent validity

**Choice:** Valid extents remain inert until current artifact, runtime, resource, policy, and execution admission passes. Reachability may retain extent objects, but deletion remains under the world retention change.

**Rationale:** Correct bytes and layout do not grant permission to execute or retain them.

## Rollout

1. Pin private Radicle source `rad://z37R1bP1kHcELs89RNbQRaqbCVKxB` at reviewed revision `65f00649eebd5b42426f76f77ffa1f91e26d17eb`, then wait for the Mantle producer bundle. Private visibility does not replace immutable source identity.
2. Pin exact source and add compatibility vectors.
3. Add pure profile and manifest admission without mapping.
4. Add capability-relative materialization, sealing, and read-back.
5. Add one native or AOT executable mapping fixture.
6. Add optional hardened Wasmtime adoption only under an exact compiled-artifact cohort.

## Risks / Trade-offs

- Page-size and loader behavior vary by host. Profiles must remain exact and fail closed.
- AOT artifacts can embed relocation assumptions. Those assumptions belong in the compatibility cohort.
- Direct mapping can improve sharing but increases unsafe-adapter review needs. Keep unsafe code narrow and justified.
- Ordinary artifact support can hide extent gaps. Operator output must name the selected profile.
