# Remote Artifact Sync Specification

## Purpose

Defines the `remote-artifact-sync` capability.

## Requirements

### Requirement: Remote closure descriptors
r[molten.remote_sync.closure_descriptor] Molten MUST define remote dependency closure descriptors with root artifact id, dependency set or closure hash, artifact kind hints, effect manifest refs, policy/evidence refs, requested handler profile, and replay nonce.

#### Scenario: Descriptor names root and closure
- GIVEN a remote install or execution request
- WHEN Molten builds the closure descriptor
- THEN the descriptor names the root artifact, dependency closure, artifact kinds, effect manifests, policies, handler profile, and replay nonce.

### Requirement: Missing artifact set
r[molten.remote_sync.missing_set] Molten MUST compute the receiver's missing artifact set from a local registry/cache and a closure descriptor before fetching or installing artifacts.

#### Scenario: Present artifact is not fetched
- GIVEN a receiver already has one dependency from a requested closure
- WHEN missing-set calculation runs
- THEN the present artifact is listed as already present and not fetched again.

### Requirement: No mobile closures
r[molten.remote_sync.no_mobile_closures] Molten MUST move admitted artifacts, dependency metadata, arguments, and content refs for remote execution, not arbitrary serialized live heap closures.

#### Scenario: Live heap closure request denies
- GIVEN a remote execution request attempts to carry inline live closure state
- WHEN Molten validates the request
- THEN it denies before install or execution.

### Requirement: Transport-neutral sync descriptors
r[molten.remote_sync.transport_neutral] Molten MUST represent sync descriptors and remote execution requests as canonical envelopes independent of Iroh-specific transport details.

#### Scenario: Same descriptor over two carriers
- GIVEN the same canonical sync descriptor arrives over two admitted carriers
- WHEN Molten validates it
- THEN the missing-set and admission decisions are identical.

### Requirement: Iroh/blob fetch boundary
r[molten.remote_sync.iroh_fetch] Molten MUST fetch missing immutable artifact payloads or chunk manifests through admitted blob/chunk-store fetch boundaries when remote content transport is used; loopback sync MAY use the same canonical ids without a network hop.

#### Scenario: Chunk manifest fetch verifies content
- GIVEN a remote chunk manifest and missing chunks are available through an Iroh/blob root
- WHEN Molten fetches them
- THEN the imported manifest and chunks match their declared content ids before use.

### Requirement: Hash verification before staging
r[molten.remote_sync.hash_verify] Molten MUST verify fetched or loopback-copied artifact bytes against domain-separated canonical artifact ids before staging or installing them.

#### Scenario: Hash mismatch denies install
- GIVEN fetched artifact bytes do not hash to the requested artifact id
- WHEN Molten verifies the artifact
- THEN staging and installation deny before execution.

### Requirement: Install admission
r[molten.remote_sync.install_admission] Molten MUST gate staged dependency closure installation through local policy, provenance, source-gate, capability, resource, and artifact-registry admission before use.

#### Scenario: Local policy denies fetched artifact
- GIVEN a fetched dependency lacks required provenance or policy evidence
- WHEN receiver-side install admission runs
- THEN the dependency is not installed and execution remains denied.

### Requirement: Remote sync cache index
r[molten.remote_sync.cache_index] Molten MUST record verified and admitted remote artifacts in local registry, ledger, or cache indexes with source, evidence, last-use, and pinning metadata sufficient for retention decisions.

#### Scenario: Pinned synced artifact survives GC
- GIVEN a synced artifact is referenced by an active execution, protocol, storage ref, receipt, or metadata pin
- WHEN cache or ledger GC runs
- THEN the artifact remains retained.

### Requirement: Fetch and install receipts
r[molten.remote_sync.fetch_receipts] Molten MUST emit canonical receipts for missing-set calculation, fetch source, hash verification, provenance/source-gate admission, local install admission, and denial.

#### Scenario: Sync loopback emits receipt
- GIVEN a dependency closure sync completes
- WHEN Molten records the result
- THEN the receipt binds installed refs, already-present refs, provenance refs, diagnostics, and checks.

### Requirement: Remote execution envelope
r[molten.remote_sync.execution_envelope] Molten MUST define remote execution envelopes with execution id, root artifact id, entrypoint or stage ids, args/content refs, effect manifest, handler profile, capabilities, resource refs, and evidence refs.

#### Scenario: Execution envelope names authority boundary
- GIVEN a remote job worker request is constructed
- WHEN Molten serializes the request
- THEN it binds artifact/stage ids, target peer, admission refs, execution request refs, authority refs, resource refs, peer bootstrap refs, node identity refs, and evidence refs.

### Requirement: Handler binding before execution
r[molten.remote_sync.handler_binding] Molten MUST bind remote execution to admitted effect handlers or target execution profiles before starting execution.

#### Scenario: Missing handler denies execution
- GIVEN a target peer lacks an admitted handler profile for a requested stage or effect
- WHEN execution admission runs
- THEN the execution request denies before side effects.

### Requirement: Remote result envelope
r[molten.remote_sync.result_envelope] Molten MUST return canonical result or structured failure records with execution receipts, output refs, status refs, trace refs, and diagnostics.

#### Scenario: Worker result binds output refs
- GIVEN a remote or loopback worker completes
- WHEN Molten records the result
- THEN the result envelope binds output refs, stage receipts, resource receipts, delivery evidence, and worker receipt refs.

### Requirement: Loopback executor target
r[molten.remote_sync.loopback_executor] Molten MUST provide a loopback/local executor target for remote sync and remote execution tests before relying on real multi-peer scheduling.

#### Scenario: Loopback execution exercises sync boundary
- GIVEN a source registry, target registry, and worker request
- WHEN loopback sync and execution run
- THEN missing artifacts are copied through verified install admission before the target executes locally.

### Requirement: Incomplete closure rejects execution
r[molten.remote_sync.incomplete_reject] Molten MUST reject execution when dependency closure, policy admission, source-gate evidence, resource evidence, or handler binding is incomplete.

#### Scenario: Missing dependency denies worker
- GIVEN a target registry lacks a required stage artifact after sync
- WHEN worker execution admission runs
- THEN execution denies before producing output.

### Requirement: Replay bounds for remote requests
r[molten.remote_sync.replay_bounds] Molten MUST apply bounded replay, operation-id, session, nonce, or delivery-log checks to remote install and execution requests before treating them as new work.

#### Scenario: Duplicate worker request is bounded
- GIVEN a worker request or scheduled operation id has already been admitted
- WHEN the same operation is observed again
- THEN Molten replays prior evidence or denies duplicate side effects.

### Requirement: Remote sync GC safety
r[molten.remote_sync.gc_safety] Molten MUST track pins from active executions, installed protocols, durable storage refs, receipts, ledger imports, and metadata before evicting synced artifacts.

#### Scenario: Active execution pins dependency
- GIVEN a synced dependency is referenced by an active execution receipt
- WHEN cache eviction is evaluated
- THEN the dependency is retained until the execution and retention policy release it.

### Requirement: Remote sync property tests
r[molten.remote_sync.property_tests] Molten SHOULD include Hegel property tests for closure determinism, missing-set correctness, hash verification, replay bounds, and cache pin invariants within supported bounds.

#### Scenario: Generated missing set is correct
- GIVEN generated source and target closure inventories
- WHEN missing-set calculation runs
- THEN only absent refs are selected for fetch and already-present refs remain untouched.

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
