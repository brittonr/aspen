# Runtime Spine Delta: Effect Manifest Profiles

## ADDED Requirements

### Requirement: Effect manifests declare possible runtime effects
r[molten.effects.ability_manifest_boundary] Molten MUST require executable artifacts to declare canonical effect manifests that bind effect ids, operations, input/output schema refs, resource classes, capability needs, policy refs, evidence refs, and checks stating that Unison abilities are prior art only.

#### Scenario: Declared effect manifest is admitted for review
- GIVEN an executable artifact declares a manifest for storage read and log emission operations
- WHEN Molten validates the artifact before execution
- THEN the manifest records operation ids, schemas, resources, policy refs, and capability needs
- AND downstream handler admission can evaluate those declarations.

#### Scenario: Missing manifest denies execution
- GIVEN an executable artifact may request host effects but carries no admitted effect manifest
- WHEN Molten evaluates execution admission
- THEN execution denies before adapter startup
- AND diagnostics name the missing effect manifest.

### Requirement: Handler profiles require admission receipts
r[molten.effects.handler_profile_admission] Molten MUST admit concrete handler profiles with receipts that bind supported effect ids, operation schemas, resource bounds, determinism class, replay class, policy refs, capability context, and evidence refs.

#### Scenario: Local deterministic profile admits transcript run
- GIVEN a transcript requests a local deterministic handler profile compatible with an artifact effect manifest
- WHEN profile admission evaluates current policy and capability context
- THEN Molten emits a passing handler-profile admission receipt
- AND execution may use only the admitted handler operations.

#### Scenario: Stale profile receipt denies
- GIVEN a handler-profile admission receipt was produced under older policy or revoked capability context
- WHEN Molten attempts to reuse it for execution
- THEN admission denies or requires recomputation
- AND no side effect is issued from the stale receipt.

### Requirement: Undeclared effects deny before side effects
r[molten.effects.undeclared_effect_denial] Molten MUST deny effect requests that are undeclared, schema-incompatible, profile-incompatible, missing required capability, or unsupported by current resource policy before invoking side-effecting handlers.

#### Scenario: Declared operation executes through admitted handler
- GIVEN an artifact requests an operation listed in its effect manifest and admitted handler profile
- WHEN capability, policy, and resource gates pass
- THEN the handler may execute the operation
- AND the receipt binds the operation and manifest refs.

#### Scenario: Undeclared operation is suppressed
- GIVEN an artifact attempts to request an operation not listed in its effect manifest
- WHEN the effect boundary validates the request
- THEN Molten denies before invoking any handler
- AND records suppression diagnostics.

### Requirement: Handler profile refs bind replay and cache evidence
r[molten.effects.profile_replay_binding] Molten MUST bind exact effect manifest refs and handler profile refs into replay, transcript, evaluation-cache, job DAG, and remote execution evidence.

#### Scenario: Replay uses same profile refs
- GIVEN a recorded run used handler profile H and effect manifest M
- WHEN Molten replays the run
- THEN replay eligibility requires H and M or admitted compatible replacements
- AND the replay receipt records the profile decision.

#### Scenario: Cache hit under different profile denies
- GIVEN an evaluation-cache entry was produced under handler profile H1
- WHEN a caller asks to reuse it under handler profile H2 without compatibility evidence
- THEN Molten denies the cache hit for normative pass evidence.

### Requirement: Effect adaptation validation covers positive and negative paths
r[molten.effects.unison_adaptation_validation] Molten MUST include positive and negative fixtures proving declared effects pass, undeclared effects deny, profile mismatches deny, stale profile receipts deny, and Unison runtime compatibility is not claimed.

#### Scenario: Declared profile fixture passes
- GIVEN an artifact, manifest, handler profile, and capability context are compatible
- WHEN validation runs
- THEN Molten emits passing handler-profile evidence.

#### Scenario: Unison compatibility fixture denies claim
- GIVEN documentation or metadata claims Unison runtime compatibility for Molten effect handling
- WHEN validation checks the manifest boundary
- THEN it denies the claim
- AND records that Unison abilities are prior art only.