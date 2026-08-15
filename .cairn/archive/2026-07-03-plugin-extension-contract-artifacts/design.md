# Design: plugin extension contract artifacts

## Scope

This change introduces the canonical contract unit that plugin manifests and future negotiation/compatibility gates can bind. It does not require a live extension marketplace, dynamic loading, or runtime Nickel evaluation.

## Contract artifact shape

A `plugin-extension-contract-v1` value should be canonical Preserves evidence with fields for:

- contract schema and contract ref;
- extension id;
- extension semantic version or explicit ABI version;
- compatible host ABI id/range;
- lifecycle callbacks provided, required, or extended;
- hostcall descriptors;
- input and output schema refs for each hostcall;
- required authority scopes or authority-context refs;
- required resource dimensions or resource-policy refs;
- effect manifest refs and effect operation ids;
- replay/idempotency class;
- stable error class refs;
- conformance suite refs;
- policy, provenance, and supply-chain evidence refs;
- checks for no ambient authority and fail-closed interpretation.

Hostcall descriptor refs become the unit bound by plugin hostcall receipts. The primitive `<plugin-hostcall OPERATION>` descriptor remains valid for current fixtures, but richer descriptors should include schema, authority, resource, and effect requirements.

## Manifest binding

Plugin manifests should continue to bind plugin id, artifact ref, ABI version, and existing evidence refs. They should additionally bind `extension-contract-refs`. A hostcall or lifecycle callback introduced by an extension is only declared when the active manifest binds the matching contract ref and that contract descriptor admits the operation.

## Contract-aware hostcall gate

Hostcall admission should be a pure function over:

```text
active manifest
extension contracts
hostcall operation
hostcall descriptor ref
input schema ref
output schema ref or expected response shape
authority refs
resource refs
effect receipt refs
executor receipt ref
```

The decision is `pass` only when all requirements for that specific descriptor match. Non-empty evidence is not enough.

## Nickel authoring

Human-authored contracts should live as typed Nickel files. Nickel is used for authoring, validation, defaults, and merge semantics. The runtime consumes checked-in canonical exports and refs only; it must not execute Nickel as an authority-granting step.

Expected authored surfaces:

- `PluginExtensionContract` Nickel contract type;
- positive fixture contract;
- negative fixtures for missing schema, missing authority, invalid version, ambient hostcall, duplicate hostcall descriptor, and unsafe defaults;
- export/check command recorded in docs or build checks.

## Functional core

Parsing, descriptor matching, authority/resource/effect requirement matching, and decision construction belong in pure core functions over already-loaded canonical values. CLI/file/build integration remains the imperative shell.

## Non-goals

- No extension negotiation receipt in this change.
- No upgrade compatibility receipt in this change.
- No dynamic plugin loading or runtime Nickel evaluation.
