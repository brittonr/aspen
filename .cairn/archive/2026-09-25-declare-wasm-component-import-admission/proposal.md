# Proposal: Wasm component import-table admission

## Why

The shared Molten runtime admits Wasm components but does not yet enforce a complete, declared import table as a hard admission fact. MoonBit's component-model work shows that explicit WIT import and export surfaces, with no hidden ambient imports, are the practical route to minimal verifiable components. An admitted component with an undeclared ambient import breaks the zero-import discipline and invalidates the runtime's bounded evidence.

## What Changes

- Require every admitted component to declare its complete import table. r[aspen.wasm_import_admission.manifest]
- Reject, before instantiation, any component whose imports drift from the declared WIT world or reach undeclared ambient capabilities. r[aspen.wasm_import_admission.surface]
- Make admission a pure core decision over typed artifact facts. r[aspen.wasm_import_admission.admission]
- Bind tool, verifier, and manifest identities in bounded receipts. r[aspen.wasm_import_admission.evidence]
- Add positive and negative fixtures for exact, drifting, importing, and non-claim cases. r[aspen.wasm_import_admission.fixtures]

## Impact

- **Core**: a focused family owns manifest-shape validation, world matching, import-set comparison, and receipt payload construction without host effects.
- **Shell**: the host owns artifact reads, tool invocation, instantiation, and bounded runtime observations.
- **Existing owners**: the shared component runtime, Mantle materialization, and Lattice pack admission keep their claims.

## Lifecycle Prerequisites

None. Admission consumes values the shared runtime already observes. It adds no new component tooling.

## Out of Scope

- New component tooling, WIT authoring, guest semantics, WASI host harvesting, or runtime redesign.
- Claims of sandbox completeness, component correctness, or married-safety beyond the declared import set.

## Affected Specs

- `wasm-import-admission`: manifest, surface, admission, evidence, fixtures, and non-claims.
