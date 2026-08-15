## Why

Plugin manifests currently list callbacks, hostcall refs, schemas, policy refs, resource refs, effect refs, and supply-chain refs directly. That works for the initial host ABI, but it does not provide a reusable contract unit for extension authors, host operators, conformance tests, compatibility checks, or generated documentation.

Molten should make plugin and extension contracts first-class canonical artifacts. A plugin manifest can then bind reviewed extension contracts instead of relying on ad hoc lists whose meaning is spread across code and tests.

## What Changes

- Define a canonical `plugin-extension-contract-v1` artifact for extension id/version, hostcall descriptors, lifecycle additions, schema refs, authority/resource/effect requirements, replay/idempotency class, error class refs, conformance refs, and policy/supply-chain evidence refs.
- Require plugin manifests to bind extension contract refs for extension-provided lifecycle callbacks and hostcalls.
- Require hostcall admission to evaluate the specific contract descriptor for a hostcall rather than accepting any non-empty authority/resource/effect evidence.
- Add typed Nickel authoring contracts for human-maintained plugin extension definitions, with checked-in canonical exports consumed by Rust validation.

## Impact

- **Specs**: `plugin-host` gains a stable extension contract artifact model.
- **Docs/config**: extension authors get typed Nickel contracts instead of untyped fixture conventions.
- **Core**: plugin manifest parsing and hostcall admission gain contract-aware validation once implemented.
- **Future work**: negotiation, compatibility receipts, and conformance suites can target extension contract refs.
