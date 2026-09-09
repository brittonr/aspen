# Design: Adopt the shared Wasm component runtime

## Context

Molten already owns component profile, materialization, authority, resource, replay, and receipt semantics. The shared runtime extraction owns only product-neutral Wasmtime mechanics. Adoption must remove duplicate mechanism without moving product meaning outward.

## Success Contract

The same admitted component facts produce equivalent normalized local and shared-runtime plans and outcomes. After cutover, one actor and one system extension use the shared runtime through ordinary product composition roots. One declared hostcall crosses the existing Molten effect boundary without ambient WASI.

## Decisions

### Require completed immutable producer boundaries

Adoption starts only after `hardened-wasmtime-runtime` archives its extraction change and publishes one immutable revision. Molten also pins the Mantle consumer verifier and matching materialization schema.

Missing, mutable, stale, or sibling-only prerequisites block cutover. A candidate revision may support parity experiments, but it cannot become the default product path.

### Preserve Molten profile and receipt authority

Molten keeps `molten.wasm.component.v1`, WIT package and world identities, canonical Preserves schemas, materialization policy, Basalt/UCAN admission, resource policy, actor and extension lifecycle, replay, and receipts.

A product adapter converts admitted Molten facts into shared-runtime requests and converts normalized observations back into Molten receipt inputs. Shared runtime types cannot enter the pure actor or extension domain.

### Dual-run before cutover

The frozen component corpus runs through the current and shared paths. Parity compares admission class, world and import checks, normalized success or failure, canonical output, fuel and resource classes, and product receipt payload inputs.

Positive fixtures must agree. Negative fixtures must fail at an equivalent pre-effect boundary with the expected stable product class. Any mismatch blocks cutover and retains the local path.

### Add one explicit effect-request hostcall

A new versioned product world may import only a declared Molten effect-request function. Its carrier is bounded canonical Preserves. The guest requests an effect. It does not execute one directly.

The adapter validates operation, capability, policy, authority, resource, replay, and generation facts before it passes a request to an existing Molten effect port. The response is recorded and returned as bounded canonical Preserves.

No WASI interface, ambient process state, or undeclared host function enters the linker.

### Activate real product consumers

One ordinary actor path and one system-extension callback path must load Mantle-materialized components through normal manifests and composition roots. Test-only loose bytes and direct unit calls do not satisfy this requirement.

The product fixture must include at least one successful pure invocation, one admitted effect request, one denied effect request, one trap, and one resource denial.

### Make cutover and rollback coupled

Cutover removes product-local Wasmtime decision logic or leaves only a thin compatibility adapter. The dependency revision, adapter, configuration, and product fixtures move together.

Rollback restores the prior dependency and local adapter together. It cannot retain shared-runtime receipts while restoring local execution, or restore one consumer class only.

## Functional Core and Imperative Shell

- **Molten core**: profile, materialization, authority, effect, replay, consumer, receipt, cutover, and rollback decisions.
- **Shared core**: product-neutral Wasmtime feature, artifact, import, export, resource, and outcome decisions.
- **Shells**: file remeasurement, component compilation and instantiation, hostcall linkage, effect-port calls, cancellation, cleanup, and receipt persistence.

## Risks and Controls

- Adapter drift can change product outcomes. Frozen dual-run parity blocks cutover.
- A generic effect carrier can become ambient authority. Every operation remains closed, declared, and currently admitted.
- Two runtime paths can persist indefinitely. The change records one cutover or one explicit blocked disposition.

## Non-Claims

Passing adoption evidence does not prove Wasmtime correctness, component correctness, effect correctness, host isolation, actor correctness, extension correctness, whole-system safety, or release eligibility.
