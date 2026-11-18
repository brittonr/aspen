## Tasks

- [x] [serial] Define the pinned wRPC/WIT pilot profile, unary call world, transport-port declaration, compatibility cohort, resource bounds, and explicit experimental non-claims. r[molten.component_rpc.profile] r[molten.component_rpc.async_scope]
  - Reviewed on 2026-11-18: `config/component-rpc/profile.ncl` declares the wRPC v0.15.0 cohort, the `hello` unary world, the fabric port, the bounds and the six non-claims, and every declared value that a run can observe matched what the [admission run](evidence/real-bundle-admission-run-2026-11-18.json) admitted. That run rejected cohort drift, a wrong world, stream/future paths, an async world, a production overclaim and, at endpoint import before any dial, a drifted transport profile. Nothing ties the Nickel file to the Rust constants automatically; the comparison is a one-off.
- [x] [depends:component-rpc-profile] Define Mantle inputs for client/server components, generated bindings, WAC composition, and deny-all WASI virtualization; implement exact materialization-bundle admission and reject loose or mismatched production artifacts. r[molten.component_rpc.materialization]
  - Executed on the published Mantle bundle: `Bundle::load_package` remeasured, through the shared verifier, every member that the client and server materialization bundles declare, and admitted the standalone client and server only with the pinned manifest, cohort and worlds, a deny-all WASI profile (no allowed WASI interfaces; the server keeps no imports) and WAC output marked comparison-only. The manifest's own 262-entry member list is only bounded, not remeasured. Incomplete, missing-client, loose composed, cohort-drift and local-build manifests were rejected; the shared verifier's tests reject drifted member bytes. Mantle's `materialize-wrpc-producer` owns the producer inputs.
- [x] [depends:component-rpc-profile] Implement pure WIT-wire/Preserves mapping, invocation identity, authority/resource request, transcript, replay, and pilot-classification cores. r[molten.component_rpc.bridge] r[molten.component_rpc.authority] r[molten.component_rpc.transcript] r[molten.component_rpc.functional_core]
  - Reviewed and executed: the mapping, invocation, authority request and binding, transcript, replay and classification functions take explicit inputs and perform no filesystem, network, process, clock or runtime effects; UCAN verification receives injected time, keys, revocations and replay admission. The separate `replay` process recomputed the same invocation, transcript and result identities from the persisted bytes, and the mapping, transcript and classification negatives were rejected.
- [x] [depends:component-rpc-materialization] Add the opt-in wRPC runtime adapter over an admitted fabric transport session without rerouting existing Molten protocols or building guest artifacts. r[molten.component_rpc.transport]
  - Executed: the measured client reached the measured server through `wrpc_runtime_wasmtime` over two admitted Iroh fabric frame sessions (request, then reverse result), and both transcripts keep the frame evidence. The adapter exists only behind the non-default `component-rpc` feature and its CLI subcommand; no node protocol registration uses its ALPN, and Molten builds no guest. Loopback only, not remote or cross-host RPC.
- [x] [depends:component-rpc-authority-core] Enforce peer, component, world, function, policy, Basalt/UCAN authority, resource, delivery, retry, and replay admission before live transport effects. r[molten.component_rpc.authority]
  - Executed before any frame was sent: a wrong UCAN grant, an ungranted peer and component, an invalid holder signature, a duplicate nonce, a current Basalt policy deny, a missing token, expired authority, resource and delivery contract drift, an unmeasured callee, a wrong world and function, and oversized arguments were denied; a second attempt after the call was denied without journal or transport effects. Molten `43482e4b1` added the component, delivery and retry cases. Fixture keys and verification time only, not production credential admission.
- [x] [depends:component-rpc-bridge] Emit canonical bridge/call receipts binding both wire representations, component/WIT identity, transport session, authority/resource, delivery, and terminal result. r[molten.component_rpc.transcript]
  - Executed: each participant transcript binds the scope (components, WIT pair, bundle, policy, resources, peers, sessions, delivery and dedup), the authority receipt, both frame evidence records, the Preserves request and result with their wire digests, and the ordered begin, delivery and terminal events. `show` read both back against the report and offline `replay` verified them; reordered, duplicate, cross-call, wire-drift, uncertain-delivery and live-timing transcripts were rejected.
- [x] [parallel] Add optional redacted OpenTelemetry-WASI export with bounded queues, diagnostic-only roles, canonical receipt links, and payload exclusion tests. r[molten.component_rpc.telemetry]
  - Executed on the published Mantle emitter on 2026-11-18: RPC-only authority, the secret attribute and the 33rd span were denied with nothing exported, and one safe span was exported and read back with its receipt link. See the [real-bundle run](evidence/real-bundle-run-2026-11-18.json).
- [x] [parallel] Add positive Mantle-materialized unary loopback/replay fixtures and negative incomplete/mismatched bundle, local guest build, wrong-world, malformed mapping, unauthorized peer/function, over-budget call, duplicate/reordered transcript, stream/future use, telemetry leak, and live-timing replay fixtures. r[molten.component_rpc.materialization] r[molten.component_rpc.validation]
  - Executed on the published Mantle client/server bundle on 2026-11-18: one admitted call returned `hello from Rust`, offline replay verified it, and all 33 negative cases, covering every listed class, were rejected. See the [real-bundle run](evidence/real-bundle-run-2026-11-18.json).
- [x] [depends:component-rpc-validation] Record pilot graduation/retirement criteria and run focused adapter, transport-session, replay, authority, telemetry, Octet, Cairn validation, and lifecycle gates. r[molten.component_rpc.nonclaims] r[molten.component_rpc.validation]
  - Graduation and retirement criteria are in `docs/component-rpc-pilot.md`. On 2026-11-18 the focused and unit tests, the real-bundle adapter, transport, replay, authority and telemetry run, Clippy and Octet over `component-rpc-telemetry`, strict Cairn validation and the proposal, design and tasks gates passed; see the [verification record](evidence/verification-2026-11-18.md). Octet first reported 56 distinct findings (110 across the library and test targets) in this never-checked feature; Molten `ed52b83c0` repairs 55 and allows one with a stated reason. The Iroh overlap decision belongs to a future promotion, so the pilot stays experimental.

## Source handoff note (2026-10-30)

Source implementation: `src/componentrpc/`, `config/component-rpc/`, `tests/componentrpc.rs`, and `docs/component-rpc-pilot.md`. Real wRPC and the separately selected optional WASI OTel host paths are authored against the exact Mantle contracts. Task boxes intentionally remain unchecked: no prohibited build/test/lint/formatter/lifecycle gate or producer realization was run by the delegated consumer worker. Root dependency/mount/command integration and actual bundle-backed positive/negative execution must be reviewed and recorded by the integration owner before claiming completion.

## Parent local controls (2026-11-17)

Root feature dependencies, public module mounts and command dispatch are present.
The [source/image-bound local observation](evidence/local-controls-2026-11-17.json)
records four passing wire/diagnostic controls under `component-rpc-telemetry`,
the actual five-command CLI help surface and Nickel profile typechecking.
The initial Nix-shell attempt failed before tests started because the default
temporary filesystem quota was exhausted; the passing attempt used task-owned
scratch on the repository filesystem without deleting unrelated data.

These controls do not execute a materialized client/server call, authority or
materialization negatives, transcript replay or the WASI OTel callbacks. The
shared Mantle verifier/source-tree API is already selected; the remaining
bundle admission and end-to-end acceptance criteria stay unchecked. No task is
closed by confusing a compiled optional feature or diagnostic queue with those
executions. Molten's component-RPC integration maintainers own this local recipe.

## Producer observation (2026-11-18)

Mantle `994bf653` materialized the client/server bundle and passed its
real-byte acceptance rail; see the
[integration record](https://github.com/OnixResearch/mantle/blob/994bf653770d5f7853a49ce9aa68e6c3e6335621/.cairn/changes/materialize-wrpc-producer/evidence/integration-2026-11-18.md).
The compiled telemetry emitter imports `wasi:otel/tracing@0.2.0-rc.2` plus
type-only `wasi:otel/types@0.2.0-rc.2` and `wasi:clocks/wall-clock@0.2.3`. The
telemetry host now admits that observed `0.2.3` clock interface, still only as
a type-only dependency, instead of `0.2.0`. The correction is compile-checked,
not executed: the telemetry bundle stops at Octet admission, which needs an
Octet-owned profile for those imports, so no telemetry task is closed.

## Real-bundle run (2026-11-18)

Molten `7e158a95c` ran `run-fixture`, `show`, `replay`, `telemetry-fixture`
and `telemetry-show` against Mantle `c376f843e`'s published reference and
telemetry bundles; the [run record](evidence/real-bundle-run-2026-11-18.json)
binds the source commit, binary image, producer revision, bundle identities and
outputs. The first real-bundle attempt exposed two consumer defects, fixed in
`7e158a95c` with regression tests: private staging could not remove the
read-only directories that faithful Nix store closure copies keep, and the
telemetry host rejected the tracing import because a real instance type also
exports its record, flags and variant types.

The telemetry and fixture tasks close on that record.

## Criterion review (2026-11-18)

Reviewing the other tasks against the first record found no executed denial
for component, delivery or retry admission, or for transport-profile drift.
Molten `43482e4b1` adds five fixture cases for them, and the
[admission run](evidence/real-bundle-admission-run-2026-11-18.json) re-ran the
same published bundles: all 38 negative cases were rejected, replay verified
and the telemetry fixture passed unchanged. Both runs use the loopback fixture
fabric, so neither is remote or cross-host RPC evidence. Mantle's telemetry
toolchain pins Octet `f43a1d9e`, which awaits review in OnixResearch/octet#32
and is not yet on Octet `main`.

The first Octet run with the feature enabled reported 56 distinct Tiger Style
findings (110 across the library and test targets),
because the configured Octet scope builds default features only. Molten
`ed52b83c0` repairs 55 of them without changing behaviour and allows one with a
stated reason; its real-bundle run
rejected the same 38 cases in the same order and replayed identically. Nothing
reruns that feature-enabled check automatically.
