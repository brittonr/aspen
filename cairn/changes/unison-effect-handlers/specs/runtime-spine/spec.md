# Runtime Spine Delta: Effect Handler Manifests

### Requirement: Effect manifests are canonical
r[molten.effects.manifest_model] Executable artifacts MUST describe admitted effects with canonical `effect-manifest-v1` records that bind artifact kind, artifact ref, executor kind, declared effect ids, operation names, schema refs, policy refs, evidence refs, and no-Unison-runtime-compatibility checks.

#### Scenario: Manifest identity is stable
- GIVEN an executable artifact and its declared effects
- WHEN Molten renders the effect manifest
- THEN the manifest has a stable canonical content ref
- AND records that Unison abilities are non-normative prior art only.

### Requirement: Effect ids are stable and declared
r[molten.effects.effect_ids] Declared effect ids MUST be lowercase deterministic ids bound to operation names and input/output schema refs, and duplicate effect-id/operation pairs MUST fail closed.

#### Scenario: Duplicate declared effect is rejected
- GIVEN a manifest containing the same effect id and operation twice
- WHEN Molten validates the manifest
- THEN validation denies the manifest before it can admit handler bindings.

### Requirement: Artifacts link effect manifests
r[molten.effects.artifact_link] Artifact records for executable Wasm, Steel, native, choreography, job, adapter, or remote-proxy code MUST link admitted effect manifests by content ref instead of relying on ambient runtime knowledge.

#### Scenario: Artifact effects field binds manifest ref
- GIVEN an executable artifact installed in the registry
- WHEN it declares effects
- THEN the artifact's canonical effects field points at the effect manifest ref
- AND the manifest itself binds back to the artifact ref.

### Requirement: Unison runtime compatibility is not claimed
r[molten.effects.no_unison_runtime] Molten MUST document Unison abilities/effects as prior art only and MUST NOT claim Unison syntax, type system, runtime, or generalized algebraic effect compatibility.

#### Scenario: Manifest records reference boundary
- GIVEN a Molten effect manifest inspired by Unison-style ability declarations
- WHEN the manifest is rendered
- THEN its checks record that Molten does not implement Unison runtime compatibility.

### Requirement: Handler profiles are explicit
r[molten.effects.handler_profiles] Molten MUST represent admitted handler profiles with canonical `handler-profile-v1` records for production, local, mock, chaos, profiling, and dry-run profiles, binding policy, capability, resource, handler binding, and evidence refs.

#### Scenario: Unsupported profile is rejected
- GIVEN an effect request naming an unsupported handler profile
- WHEN Molten parses the request or profile
- THEN validation fails before any effect handler is invoked.

### Requirement: Effect binding receipts gate requests
r[molten.effects.binding_receipts] Molten MUST emit canonical `effect-binding-receipt-v1` records that bind manifest ref, handler-profile ref, request ref, effect id, operation, decision, diagnostics, evidence refs, and deny-undeclared-effect checks.

#### Scenario: Declared effect receives pass receipt
- GIVEN a request whose artifact, effect id, operation, and handler profile match an admitted manifest and profile
- WHEN Molten admits the request
- THEN it emits a passing effect binding receipt.

#### Scenario: Undeclared effect receives deny receipt
- GIVEN a request for an effect id or operation absent from the artifact manifest
- WHEN Molten admits the request
- THEN it emits a deny receipt with diagnostics
- AND no handler side effect is authorized by the request shape.

### Requirement: Effect request and response envelopes are canonical
r[molten.effects.request_envelope] Effect requests and responses MUST use canonical `effect-request-v1` and `effect-response-v1` envelopes binding artifact refs, effect ids, handler profiles, input/output refs, capability refs, diagnostics, evidence refs, and decision checks.

#### Scenario: Request and response refs are stable
- GIVEN the same artifact ref, effect id, handler profile, input ref, capabilities, and evidence refs
- WHEN Molten renders the effect request and response envelopes
- THEN their canonical refs are stable and replayable.

### Requirement: Undeclared effects deny before side effects
r[molten.effects.deny_undeclared] Molten MUST reject effect requests whose effect id and operation pair is absent from the artifact's admitted effect manifest before exposing Wasmtime hostcalls, Steel APIs, adapter calls, or remote proxy operations.

#### Scenario: Hostcall is absent from manifest
- GIVEN an executable artifact with a manifest declaring only `dataspace.send`
- WHEN it requests `blob.get`
- THEN Molten emits a deny binding receipt before exposing the hostcall or adapter operation.
