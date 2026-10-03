# Extension integration input reference

Mode: Reference

Use this reference when assembling an integration handoff: identify which layer owns each input, what must match, and which artifact answers the next review question. It complements the admission theory in [Manifest and tier admission](../../technical/extensions/manifest-and-tier-admission.md), rather than replacing the [system-extension contract](../../system-extension-runtime.md). Navigation: [Handbook](../README.md).

**Scope:** source-checked field and artifact inventory, not an executed integration. Names below are Rust fields, profile identifiers, or emitted filenames as identified; they are not a new JSON installation format. Canonical Preserves bytes and BLAKE3 identities, not Rust layout, define identity.

## Execution-path selection

| Selection | Owning surface | Required interpretation |
| --- | --- | --- |
| `in-process-native` | Generic manifest/executor profile | Trusted native execution, not sandboxing |
| `native-process` | Native host plus execution fabric adapter | One bounded process per callback under admitted executable context |
| `sandboxed-component` in deterministic fixture | `EchoExecutor` and `WasmProbe` | Fuel-bounded no-import core-module probe; Rust builds outcomes |
| `molten.wasm.component.v1` | Component profile and runtime shell | Component artifact, WIT cohort, materialization and receipt rails |
| `molten.wasm.abi.v1` | Legacy artifact classification | Core-module ABI; not interchangeable with Component Model |

The CLI value enum includes `native-process`, but `run_executable_system_extension_fixture` refuses it. The native service methods documented in the governing page are not additional CLI subcommands. Do not translate method names into invented install, recover, or journal flags.

## Generic manifest inputs

The concrete fixture's `manifest_input` illustrates these fields. Synthetic fixture references are examples of input shape, not usable deployment evidence.

| Field group | Owner supplying it | Review join |
| --- | --- | --- |
| `schema`, `extension_id`, `service_id`, `implementation_ref` | Extension integrator | Exact schema and implementation identity |
| `callback_groups` | Extension implementer | Declared lifecycle and traffic callbacks |
| `required_ports`, `optional_ports` | Integrator and fabric admission | Exact port/version, operation, schemas, resources, and profile |
| `capability_refs`, `policy_refs`, `provenance_refs` | Respective evidence authorities | Independently supplied evidence, not a receipt returned by the extension |
| `resources`, `execution_profile` | Admission owner | Finite envelope and separately admitted executor profile |
| `state_schema`, `compatible_state_schemas` | State/migration owner | Explicit continuity compatibility |
| `evidence_profile_ref`, `initial_generation`, `non_claims` | Integration admission | Evidence cohort, initial fence, and bounded claims |

Tier admission is an additional input to canonical manifest admission, not something granted by an implementation reference. Optional ports may be absent; available incompatible ports do not authorize silent substitution.

## Callback and effect inputs

| Structure | Significant fields | Owner and boundary |
| --- | --- | --- |
| `CallbackEvent` | `callback`, `generation`, `event_ref`, `payload_ref`, `accounted_bytes`, `logical_tick`, `deadline_tick`, `cancellation_requested` | Caller presents it to pure dispatch admission |
| `CallbackInvocation` | Callback identity plus `sequence` and required deadline | Dispatch plan supplies it only when scheduled |
| `TypedEffectRequest` | `target`, `operation`, input/output schema refs, `request_ref`, `generation`, `accounted_bytes` | Executor proposes; host validates before approval |
| `CallbackOutcome` | Output refs, effects, optional state/checkpoint refs, health | Executor return is untrusted until admitted |

A logical deadline is not an operating-system timeout. The source compares deadline with logical tick and the manifest's allowed interval. Physical process bounds belong to the native execution adapter. Similarly, typed effect approval is not provider execution or cross-provider atomicity.

## Native materialized-value inputs

The `native-host-local-pilot-v2` profile uses ALPN `molten/system-extension/native/v2` and framing `preserves-packed-materialized-values-v2`. Its protocol does not have a reference-only fallback.

| Input or result | Owner | Necessary pairing |
| --- | --- | --- |
| `NativeCallbackContext` | Admitted host template and instance | Manifest/executable/instance identities, state ref, policy/resource/port refs |
| `NativeCallbackInputs` | Value-port materialization | Optional payload and prior-state values must match required refs |
| `NativeCallbackValue` | Value producer, independently checked by consumer | `value_ref` with exact `bytes` |
| `NativeMaterializedCallbackOutcome` | Child producer | Outputs, effect bodies, next state, checkpoint, and health |
| `NativeOperationRecord` | Host journal transitions | Operation/parent refs, kind, generation, state, terminal ref, retry flag |

The independent native fixture reads one bounded envelope from stdin and writes one canonical packed outcome to stdout. Diagnostics belong on stderr, not mixed into the protocol frame. Its state construction is fixture-specific byte concatenation, not a general application schema recommendation.

## Component integration cohort

The Nickel profile groups toolchain versions, WIT package/world/source reference, admitted features, deterministic settings, resource bounds, allowed imports, and non-claims. The first cohort allows no imports or WASI. Production materialization requires a complete Mantle bundle and external evidence roles; loose component bytes are test-only. Matching a world string does not replace byte remeasurement or nested resource inspection.

## Artifact lookup

| Artifact or API | Owner | What it answers |
| --- | --- | --- |
| `manifest.preserves` | Fixture CLI writer | Which initial manifest was admitted? |
| `upgraded-status.preserves`, `rolled-back-status.preserves` | Fixture CLI writer | Which transition snapshots were captured? |
| `recovered-status.preserves`, `status.preserves` | Fixture CLI writer | Recovery snapshot versus final stopped snapshot |
| `evidence/{index}-{kind}.preserves` | Fixture CLI writer | Indexed host evidence, not a native journal export |
| `NativeHostJournal::history` | Configured journal adapter | What instance records were saved over time? |
| Component inspection/instantiation/execution/denial receipts | Component runtime | Which stage and bounded observation were recorded? |

## Worked handoff check

An integrator supplies a native outcome containing an effect request reference but no request bytes. Generic reference validation may explain why the reference looks well-shaped; it cannot satisfy native v2. The native outcome decoder and materialization admission are the relevant owners. Ask for exact bytes with matching identity and the admitted bounds, not a generic fixture receipt or a fallback profile. This is an input-cohort failure, not permission to relax publication requirements.

## Sources

- [Handbook](../README.md)
- [Manifest and tier admission](../../technical/extensions/manifest-and-tier-admission.md)
- [System-extension runtime](../../system-extension-runtime.md)
- [Native host contract](../../native-system-extension-host.md)
- [Fixture manifest fields](../../../src/system_extension/parts/fixture/p001/body.rs)
- [Callback and effect model](../../../crates/molten-core/src/system_extension/dispatch.rs)
- [Native protocol fixture and tests](../../../src/system_extension/native_host/tests.rs)
- [Independent native producer](../../../src/bin/molten-native-extension-fixture.rs)
- [Component profile template](../../wasm-component-runtime/profile-template.ncl)
- [CLI artifact inventory](../../../src/cli/runtime/system_extension/ops.rs)
