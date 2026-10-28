# Tasks

## Phase 1: Prerequisites and baseline

- [x] [serial] Record current component runtime, actor, system-extension, effect-hostcall, receipt, and negative-fixture baseline results before core changes. r[molten.wasm_component_adoption.parity] r[molten.wasm_component_adoption.validation]
  - Evidence: [2026-10-26 compatibility preflight](evidence/2026-10-26-compatibility-preflight.md#executed-local-product-baseline): 85 selected runtime/extension/hostcall cases and 11 selected actor-ABI cases passed in two overlapping selections; no product source changes.
- [x] [serial] Pin completed immutable `hardened-wasmtime-runtime` and Mantle consumer-verifier revisions with matching contracts and source evidence. r[molten.wasm_component_adoption.prerequisite]
- [x] [serial] Freeze the parity corpus and define exact local-to-shared request, blocker, outcome, resource, and receipt-payload mappings. r[molten.wasm_component_adoption.parity] r[molten.wasm_component_adoption.cutover]
  - Evidence: [immutable inputs, frozen mapping and original counterexamples](evidence/2026-10-26-runtime-adoption-parity.md), followed by the [published v4 correction and unchanged-cohort parity](evidence/2026-10-27-boundary-parity.md). The candidate remains test-only and the complete local cohort remains the production default. No synthetic positive production Mantle bundle or default backend change is admitted.

## Phase 2: Adapter and parity

- [ ] [serial] Implement the thin Molten adapter while keeping product profile, materialization, actor, extension, effect, replay, receipt, and policy types outside the shared runtime. r[molten.wasm_component_adoption.cutover] r[molten.wasm_component_adoption.boundary]
- [ ] [parallel] Dual-run positive pure execution, canonical output, trap, fuel, deadline, resource, and cleanup fixtures. r[molten.wasm_component_adoption.parity] r[molten.wasm_component_adoption.validation]
- [ ] [parallel] Dual-run negative loose bundle, wrong world, undeclared import, stale evidence, malformed output, resource, fallback, and receipt-overclaim fixtures. r[molten.wasm_component_adoption.parity] r[molten.wasm_component_adoption.validation] r[molten.wasm_component_adoption.nonclaims]
  - The original scheduling/memory counterexamples are corrected by the immutable v4 producer and reviewed mapping. Operated product deadlines and complete production consumer/materialization/effect evidence remain unproved; the exercised import-free corpus does not close these end-to-end tasks.

## Phase 3: Product hostcall and consumers

- [x] [serial] Define the versioned Molten effect-request import, canonical Preserves carrier, closed operations, authority and resource inputs, recorded-effect binding, and no-WASI linker profile. r[molten.wasm_component_adoption.hostcalls]
  - Evidence: [contract-only effect-world continuation](evidence/2026-10-27-boundary-parity.md#contract-only-effect-world-continuation), exact candidate Nickel/WIT, bounded canonical codecs and a real generated-WIT carrier invocation. `execution_admitted=false`; definition does not close the following adapter, effect-dispatch, consumer or cutover tasks.
- [x] [serial] Implement the product host adapter through existing effect ports with current policy, Basalt/UCAN, generation, resource, and replay admission before dispatch. r[molten.wasm_component_adoption.hostcalls] r[molten.wasm_component_adoption.boundary]
- [x] [parallel] Add admitted and denied effect-request fixtures with exact request, response, replay, and receipt bindings. r[molten.wasm_component_adoption.hostcalls] r[molten.wasm_component_adoption.validation]
  - Evidence: [inactive host-adapter continuation](evidence/2026-10-27-host-adapter.md): genuine signed UCAN/holder admission, actual capability-backed canonical blob and recording bytes, current-policy/lifecycle/resource denials before protected reads, complete offline replay without live fallback, and poisoned-session handling of uncertain publication. The public-API smoke admitted and denied retrieval with exactly one protected read. No effect-world linker, actor/extension installation, durable nonce-store selection or cutover is claimed.
- [ ] [serial] Wire one ordinary actor and one system extension through non-test product composition roots and matching Mantle bundles. r[molten.wasm_component_adoption.consumers]
  - Historical counterexample: the [original producer/consumer handoff](evidence/2026-10-27-producer-handoff.md) remeasured its directory source successfully while the old pinned consumer reported that source missing and matched all 36 file members. Preserve that observation; it is not a current tree-verifier failure.
  - Resolved tree-intake prerequisite: [immutable Mantle publication and executed pinned intake](evidence/2026-10-27-source-tree-intake.md#immutable-publication-and-executed-pinned-intake) use `cf7623b81a63e3df9b5d69ad812c4a63ab78c1e0`. Private snapshots delegate to `ConsumerRoot::copy_source_closure` over opened capability bridges, retain unchanged tree identity and require complete Structural+Bytes agreement across all 37 members. The actual fresh-pipeline probe retains exact component/WIT bytes and denies empty-directory source drift with cleanup. Its profile remains explicitly unadmitted; no production bundle rewrite, independent tree walker, directory permission for other roles or actor/extension installation is claimed.
  - Separate supported-world prerequisite: [the actual supported-world pipeline](evidence/2026-10-27-source-tree-intake.md#distinct-supported-world-prerequisite) reached normalization but the unchanged pinned Octet artifact policy denied two nested component type imports and bulk memory. Successful intake with the explicitly unadmitted probe profile does not close this blocker. Matching production consumer installation requires the owner correction and immutable publication; no profile weakening, bundle rewrite or runtime activation is authorized by the tree-intake repair.

## Phase 4: Cutover and evidence

- [ ] [serial] Define coupled cutover and rollback for dependency, adapter, profile, actor, extension, and receipt cohorts. r[molten.wasm_component_adoption.rollback]
- [ ] [serial] Remove duplicate product-neutral mechanism or reduce it to a compatibility shim only after complete parity and consumer evidence passes. r[molten.wasm_component_adoption.cutover] r[molten.wasm_component_adoption.rollback]
- [ ] [serial] Update runtime and consumer documentation with ownership, dependency, operation, rollback, and non-claim boundaries. r[molten.wasm_component_adoption.boundary] r[molten.wasm_component_adoption.nonclaims]
- [ ] [serial] Run focused runtime, actor, extension, effect, replay, receipt, Octet, Cairn, workspace, and relevant Nix checks, then record exact dependency and bundle identities. r[molten.wasm_component_adoption.validation]
