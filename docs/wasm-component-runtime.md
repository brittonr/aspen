# Wasm component runtime profile

Aspen's first Wasm component cohort is `molten.wasm.component.v1`. It is an
explicit execution profile; it is not an alias for the legacy
`molten.wasm.abi.v1` core-module ABI.

The normative profile is authored in
[`wasm-component-runtime/profile.ncl`](wasm-component-runtime/profile.ncl),
validated by typed contracts in
[`wasm-component-runtime/contracts.ncl`](wasm-component-runtime/contracts.ncl),
and exported deterministically to
[`wasm-component-runtime/generated/profile.json`](wasm-component-runtime/generated/profile.json).
The profile pins the Wasmtime, Component Model, WIT, `wit-bindgen`, WIT package,
world, and WIT-source cohorts together. Partial and stale cohort combinations
are invalid.

## Execution boundary

`src/wasm/component/mod.rs` exposes the component runtime shell. Its pure admission
core validates:

- exact profile, WIT, feature, and artifact-kind identity;
- fixed memory and table growth plus explicit instance, memory, table, import,
  export, fuel, stack, payload, and result bounds;
- sorted, unique import declarations and one reviewed authority grant per
  admitted import;
- production materialization from a complete Mantle bundle and external
  Valence, Cairn, policy, authority, and resource references.

The imperative shell independently validates the admitted Wasm feature set,
re-inspects nested core memory, table, and instance declarations against the
materialization facts, and then compiles and instantiates the component with
Wasmtime's generated WIT bindings, resource limiter, fuel metering, canonical
NaN lowering, and deterministic feature configuration. Canonical Preserves
bytes are the authoritative host/guest payload carrier. Malformed or
non-canonical outputs deny the execution.

The first cohort admits no imports and no WASI interfaces. Consequently its
linker has no host functions. A future profile must declare each import and
provide a matching policy, authority, resource, and recorded-effect reference;
ambient WASI is never inherited from the process.

## Materialization and evidence

Production execution accepts only portable component and WIT bytes remeasured
from a complete Mantle bundle. Loose bytes are test-only and cannot produce
production evidence. Bundle references are externally linked rather than
embedded recursively, so circular self-attestation is rejected.

Canonical component receipts cover inspection, instantiation, execution,
hostcall, denial, and migration stages. They bind component and WIT identities,
profile and runtime-configuration identities, import/capability grants,
resource bounds, stage parents, canonical input/output identities, fuel, and
role-separated Mantle, Valence, Cairn, policy, authority, resource, and
recorded-effect references. Denial receipts bind a closed, typed denial/trap
class; raw engine error text remains diagnostic-only. Structural validation
checks canonical shape and identity, contextual validation compares every field
to the independently derived inspection or execution plan, and chain validation
checks each stage parent before replay comparison. Readback labels component
receipts explicitly, and replay compares canonical receipt identity rather than
host diagnostics.

Receipts establish bounded facts for the declared bytes, cohort, inputs,
resources, and cited evidence. They do **not** establish behavioral correctness,
whole-system correctness, hostcall purity, entitlement beyond cited authority,
provenance beyond cited external evidence, source-language equivalence,
replay equivalence outside recorded effects, or release eligibility by
themselves.

## Migration and consumers

Artifact headers are classified before execution:

- `molten.wasm.abi.v1` requires a core module;
- `molten.wasm.component.v1` requires a component;
- mismatches deny without fallback or silent reinterpretation.

Both actor and system-extension consumer classes pass through the same
materialization, admission, runtime, and receipt rails. This establishes
profile-level readiness only; it does not claim that every existing actor or
system extension has migrated.

Focused positive and negative tests live under `src/wasm/component/tests/`.
Nickel rejection fixtures cover stale or incomplete cohorts, ambient WASI, and
nondeterministic growth under
`docs/wasm-component-runtime/fixtures/negative/`.
