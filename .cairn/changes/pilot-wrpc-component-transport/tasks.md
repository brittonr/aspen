## Tasks

- [ ] [serial] Define the pinned wRPC/WIT pilot profile, unary call world, transport-port declaration, compatibility cohort, resource bounds, and explicit experimental non-claims. r[molten.component_rpc.profile] r[molten.component_rpc.async_scope]
- [ ] [depends:component-rpc-profile] Define Mantle inputs for client/server components, generated bindings, WAC composition, and deny-all WASI virtualization; implement exact materialization-bundle admission and reject loose or mismatched production artifacts. r[molten.component_rpc.materialization]
- [ ] [depends:component-rpc-profile] Implement pure WIT-wire/Preserves mapping, invocation identity, authority/resource request, transcript, replay, and pilot-classification cores. r[molten.component_rpc.bridge] r[molten.component_rpc.authority] r[molten.component_rpc.transcript] r[molten.component_rpc.functional_core]
- [ ] [depends:component-rpc-materialization] Add the opt-in wRPC runtime adapter over an admitted fabric transport session without rerouting existing Molten protocols or building guest artifacts. r[molten.component_rpc.transport]
- [ ] [depends:component-rpc-authority-core] Enforce peer, component, world, function, policy, Basalt/UCAN authority, resource, delivery, retry, and replay admission before live transport effects. r[molten.component_rpc.authority]
- [ ] [depends:component-rpc-bridge] Emit canonical bridge/call receipts binding both wire representations, component/WIT identity, transport session, authority/resource, delivery, and terminal result. r[molten.component_rpc.transcript]
- [x] [parallel] Add optional redacted OpenTelemetry-WASI export with bounded queues, diagnostic-only roles, canonical receipt links, and payload exclusion tests. r[molten.component_rpc.telemetry]
  - Executed on the published Mantle emitter on 2026-11-18: RPC-only authority, the secret attribute and the 33rd span were denied with nothing exported, and one safe span was exported and read back with its receipt link. See the [real-bundle run](evidence/real-bundle-run-2026-11-18.json).
- [x] [parallel] Add positive Mantle-materialized unary loopback/replay fixtures and negative incomplete/mismatched bundle, local guest build, wrong-world, malformed mapping, unauthorized peer/function, over-budget call, duplicate/reordered transcript, stream/future use, telemetry leak, and live-timing replay fixtures. r[molten.component_rpc.materialization] r[molten.component_rpc.validation]
  - Executed on the published Mantle client/server bundle on 2026-11-18: one admitted call returned `hello from Rust`, offline replay verified it, and all 33 negative cases, covering every listed class, were rejected. See the [real-bundle run](evidence/real-bundle-run-2026-11-18.json).
- [ ] [depends:component-rpc-validation] Record pilot graduation/retirement criteria and run focused adapter, transport-session, replay, authority, telemetry, Octet, Cairn validation, and lifecycle gates. r[molten.component_rpc.nonclaims] r[molten.component_rpc.validation]

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

Tasks 9 and 10 close on that record. The other tasks still need review against
it criterion by criterion; the run uses the loopback fixture fabric, so it is
not remote or cross-host RPC evidence. Mantle's telemetry toolchain pins Octet
`f43a1d9e`, which awaits review in OnixResearch/octet#32 and is not yet on
Octet `main`.
