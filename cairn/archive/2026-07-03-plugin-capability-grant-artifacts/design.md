# Design: plugin capability grant artifacts

## Scope

This change makes plugin host authority explicit and typed. It does not add ambient host access, dynamic loading, or runtime Nickel evaluation. Steel and Wasm plugins continue to reach the host only through declared hostcalls, and the host admits those calls only after canonical grant, policy, resource, effect, and executor evidence passes.

## Typed reference vocabulary

Plugin host data should distinguish ref roles before admission logic runs:

```text
ArtifactRef
ManifestRef
ExtensionContractRef
HostcallDescriptorRef
SchemaRef
PolicyRef
ResourceRef
EffectManifestRef
EffectReceiptRef
ExecutorReceiptRef
CapabilityGrantRef
RevocationReceiptRef
ProofRef
IssuerRef
```

All of these may be represented as BLAKE3-addressed canonical evidence, but they are not interchangeable. A `CapabilityGrantRef` is a ref to a parsed `plugin-capability-grant-v1` value; a raw artifact, schema, or policy ref MUST NOT satisfy authority merely because it has a BLAKE3 prefix.

## Grant artifact shape

A canonical `plugin-capability-grant-v1` value should bind:

- grant schema id and grant ref;
- subject plugin ref and plugin id;
- active manifest ref;
- extension contract ref when the operation comes from an extension;
- hostcall descriptor ref and operation;
- input/output schema refs expected by the descriptor;
- resource refs and optional resource sub-scope;
- effect manifest refs and required effect receipt classes;
- policy refs and policy profile;
- issuer ref, proof refs, and optional Basalt/UCAN evidence refs;
- attenuation metadata such as delegated scope, maximum delegation depth, budget refs, turn/tick validity, and replay/idempotency class;
- revocation receipt refs or revocation-list refs;
- checks for no ambient authority and deterministic replay.

The grant ref is the BLAKE3 hash of the canonical grant value. The hash provides identity and integrity; it is not authority unless the host has parsed the grant and all semantic checks pass.

## Hostcall admission

Hostcall admission remains a pure function over already-loaded canonical values:

```text
active manifest
bound extension contracts
hostcall descriptor
request operation
request schema refs
request resource refs
request effect/executor receipt refs
capability grant artifacts
revocation evidence
policy/effect context
```

The decision is `pass` only when at least one supplied grant matches the selected descriptor and the whole request context:

- same plugin id/ref and active manifest ref;
- same operation and hostcall descriptor ref;
- same extension contract ref when applicable;
- matching input/output schema refs;
- requested resource is within the grant's resource scope;
- required effect manifest and effect receipt refs are present;
- required policy and proof refs are present;
- attenuation limits are not exceeded;
- no bound revocation evidence invalidates the grant.

Generic `authority_refs` may remain in receipts as compatibility/proof evidence, but passing hostcall admission should require `capability_grant_refs` whose bodies validate. Compatibility code should fail closed when a hostcall descriptor requires typed grants and only generic authority refs are supplied.

## Revocation and attenuation

Revocation and expiry must be deterministic. The pure core should not read clocks or external revocation lists directly. The shell supplies canonical revocation receipts, validity-window evidence, or policy snapshots; the core verifies their refs and statuses.

Attenuation should support scope narrowing without ambient inference:

- operation narrowing;
- resource prefix/subtree narrowing;
- schema/profile narrowing;
- bounded delegation depth;
- budget or byte-count refs;
- turn/tick validity evidence;
- replay/idempotency class restrictions.

## Steel and Wasm boundary

Steel and Wasm executors do not receive raw filesystem, process, network, or WASI access through this change. They can only call declared hostcall imports/functions. The host returns pass/deny hostcall responses after the capability grant, descriptor, policy, resource, effect, and executor checks have produced receipt evidence.

## Nickel authoring

Human-maintained capability grant fixtures should use typed Nickel contracts. Runtime admission consumes checked-in canonical Preserves exports and refs only. Nickel source presence is documentation/authoring evidence, not runtime authority.

Expected authored surfaces:

- `PluginCapabilityGrant` Nickel contract type;
- positive storage-read grant fixture;
- negative fixtures for raw artifact ref as grant, wrong operation, wrong manifest, wrong resource, missing proof, expired grant, revoked grant, and over-delegation;
- export/check documentation for regenerating canonical evidence.

## Functional core

Parsing, typed-ref classification, grant matching, attenuation checks, revocation checks, and receipt construction belong in pure functions over in-memory values. CLI/file/build code loads artifacts, resolves refs, invokes Nickel export checks, and writes receipts.

## Non-goals

- No ambient host access for Steel, Wasm, native, adapter, or remote plugins.
- No wall-clock reads in core grant validation.
- No runtime Nickel execution as authority.
- No requirement to replace Basalt/UCAN evidence; those refs can be proof inputs bound by the plugin capability grant.
