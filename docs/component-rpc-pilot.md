# Component RPC pilot

## Scope and exact cohorts

`component-rpc` is opt-in. Defaults and existing dataspace, node-control, job,
service and local component protocols are unchanged. The only RPC is
`wrpc-examples:hello/handler.hello: func() -> string`.

The selected upstream is [wRPC v0.15.0](https://github.com/bytecodealliance/wrpc/tree/aa01409ed38bf7a978570ff567c4475376925df6):
`wrpc-runtime-wasmtime` 0.29.0, `wrpc-transport` 0.28.4 and isolated Wasmtime
33.0.0. This runtime must never receive shared Wasmtime 45 engines, stores,
components or handles. Streams, futures, implicit cancellation and transport
extensions are rejected. See `config/component-rpc/profile.ncl` and the two
explicit Basalt policy sources in that directory.

Mantle owns guest compilation, bindings, WAC composition, WASI virtualization
and all production artifact bytes. Molten contains no guest-build command.
Mantle's `wrpc-reference-bundle` supplies:

- `manifest.json`, schema `mantle-wrpc-reference-bundle-v1`;
- standalone `client/materialization-bundle.json` and server equivalent;
- each role's `runtime-profile.json` and exact selected world WIT;
- the complete source/binding/composition/virtualization member closure;
- `capability/`, mirroring immutable logical member paths under an authorized
  root instead of requiring a filesystem `/` capability.

The standalone client exports `run() -> u32`, calls the imported remote hello,
checks `hello from Rust` and returns 42. The server exports the actual hello
function. WAC-composed artifacts are comparison-only, never remote evidence.
`Bundle::load_package` reuses the existing published Mantle structural/byte
verifier and private no-follow snapshots. Loose, missing, wrong-cohort and
comparison artifacts do not become an admitted bundle.

## Real invocation and evidence

The existing fabric frame API acknowledges delivery by echoing a frame. An
acknowledgement is **not** an RPC result. The pilot uses this explicit flow:

1. Check current policy, signed UCAN issuer/audience/grant, holder signature,
   replay backend, exact components/WIT, both peer identities, generation,
   resource contract, delivery policy, invocation and dedup context.
2. Persist a canonical intent through a held output directory capability.
3. Send the canonical request on an admitted fabric session.
4. Verify the inbound peer and request mapping; invoke the actual measured
   server using `wrpc_runtime_wasmtime::call` and its real value codec.
5. Carry the independently encoded result on the admitted reverse session.
6. Return it to the materialized client's import through upstream
   `wrpc_runtime_wasmtime::link_item` and `wrpc_transport::Invoke`.
7. Persist participant transcripts and terminal observations. Publish the
   aggregate report last.

The hello wire request is empty; the Preserves envelope is not. Results are a
minimal unsigned LEB32 string length plus exact UTF-8 bytes. Strict admission
rejects nonminimal lengths, overflow, truncation, trailing bytes and excessive
payloads. Wire digests and canonical Preserves envelope identities remain
separate. Mapping success is not a general equivalence proof.

Scopes bind caller/callee components, WIT pair, complete bundle, policy,
resources, caller/server Iroh identities, request and derived response
sessions, delivery, dedup, generation and logical deadline. The actual authority
module uses Basalt enforcement and UCAN token, holder-signature and replay
verification, not caller-provided booleans or a permissive authority trait.
Leases are non-clonable and each client session is consumed at most once.

Every new identity uses canonical packed Preserves through `crate::codec`.
The existing shared `wasm_component::r#ref` still hashes Debug-rendered profile
fields; this inherited provider compatibility contract was not changed. It
must not be advertised as a stronger new canonical-profile identity.

Failures after an attempted request are uncertainty requiring reconciliation,
not a new policy denial or an invitation to retry. Automatic retries are zero.
Journal intent/live/terminal files are create-new and synced. A missing
terminal file after a crash does not prove that nothing happened. This rail
makes no exactly-once or crash-recovery claim.

## Operator integration contract

The root command owner mounts `src/componentrpc/mod.rs` as `component_rpc` and
resolves explicit user-selected directory roots to held capabilities. Domain
code never discovers ambient roots or reconstructs paths after admission.

Recommended root commands and their exact domain calls:

| Command | Domain call |
| --- | --- |
| `component-rpc run-fixture --bundle-root B --staging-root S --output-root O` | `operator::run(&B, &S, &O).await` |
| `component-rpc show --output-root O` | `operator::show(&O)` |
| `component-rpc replay --bundle-root B --staging-root S --output-root O` | `Bundle::load_package(&B, &S)`, then `operator::replay(&O, &bundle)` |

These commands require root dispatch integration; their names here are the
integration contract, not a claim that an unbuilt binary exposed them.
Output roots must be fresh. Readback validates both canonical transcript refs
and their recorded values. Offline replay decodes persisted Preserves, checks
exact bundle/component/WIT scope, and consumes recorded effects without
opening listeners, consulting wall time or granting live authority.

`run-fixture` uses fixed test-only keys and verification time to exercise real
signed UCAN admission. It is not a production credential fallback. It runs
actual wrong-grant, wrong-peer, invalid-holder-signature, duplicate-nonce,
current-policy-deny, missing-token, expired-authority and resource-deny gates.
Actual `Invoke` calls with wrong world/function and excessive arguments must
leave the valid session unused; the subsequent real call must still succeed.
Package and transcript negative cases are evaluated against the actual
handoff/completed transcript, never fabricated successful evidence. The
focused tests in `tests/componentrpc.rs` cover exact mapping boundaries,
unsupported call shapes and bounded diagnostic fields.

## Optional actual OpenTelemetry-WASI export

`component-rpc-telemetry` additionally enables a separate Mantle emitter bundle;
it does not widen either hello guest's imports. The selected interface is
[WASI OTel revision e5262a8](https://github.com/WebAssembly/wasi-otel/tree/e5262a84044aff064c9599eebc05013bb3432795),
`wasi:otel/tracing@0.2.0-rc.2`. The real callbacks are `current-span-context`,
`on-start` and `on-end`. Type-only `wasi:otel/types` and wall-clock datetime
imports are admitted only with no effect functions/resources. Actual clocks,
network access and arbitrary telemetry interfaces remain denied.

Mantle's `wrpc-telemetry-reference-bundle` has schema
`mantle-wrpc-telemetry-reference-bundle-v1`, role `telemetry`, standalone
`run(mode: u32) -> u32`, and an unchanged capability mirror. Its exact emitter
uses safe mode 0, secret-attribute mode 1 and 33-span mode 2. A producer's WAC
comparison sink is not the trusted consumer host.

The exporter requires a separate current policy and signed UCAN ability
`component-rpc/diagnostics-export`, scoped to emitter, canonical receipt and
sink capability. RPC invocation authority cannot authorize telemetry export.
The real tracing callbacks enforce a single active span, trusted context,
32-span queue, fixed approved names, payload-minimized attributes, no events
or links and no application-acceptance status. Unknown attributes, including
`authorization`, are denied. Only a matching canonical receipt attribute is
allowed. Trace IDs, timestamps, credentials and raw application payloads never
enter exported records. Completed output is bounded to 64 KiB and written
create-new to `otel-diagnostic.preserves` through the separately admitted sink.
This is a WASI OTel diagnostic file exporter, **not an OTLP protocol claim**.

Root integration for `component-rpc telemetry-fixture` calls
`telemetry::operator::run_fixture(telemetry::operator::Input { package, staging,
rpc_output, sink, sink_ref }).await`. `telemetry-show` calls
`telemetry::operator::show(&sink)`. The fixture first proves RPC-only authority
is denied, then checks the actual mode-1 secret rejection and mode-2 queue
rejection before running safe mode. It checks no diagnostic file appeared for
those failed modes, reads back safe output and verifies that the existing RPC
report was unchanged. It publishes a diagnostic-only fixture summary, not a
new application acceptance receipt.

The independent `diagnostics` module is merely a bounded native projection;
it is not mislabeled as WASI OTel or OTLP. Export failures cannot mutate,
replace or grant authority from the canonical RPC receipt.

## Graduation, retirement and verification

Promotion requires exact-cohort real client/server conformance, complete
negative admission coverage, canonical replay, bounded uncertainty handling,
independently admitted telemetry behavior and a written overlap decision for
existing Iroh protocols. Any unsupported async dependency, policy regression,
required local guest build, unbounded exporter or unstable cohort keeps the
pilot experimental or retires it. Success proves neither protocol stability,
semantic equivalence, transport security, production readiness, component
correctness nor replacement eligibility.

No builds, runtime smoke, tests, formatters, linters or lifecycle mutations were
run by this delegated consumer implementation, as explicitly instructed.
Root pins/mounts/dispatch, Mantle realization, the published verifier's current
capability/source-tree API, focused adapter/authority/replay/export execution,
Octet and Cairn gates must be integrated and exercised before task completion.
Source files and executable fixture paths are not execution evidence.
