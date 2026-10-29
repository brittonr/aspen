# Design: Adopt the shared Wasm component runtime

## Context

Molten already owns component profile, materialization, authority, resource, replay, and receipt semantics. The shared runtime extraction owns only product-neutral Wasmtime mechanics. Adoption must remove duplicate mechanism without moving product meaning outward.

## Success Contract

The same admitted component facts produce equivalent normalized local and shared-runtime plans and outcomes. After cutover, one actor and one system extension use the shared runtime through ordinary product composition roots. One declared hostcall crosses the existing Molten effect boundary without ambient WASI.

## Decisions

### Require completed immutable producer boundaries

Adoption starts only after `hardened-wasmtime-runtime` archives its extraction change and publishes one immutable revision. Molten also pins the Mantle consumer verifier and matching materialization schema.

Missing, mutable, stale, or sibling-only prerequisites block cutover. A candidate revision may support parity experiments, but it cannot become the default product path.

The hosted consumer's published Valence `9201074343cd9da0ffaf6ec9db690af5b445d663`
requires Artifact `5f3962d7471d1547e044f57daeda72f56621fd52`. The release validator
correctly rejects mixing that identity with Molten's earlier `c932138` cohort.
Align every active authentication/binding manifest, lock, source pin, plan and
consumer identity to `5f3962d`; retain the duplicate-source denial. The selected
auth-core, auth-ed25519 and binding-core directories have identical existing
files in both source snapshots; the newer snapshot adds package license files.
Its workspace prospectively declares AGPL-3.0-or-later, compatible with Molten's
existing project choice; preserve notices and prior permissive grants. This is
a source-identity prerequisite, not a mechanism or authority-policy change.
Historical migration receipts remain unchanged and identify only their original
cohort. New cohort evidence must separately bind current checks and rollback.
The [current cohort observation](evidence/2026-10-28-hosted-consumers/cohort-alignment.json)
binds the canonical source, selected features, four exercised Nix checks and
coupled rollback; its sibling BLAKE3 sidecar identifies those evidence bytes.

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

The contract-only candidate is `molten.wasm.component.effects.v1`, WIT package `molten:component-effects@1.0.0`, world `actor-effects`. Its only import is `effect-request`, its only export is `invoke`, and its sole operation is `blob.get` / `get` under `hostcall:blob-get`. Both functions use `func(list<u8>) -> result<list<u8>, string>`. The exact Nickel contract requires `execution_admitted=false`; declaring this surface cannot activate an execution profile.

The request carrier binds unsigned generation, incarnation and sequence, exact policy/authority/resource/recorded-effect references, the existing canonical effect request and materialized `<blob-get OBJECT-CONTENT-REF>`. The response binds the complete request-carrier identity separately from the nested effect response's request identity, preserving the existing effect-port DTO contract. A successful response carries the exact materialized output matching its canonical reference; a denial carries neither output nor output reference.

Each complete carrier is bounded to 65,536 bytes, nesting depth 32 with root depth zero, and 4,096 nodes. Preflight traversal bounds recursive decoding and rejects annotations and embedded values; strict canonical intake rejects trailing values and alternate encodings. WIT errors are carrier/host failures, not invented policy denials or effect success.

The future host adapter must independently recheck current policy, Basalt/UCAN, resources, recorded-effect facts, lifecycle fences, monotone sequence and the admitted artifact/handler/capabilities before dispatch. Replay binds complete carrier identities plus run, handler, turn, boundary and sequence without live fallback. Contract codecs and declared-surface checks are not this adapter or production consumer installation. Exact wire shapes, maintenance ownership and regeneration/check commands live in [the governing contract section](../../../docs/wasm-component-runtime.md#contract-only-effect-request-world).

The host-adapter continuation keeps execution activation separate. Its explicit host-owned current inputs supply the installed effect manifest/profile, capability references, generation/incarnation, finite limits, Nickel-exported Basalt policy bytes, trusted issuer/holder identities and caller-owned UCAN verification/revocation/replay services. Guest references must match independently measured current inputs; receipt-shaped assertions and capability-name presence are not authority.

The closed blob resource is `molten://blob/OBJECT-CONTENT-REF`, ability `blob/get`, Basalt contract `component-blob-get`. The holder signs the complete canonical carrier plus run, handler-profile, turn and boundary identities under the effect-world operation profile. The existing Basalt-exported compact Ed25519 UCAN verifier owns token/proof/caveat/revocation/signature and consume-once holder replay checks. Molten owns exact product mapping and canonical policy/resource/recording-plan identities. This is the pinned `onix-compact-ed25519-v1` profile, not final UCAN 1.0.0.

Live dispatch reuses the native callback value port for bounded canonical blob materialization and recording. Its capability-ledger implementation reads through a no-follow bounded namespace and measures stored bytes. Request and intent publication precede the blob read; the bounded response and complete context-bound completion record must publish before success returns. A post-dispatch/recording failure poisons that session rather than becoming a policy denial or retry. Process-visible ledger publication does not claim durable replay or atomic multi-file commit.

Offline replay accepts only exact recorded canonical response bytes and a separately identified completion record. It validates complete scope, lifecycle, sequence, request and response identities and has no live-effect port or authority-grant path. The ordinary deterministic effect-log DTO remains the projection consumed by its existing validator. Production consumers, durable nonce storage selection and effect-world execution activation remain separate adoption gates.

### Activate real product consumers

One ordinary actor path and one system-extension callback path must load Mantle-materialized components through normal manifests and composition roots. Test-only loose bytes and direct unit calls do not satisfy this requirement.

The product fixture must include at least one successful pure invocation, one admitted effect request, one denied effect request, one trap, and one resource denial.

The [2026-10-27 original producer handoff](evidence/2026-10-27-producer-handoff.md) recorded a historical prerequisite: Mantle revision `c99556e5c91dfd13fcc261dc451f471359d47c38` published a framed directory source closure, but its published consumer byte layer accepted only regular files. Producer success and remeasurement therefore did not establish verified consumer bytes. The completed tree-aware publication and Molten private intake below resolve that specific mismatch without changing source identity.

The [published closed-profile selection and immutable supported-world intake](evidence/2026-10-28-supported-world-intake.md) resolve the original import-free supported-world artifact denial. Adoption remains blocked on a distinct reviewed effect-world cohort: the normalized generated effect guest fails the current selected Octet profile, and the exact product effect contract still requires `execution_admitted=false`. Keep the complete local runtime and all current execution-admission restrictions unchanged. No synthetic positive production bundle, archive-as-file substitution, source-tree omission, fabricated Bytes report or weaker artifact profile is an alternative.

The implementation continuation gives private intake a typed reference to the
published `MaterializationBundle` and calls the provider's
`ConsumerRoot::copy_source_closure` through the existing Linux descriptor bridge.
Molten retains its cap-std v3 root capabilities; Mantle owns bounded-tree
traversal/copy and producer v1 framing, including empty directories. Every other
role remains a no-follow regular file, with shared-path conflicts denied and
selected component/WIT bytes captured unchanged. A private source copy alone
does not admit execution: the unchanged DTO must still receive complete
Structural+Bytes report agreement over the snapshot, and cleanup failure denies.
The [original published immutable provider pin and fresh probe intake](evidence/2026-10-27-source-tree-intake.md#immutable-publication-and-executed-pinned-intake)
and [current immutable supported-world continuation](evidence/2026-10-28-supported-world-intake.md)
exercise those integration gates. Neither private intake nor supported-world
materialization establishes runtime invocation or product consumer installation.

### Make cutover and rollback coupled

Cutover removes product-local Wasmtime decision logic or leaves only a thin compatibility adapter. The dependency revision, adapter, configuration, and product fixtures move together.

Rollback restores the prior dependency and local adapter together. It cannot retain shared-runtime receipts while restoring local execution, or restore one consumer class only.

**Executed cutover.** The cutover removed product-local Wasmtime decision logic outright; no compatibility adapter remains. The product API is `wasm_component::Engine`. Parity was established against the archived local oracle at `e5fa4549f` (25 paired tests including the cases added with the cutover, [baseline](evidence/2026-10-29-cutover/pre-cutover-parity-baseline.json)); the surviving corpus asserts those exact stages, classes, outputs and ledgers. The one deliberate difference is the operated epoch deadline the local oracle lacked, recorded as an approved strengthening in the [runtime guide](../../../docs/wasm-component-runtime.md#shared-runtime-adoption-status). Because the dependency pins did not change, rollback is a single revert of the cutover change; the [rollback exercise](evidence/2026-10-29-cutover/rollback-exercise.json) records that revert applying cleanly, restoring the pre-cutover tree exactly, and its paired corpus and consumer suites passing again.

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
