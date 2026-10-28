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

## Shared-runtime adoption status

The root dependency cohort pins `hwr.shell.v4` at
`cf4a1913f7f2ca7eb7f655663e87b8b72d0c506c` and the published Mantle consumer
verifier at `e1827a8cc1cacf2810aa2314dc8971177832a8f0`. The source gate binds
their completed native archives and exact evidence digests, including Mantle's
closed Octet profile selection. These pins are prerequisites, not authorization
to change the execution backend.

The adapter under `runtime/shared/` remains test-only. Its bounded initialization
driver consumes typed Instantiation yields through the successful checkpoint
before exposing callback polls, preserving the existing callback ledger without
resetting fuel or restarting the guest. The checked memory translation retains
the product's per-memory ceiling and derives a redundant aggregate bound from
that ceiling times the already admitted memory count; no product limit, profile
identity or WIT bytes are widened or changed.

The original start-work and two-129-page-memory cases now agree on complete
canonical receipts and callback ledgers, with paired actual memory-size/count
denials. See the [immutable v4 continuation evidence](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-27-boundary-parity.md).
Public component execution still uses the complete local runtime. The existing
actor harness and sandboxed extension fixture execute legacy core modules; they
are not shared Component Model consumer installations. The [earlier frozen mapping and counterexamples](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-26-runtime-adoption-parity.md)
remain historical evidence; they are not current v4 failures. Operated product
deadlines, effect-world linker activation, matching production materializations,
real shared actor/extension installation and coupled cutover/rollback remain
acceptance gates.

The [original directory-missing handoff](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-27-producer-handoff.md)
and [original supported-world artifact denial](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-27-source-tree-intake.md#distinct-supported-world-prerequisite)
remain historical counterexamples. The current published Mantle pin retains
tree-aware verification and adds explicit closed Octet profile selection.
The [immutable supported-world continuation](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-28-supported-world-intake.md)
now accepts a genuine producer bundle under the unchanged supported Molten
profile/WIT, retains exact component/WIT bytes, requires complete Structural+Bytes
agreement, and denies empty-directory source drift with private-lease cleanup.
This resolves import-free supported-world materialization, not execution or
actor/extension installation. The normalized effect-world probe is separately
denied by the selected artifact profile. Consumer-installation tasks remain
unchecked; no bundle rewrite, fabricated Bytes report, profile weakening,
runtime activation or completed shared adoption is admitted.

The effect-request world remains an unadmitted execution candidate. The inactive
host adapter below is independently exercised, but no production linker,
shared-runtime actor/extension installation or production rollback is claimed.
Cutover must move the immutable dependency,
adapter, profile/WIT, actor, extension, materialization and receipt cohorts
together after complete parity and product-consumer evidence. Rollback restores
that entire previous cohort, never one consumer or receipt producer alone.

## Contract-only effect-request world

**Immediate outcome:** Molten defines the candidate
`molten.wasm.component.effects.v1`, not an admitted execution profile.
[`effect-contract.ncl`](wasm-component-runtime/effect-contract.ncl) is the positive
export of the exact [`effect-contracts.ncl`](wasm-component-runtime/effect-contracts.ncl)
contract; [`generated/effect-contract.json`](wasm-component-runtime/generated/effect-contract.json)
is its deterministic projection. `execution_admitted=false` is mandatory.
The existing import-free production profiles/WIT, defaults, local runtime and
test-only shared adapter are unchanged. Defining or decoding this contract does
not install a linker, host adapter, actor or system extension.

**Durable capability:** the product-owned
[`wit/molten-component-effects/runtime.wit`](../wit/molten-component-effects/runtime.wit)
pins package `molten:component-effects@1.0.0`, world `actor-effects`, and source
identity `blake3:9478f7ddd9380bcbe898f2101d615674067e0f167ddc6f0fc345553751b5eaa2`.
Its only import is `effect-request` and only export is `invoke`; both have type
`func(list<u8>) -> result<list<u8>, string>`. No WASI or other ambient import is
permitted. The closed operation set contains only `effect_id="blob.get"`,
`operation="get"`, `capability="hostcall:blob-get"`.

The versioned canonical packed Preserves carriers have these ordered shapes
(uppercase names below denote fields, not literal symbols):

```text
<component-effect-request-v1 "molten.component-effect-request.v1"
  GENERATION INCARNATION SEQUENCE
  <import-grant "effect-request" "hostcall:blob-get"
    POLICY-REF AUTHORITY-REF RESOURCE-REF RECORDED-EFFECT-REF>
  EFFECT-REQUEST <blob-get OBJECT-CONTENT-REF>>

<component-effect-response-v1 "molten.component-effect-response.v1"
  REQUEST-CARRIER-REF EFFECT-RESPONSE OUTPUT-OPTION>
```

`GENERATION`, `INCARNATION` and `SEQUENCE` are unsigned 64-bit integers.
`EFFECT-REQUEST` and `EFFECT-RESPONSE` retain the existing canonical
`effect-request-v1` and `effect-response-v1` DTO semantics. Every reference must
parse canonically. The nested effect request's `input_ref` must equal the
canonical identity of the materialized `<blob-get OBJECT-CONTENT-REF>` payload.

There are two distinct request identities: `REQUEST-CARRIER-REF` is BLAKE3 over
the complete canonical request carrier, including its fences and sequence;
the nested effect response's `request_ref` names only the nested canonical
`effect-request-v1` value. They are not interchangeable. `OUTPUT-OPTION` is
exactly `<some OUTPUT>` or `<none>`. A `pass` response requires materialized
`OUTPUT` whose canonical identity equals the nested response's `output_ref`;
a `deny` response has neither an output reference nor materialized output.
WIT `Err(string)` denotes carrier/host failure, never a fabricated policy
denial or successful effect.

Each complete request or response packet is at most 65,536 bytes, with
`max_depth=32` (root depth zero) and `max_nodes=4096` values/nodes. These same
structural bounds apply to encoder output and decoder input. Annotations and
embedded values are rejected before recursive values are constructed; trailing
bytes and noncanonical encodings are rejected. Shape, identity and bounds
validation is not admission, and recording is not effect success.

A host adapter must freshly recheck the exact policy, Basalt/UCAN authority,
resource and recorded-effect references, generation/incarnation fences,
monotone request sequence, admitted artifact/handler/operation and capability
references before any dispatch through existing Molten effect ports. Replay
must bind complete request and response carrier identities together with
run/handler/turn/boundary and sequence, preserve the nested effect identity,
and never fall back to live effects. The inactive adapter below implements these
obligations; production execution activation remains a separate gate.

**Maintenance owner:** Molten Wasm component runtime maintainers
(`brittonr/aspen`) own this WIT, Nickel contract, codecs and effect-port meaning.
From the repository root, regenerate the projection and repeat the combined
positive export/readback, exact WIT-hash and negative-fixture rail:

```sh
nix develop -c nickel export docs/wasm-component-runtime/effect-contract.ncl --format json --output docs/wasm-component-runtime/generated/effect-contract.json
nix build .#checks.x86_64-linux.wasm-component-profile
```

The rail retains all existing profile checks and rejects effect-contract WASI
imports, extra operations, execution activation, and changed depth/node bounds.
It proves contract consistency, not execution admission or consumer adoption.

## Inactive effect host adapter

`wasm_component::host::Session` implements the closed `blob.get` operation through
the existing `NativeCallbackValuePort`; it installs no engine, linker or WASI.
Host-owned `CurrentInput` supplies the installed effect manifest and handler
profile, exact artifact/grant/capability references, current generation and
incarnation, finite limits, Nickel-exported Basalt policy, trusted issuer/holder
and caller-owned UCAN verification, revocation, caveat and replay services.
Guest-supplied references must match these independently measured current facts.

The resource is `molten://blob/OBJECT-CONTENT-REF`, ability `blob/get`, Basalt
contract `component-blob-get`. The pinned compact Ed25519 UCAN profile verifies
the token, holder signature and consume-once nonce. The holder signs the complete
canonical carrier plus run, handler, turn and boundary identities. Verification
time and clock-skew tolerance are explicit caller policy, not ambient host time.
The policy fixture is authored in `wasm-component-runtime/host-policy.ncl`; its
generated JSON is checked by the existing component-profile Nix rail:

```sh
nix develop -c nickel export docs/wasm-component-runtime/host-policy.ncl --format json --output docs/wasm-component-runtime/generated/host-policy.json
nix develop -c cargo test --locked -p molten --lib -- wasm_component::effect::host
```

`system_extension::CapabilityCallbackValuePort` provides bounded canonical value
storage through a supplied Ledger namespace capability, with no-follow reads and
independent byte measurement. Admission precedes dispatch. Canonical request and
intent publication precede the protected blob read; response and context-bound
completion publication precede returning success. Authority denial returns a
canonical deny with no output or blob read. A post-reservation port or recording
failure poisons the session: uncertainty is neither policy denial nor permission
to retry. Publication is process-visible, not durable or transactionally atomic.

`wasm_component::host::Replay` accepts an independently identified completion
record and exact recorded response bytes. It checks scope, lifecycle, monotone
sequence, outer and nested request identity, materialized output and intent
linkage, stopping at the first divergence. It accepts no live effect port or
authority input. The ordinary deterministic effect-log DTO remains a separate
compatible projection, not a replacement for this complete carrier binding.

**Immediate outcome:** signed admitted retrieval, denied retrieval and exact
offline playback through real capability-backed bytes. **Durable capability:**
reusable current-admission and uncertainty boundaries owned by Molten's component
runtime maintainers; native-host maintainers own the shared value-port mechanism.
The [host continuation evidence](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-27-host-adapter.md)
records executed cases and repeatability. No effect-world execution activation,
production actor/extension installation, durable nonce-store selection, full
runtime adoption, host isolation or release eligibility is claimed.


## Materialization and evidence

Production execution accepts only portable component and WIT bytes remeasured
from a complete Mantle bundle. Loose bytes are test-only and cannot produce
production evidence. Bundle references are externally linked rather than
embedded recursively, so circular self-attestation is rejected.

The published producer schema `mantle-wasm-component-materialization-bundle-v1`
has an explicit `VerifiedBundle::load` intake; it is not translated
into the older Molten `MantleBundle` DTO. The caller supplies source and private
staging-parent directory capabilities plus an admitted fresh staging leaf, and
must exclusively lease that staging namespace throughout the call. Source
parents and leaves are opened no-follow; every declared member is remeasured
under finite byte bounds and copied into fresh private staging. Mantle owns
directory source-closure framing and bounds (10,000 entries, depth 64 and
536,870,912 content bytes), including empty directories; only that role may
contain a tree. Its `ConsumerRoot::copy_source_closure` receives the unchanged
published bundle and opened source/destination roots. Existing file-valued
source closures remain supported. Molten keeps its cap-std v3 capabilities and
bridges their live descriptors to the provider's own root implementation;
it neither upgrades product capabilities nor exposes implementation-core DTOs
to pure product types. Other roles remain no-follow regular-file measurements,
with shared-path identity conflicts and finite member/aggregate byte admission.
The complete published verifier sees only the resulting private snapshot, never
mutable source parents. Ordinary success and failure remove only the owned child;
cleanup failure denies the load. The bridge requires Linux `/proc/self/fd`.

Molten component-runtime maintainers own private snapshot admission, selected
component/WIT capture and complete Structural+Bytes report agreement; Mantle
maintainers own source-tree identity and bounded no-follow copying. The focused
regression corpus retains unchanged genuine producer bytes and rejects tree
drift (including added empty directories), symlink substitution, missing or
unsupported nodes and over-bound inputs. These implementation cases are not
real-pipeline production acceptance, actor/extension installation or cutover;
the [original immutable probe intake](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-27-source-tree-intake.md#immutable-publication-and-executed-pinned-intake)
and [current supported-world intake](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-28-supported-world-intake.md)
separately prove private intake, not runtime invocation or consumer installation.

The intake requires the actual published Structural and Bytes reports, exact
role/digest/length observations, the admitted Molten profile digest and WIT,
and portable component bytes. Structural success alone, missing bytes, profile
drift, AOT, or a reused stage denies. The producer bundle identity is preserved
unchanged; component and WIT snapshots remain owned through invocation. The
separate Molten admission envelope still supplies policy, authority, resource,
Valence and Cairn references. Neither verification nor a consumer label
installs an actor, extension or shared-runtime backend.

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
- `molten.wasm.component.time-sliced.v1` requires a component admitted under the
  time-sliced profile;
- mismatches deny without fallback or silent reinterpretation.

Both actor and system-extension consumer classes pass through the same
materialization, admission, runtime, and receipt rails. This establishes
profile-level readiness only; it does not claim that every existing actor or
system extension has migrated.

## Time-sliced execution profile

The time-sliced profile is an admitted profile identity, not a flag on the
synchronous one. It binds a scheduling quantum and an independent total
execution budget, and admission denies it when either binding is missing or
malformed. The existing component profile is unchanged and keeps its identity,
and a time-sliced request under the synchronous profile is denied with a typed
profile mismatch, as is a synchronous request under the time-sliced profile.

The pure contract around the binding lives in `src/wasm/component/slicing.rs`:
the slice ledger records the quantum boundaries and refuses to exceed the total
budget, the pending turn publishes nothing before the callback completes and
publishes exactly once, and the yielded continuation is re-enqueued as an
ordinary runnable with the admitted scheduler. A replay compares the recomputed
slicing with the recorded one and names the first boundary that differs.

r[impl molten.wasm_component.execution_quantum] `start_sliced_execution` runs a component under the time-sliced profile. It passes the same admission, materialization, and receipt rails as a synchronous execution. The store suspends the guest every quantum of fuel through Wasmtime's fuel-based asynchronous yielding, and the guest's `invoke` export stays a synchronous WIT export: the host drives it through the asynchronous entrypoint and polls one slice at a time.

`SlicedExecution::run_slice` either yields or completes:

- A yield records a quantum boundary and returns the scheduler's `Yield` command for the continuation. The admitted scheduler returns the running continuation to the ready queue behind the work already waiting.
- Completion records the final partial slice, so the ledger's consumption equals the fuel the store spent. The turn publishes exactly once.

A spent budget at any slice keeps the existing `fuel-exhausted` classification. A resumed slice continues from its suspension point: the fuel a sliced run spends equals the fuel of the same synchronous run.

A quantum is a work-accounting bound, not elapsed CPU time or a latency
guarantee. Yielded continuations do not prove fairness of the whole system, and
time slicing adds no memory or authority isolation.

Focused positive and negative tests live under `src/wasm/component/tests/`.
Nickel rejection fixtures cover stale or incomplete cohorts, ambient WASI, and
nondeterministic growth under
`docs/wasm-component-runtime/fixtures/negative/`.

## Standalone extracted-runtime boundary pilot

<!-- r[impl molten.hwr_consumer_pilot.actor] -->
<!-- r[impl molten.hwr_consumer_pilot.boundary] -->
<!-- r[impl molten.hwr_consumer_pilot.pin] -->
<!-- r[impl molten.hwr_consumer_pilot.evidence] -->

The Molten Wasm component runtime maintainers (`brittonr/aspen`) own
[`pilots/hwr-consumer/`](../pilots/hwr-consumer/) and the native Cairn archive
[`pilot-hwr-consumer`](../.cairn/archive/1970-01-01-pilot-hwr-consumer/verification.md).
This isolated Cargo workspace is not a root dependency or a production host
entrypoint. The existing `adopt-shared-wasm-component-runtime` change remains
separate and incomplete; this pilot does not satisfy its cutover gates.

The [adoption compatibility preflight](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-26-compatibility-preflight.md)
records two published-API blockers: no resumable fuel-quantum invocation, and no
failure-phase observation for Molten's staged receipts. An executable probe
observed the same normalized trap class for instantiation and invocation traps.
The local runtime and its coherent profile/receipt cohort remain unchanged.

**Immediate outcome:** the corrected external API executes the Molten actor
fixture at immutable producer revision
`ce9b222be993c43e9960ce7c12aed2a5130ecf76`. Both dependencies and `source.ncl`
select that revision and its explicit Radicle publisher namespace. The command
returned `Completed`, exact bytes `070000002a000000`, and 9,973 remaining fuel
from 10,000; malformed, oversized and missing-epoch cases returned their exact
denials. This is pilot execution, not production host adoption.

The manifest, pilot lockfile and provenance record were reviewed together;
the locked execution resolved the qualified external source without a path
override. These source-identity observations do not establish source trust.

**Durable contribution:** Molten-owned `actor` world, `actor-turn` export, WIT
identity, embedded actual Component Model guest, explicit feature policy and
resource limits, exact import/export materialization facts, consumer-operated
epoch handle, and exact actor-output and denial assertions against the reusable
mechanism. The synthetic eight-byte turn preserves actor id 7 and changes counter
41 to 42. It is deliberately not the production Preserves actor ABI.

From the repository root, run the locked standalone consumer:

```sh
CARGO_NET_GIT_FETCH_WITH_CLI=true nix develop -c cargo run --locked --manifest-path pilots/hwr-consumer/Cargo.toml
```

Prerequisites are the repository Rust cohort and linker, the Radicle remote
helper and access to the manifest's exact publisher namespace and revision,
plus registry dependencies. The unqualified Radicle adapter exposes canonical
main only; the explicit publisher also exposes the immutable candidate branch.
The producer push reported no seeds, so network replication is not claimed.
The standalone command does not build the root Molten
workspace. It generates actual component binary bytes from the embedded WAT,
hashes those exact bytes, admits no imports/WASI, requires the exact root export,
and runs with 10,000 fuel, eight-byte input/output limits, one fixed 64-KiB memory,
and an explicitly operated 1,000-tick epoch deadline. The consumer's driver ticks
every 10 ms and is stopped and joined before outcome assertions.

<!-- r[verify molten.hwr_consumer_pilot.actor] -->
<!-- r[verify molten.hwr_consumer_pilot.boundary] -->

Success prints the exact `Completed` output `070000002a000000` (`bytes=8`),
observed positive remaining fuel below 10,000, `GuestDenied` for a malformed turn,
`InputLimitExceeded` for oversized input, and `DeadlineDenied` for an absent epoch
handle. These outcomes were observed in the pinned standalone run. Actor policy,
WIT/schema binding, and output meaning remain consumer-owned; producer success
does not independently prove a WIT world or confer product authority.

`profile.ncl` owns the pilot policy and `profile.json` is its wire projection
(`nickel export pilots/hwr-consumer/profile.ncl`). Its `production` and `nickel`
markers are required by the existing extracted document schema; they grant no
production evidence scope to this fixture. WIT changes require remeasuring
`wit/actor.wit` with BLAKE3 and updating its policy reference and `src/main.rs`
identity together. The fixture is embedded loose bytes, not a Mantle bundle.
Molten retains all production materialization trust, policy, Basalt/UCAN,
canonical Preserves, actor lifecycle, receipt, replay, and release decisions.

**Non-claims:** no production cutover, local/shared parity, Mantle provenance,
policy or authority admission, canonical product receipts, actor persistence or
effect execution, host isolation, performance result, whole-system safety, or
release eligibility. No main runtime source, root dependency cohort, or historical
archive is changed by this pilot.
