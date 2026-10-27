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
verifier at `c99556e5c91dfd13fcc261dc451f471359d47c38`. The source gate binds
their completed native archives and exact evidence digests. These pins are
prerequisites, not authorization to change the execution backend.

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
Public execution and both existing consumer paths still use the complete local
runtime. The [earlier frozen mapping and counterexamples](../.cairn/changes/adopt-shared-wasm-component-runtime/evidence/2026-10-26-runtime-adoption-parity.md)
remain historical evidence; they are not current v4 failures. Operated product
deadlines, the effect world, matching production materializations, real shared
actor/extension installation and coupled cutover/rollback remain acceptance gates.

No effect-request world, shared-runtime actor or extension installation, or
production rollback is claimed. Cutover must move the immutable dependency,
adapter, profile/WIT, actor, extension, materialization and receipt cohorts
together after complete parity and product-consumer evidence. Rollback restores
that entire previous cohort, never one consumer or receipt producer alone.

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
under finite byte bounds and copied into fresh private staging. The published
path-based verifier sees only that descriptor-backed snapshot, not mutable
source parents. Ordinary success and failure remove only the owned child;
cleanup failure denies the load. This bridge requires Linux `/proc/self/fd`.

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
