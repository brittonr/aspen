# Tasks

## Phase 1: Prerequisites and baseline

- [ ] [serial] Record current component runtime, actor, system-extension, effect-hostcall, receipt, and negative-fixture baseline results before core changes. r[molten.wasm_component_adoption.parity] r[molten.wasm_component_adoption.validation]
- [ ] [serial] Pin completed immutable `hardened-wasmtime-runtime` and Mantle consumer-verifier revisions with matching contracts and source evidence. r[molten.wasm_component_adoption.prerequisite]
- [ ] [serial] Freeze the parity corpus and define exact local-to-shared request, blocker, outcome, resource, and receipt-payload mappings. r[molten.wasm_component_adoption.parity] r[molten.wasm_component_adoption.cutover]

## Phase 2: Adapter and parity

- [ ] [serial] Implement the thin Molten adapter while keeping product profile, materialization, actor, extension, effect, replay, receipt, and policy types outside the shared runtime. r[molten.wasm_component_adoption.cutover] r[molten.wasm_component_adoption.boundary]
- [ ] [parallel] Dual-run positive pure execution, canonical output, trap, fuel, deadline, resource, and cleanup fixtures. r[molten.wasm_component_adoption.parity] r[molten.wasm_component_adoption.validation]
- [ ] [parallel] Dual-run negative loose bundle, wrong world, undeclared import, stale evidence, malformed output, resource, fallback, and receipt-overclaim fixtures. r[molten.wasm_component_adoption.parity] r[molten.wasm_component_adoption.validation] r[molten.wasm_component_adoption.nonclaims]

## Phase 3: Product hostcall and consumers

- [ ] [serial] Define the versioned Molten effect-request import, canonical Preserves carrier, closed operations, authority and resource inputs, recorded-effect binding, and no-WASI linker profile. r[molten.wasm_component_adoption.hostcalls]
- [ ] [serial] Implement the product host adapter through existing effect ports with current policy, Basalt/UCAN, generation, resource, and replay admission before dispatch. r[molten.wasm_component_adoption.hostcalls] r[molten.wasm_component_adoption.boundary]
- [ ] [parallel] Add admitted and denied effect-request fixtures with exact request, response, replay, and receipt bindings. r[molten.wasm_component_adoption.hostcalls] r[molten.wasm_component_adoption.validation]
- [ ] [serial] Wire one ordinary actor and one system extension through non-test product composition roots and matching Mantle bundles. r[molten.wasm_component_adoption.consumers]

## Phase 4: Cutover and evidence

- [ ] [serial] Define coupled cutover and rollback for dependency, adapter, profile, actor, extension, and receipt cohorts. r[molten.wasm_component_adoption.rollback]
- [ ] [serial] Remove duplicate product-neutral mechanism or reduce it to a compatibility shim only after complete parity and consumer evidence passes. r[molten.wasm_component_adoption.cutover] r[molten.wasm_component_adoption.rollback]
- [ ] [serial] Update runtime and consumer documentation with ownership, dependency, operation, rollback, and non-claim boundaries. r[molten.wasm_component_adoption.boundary] r[molten.wasm_component_adoption.nonclaims]
- [ ] [serial] Run focused runtime, actor, extension, effect, replay, receipt, Octet, Cairn, workspace, and relevant Nix checks, then record exact dependency and bundle identities. r[molten.wasm_component_adoption.validation]
