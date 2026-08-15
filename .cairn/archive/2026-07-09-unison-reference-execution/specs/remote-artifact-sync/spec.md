# Remote Artifact Sync Delta: Reference Execution

## ADDED Requirements

### Requirement: Remote execution requests use exact artifact refs
r[molten.remote_execution.ref_request_envelope] Molten MUST define canonical remote execution request envelopes that bind root artifact ref, dependency closure descriptor, entrypoint id, canonical argument value or content ref, effect manifest ref, requested handler profile, presented capabilities, policy refs, provenance refs, source-gate refs, resource refs, and reply route.

#### Scenario: Ref-backed request is well formed
- GIVEN a caller requests remote execution of artifact ref A with closure descriptor C and canonical argument ref I
- WHEN Molten validates the request envelope
- THEN the request binds A, C, I, effect manifest, handler profile, capability, policy, provenance, source-gate, resource, and reply-route evidence.

#### Scenario: Name-only request denies
- GIVEN a remote execution request names an executable by mutable name without an exact artifact ref or admitted resolution receipt
- WHEN the receiver validates the request
- THEN it denies before dependency fetch or execution
- AND diagnostics require exact ref identity.

### Requirement: Receiver-driven closure admission precedes execution
r[molten.remote_execution.receiver_closure_admission] Molten MUST let the receiver compute missing dependencies, fetch selected refs, verify fetched bytes, check closure completeness, and apply local install/admission gates before remote execution.

#### Scenario: Verified closure executes locally
- GIVEN the receiver computes missing refs, fetches them, verifies BLAKE3 content refs, and passes local admission gates
- WHEN execution starts
- THEN the execution receipt binds closure completeness and install/admission evidence.

#### Scenario: Sender-pushed extra ref denies import
- GIVEN a sender includes a dependency ref not selected by the receiver's closure descriptor or missing-set plan
- WHEN the receiver evaluates the response
- THEN Molten denies or ignores the extra ref before import
- AND records diagnostics naming the unrequested ref.

### Requirement: Execution admission binds handlers and capabilities
r[molten.remote_execution.handler_profile_capability_binding] Molten MUST bind presented capabilities, effect manifests, handler profiles, resource policy, provenance, source-gate evidence, and local policy decisions into remote execution admission and result receipts.

#### Scenario: Handler profile and capabilities pass
- GIVEN a request presents attenuated capabilities and a handler profile compatible with the artifact effect manifest
- WHEN local policy, provenance, source-gate, and resource gates pass
- THEN Molten may execute the artifact
- AND the result receipt binds the exact admission refs.

#### Scenario: Missing capability denies before adapter startup
- GIVEN a request needs a capability not presented or not admitted locally
- WHEN execution admission evaluates the request
- THEN it denies before starting the execution adapter
- AND no side effect is issued.

### Requirement: Mobile closure payloads are rejected
r[molten.remote_execution.no_mobile_closure_boundary] Molten MUST reject arbitrary live closures, heap captures, file descriptors, ambient environment, process state, or unbounded serialized runtime state as executable authority in remote execution requests.

#### Scenario: Canonical argument value is allowed
- GIVEN a request carries a canonical Preserves argument value or content ref
- WHEN the request passes schema and policy admission
- THEN Molten may pass that argument to the admitted artifact.

#### Scenario: Live heap capture denies
- GIVEN a request embeds a serialized live closure or heap snapshot as the executable payload
- WHEN Molten validates the request
- THEN it denies before install, adapter startup, or side effects
- AND reports that remote execution is artifact-ref based only.

### Requirement: Reference execution validation covers positive and negative paths
r[molten.remote_execution.validation] Molten MUST include positive and negative fixtures for verified closure execution, missing dependencies, wrong hashes, sender-pushed extras, mobile closure payloads, missing capabilities, handler profile mismatch, and local policy denial.

#### Scenario: Verified remote execution fixture passes
- GIVEN a fixture with exact artifact refs, complete dependency closure, admitted handlers, and policy evidence
- WHEN validation runs
- THEN Molten emits a passing execution admission and result receipt.

#### Scenario: Incomplete closure fixture denies
- GIVEN a fixture omits a required dependency from the closure
- WHEN validation runs
- THEN execution admission denies before adapter startup
- AND diagnostics identify the missing dependency ref.