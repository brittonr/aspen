## ADDED Requirements

### Requirement: Job and Service Effect Manifest [r[job-effect-manifest.manifest]]

Aspen MUST define a versioned effect manifest for executable jobs, services, and execution closures that declares required external effects before execution.

#### Scenario: Manifest declares effects [r[job-effect-manifest.manifest.declares-effects]]

- GIVEN a job, service, or closure may access cluster resources or external effects
- WHEN its manifest is validated
- THEN the manifest MUST declare each required effect kind, resource scope when applicable, and whether the effect is read-only or mutating when that distinction applies

#### Scenario: Unknown effect rejected or version-gated [r[job-effect-manifest.manifest.unknown-effect]]

- GIVEN a manifest contains an unknown effect kind for its schema version
- WHEN admission validates it
- THEN Aspen MUST reject the manifest or require an explicit version-gated compatibility path

### Requirement: Effect Admission Maps to Capabilities [r[job-effect-manifest.admission]]

Aspen MUST map requested effects to granted capabilities or policy handles before execution starts.

#### Scenario: Declared and granted effects admitted [r[job-effect-manifest.admission.declared-granted]]

- GIVEN a manifest requests effects that are all granted by caller, job, service, or runtime policy
- WHEN admission runs
- THEN Aspen MAY admit execution and MUST record a redacted grant summary in the receipt

#### Scenario: Declared but ungranted effect denied [r[job-effect-manifest.admission.ungranted-denied]]

- GIVEN a manifest requests an effect without a matching grant
- WHEN admission runs
- THEN Aspen MUST reject execution before runtime start
- AND the failure receipt MUST identify the denied effect kind without exposing raw capability material

### Requirement: Undeclared Effect Use Is Rejected [r[job-effect-manifest.undeclared-use]]

Aspen MUST reject or block attempts to use effects that were not declared in the admitted manifest for the selected executor slice.

#### Scenario: Executor blocks undeclared storage write [r[job-effect-manifest.undeclared-use.storage-write]]

- GIVEN an admitted manifest does not declare KV or storage write access
- WHEN the selected executor slice attempts a storage write through Aspen-controlled APIs
- THEN Aspen MUST reject the operation and record a bounded undeclared-effect diagnostic

#### Scenario: Executor blocks undeclared network access when enforceable [r[job-effect-manifest.undeclared-use.network]]

- GIVEN an admitted manifest does not declare outbound network access and the selected runtime can enforce that boundary
- WHEN execution attempts outbound network access
- THEN the runtime policy MUST block or fail the operation and record a bounded diagnostic

### Requirement: Effect Manifest Drives Sandbox Policy [r[job-effect-manifest.sandbox-policy]]

Aspen MUST derive selected worker/runtime sandbox policy from the admitted effect manifest where the runtime supports enforcement.

#### Scenario: Secret access withheld [r[job-effect-manifest.sandbox-policy.secret-withheld]]

- GIVEN a manifest does not declare secret/config access
- WHEN the worker prepares the runtime environment
- THEN it MUST NOT inject secret handles, config values, or environment variables for secret access

#### Scenario: Blob read-only effect limits mutation [r[job-effect-manifest.sandbox-policy.blob-readonly]]

- GIVEN a manifest declares blob read but not blob write
- WHEN runtime policy is prepared
- THEN the worker MUST expose only read capability handles for blob access where the selected executor supports handle separation

### Requirement: Effect-Aware Receipts and Redaction [r[job-effect-manifest.receipts-redaction]]

Aspen MUST use the effect manifest to produce bounded and secret-safe admission and execution receipts.

#### Scenario: Receipt summarizes effects [r[job-effect-manifest.receipts-redaction.summarizes-effects]]

- GIVEN a job or service is admitted with an effect manifest
- WHEN the receipt is emitted
- THEN it MUST include manifest schema version, declared effect kinds, granted/denied effect summary, and redacted capability handles

#### Scenario: Receipt redacts secret-bearing effects [r[job-effect-manifest.receipts-redaction.secret-redaction]]

- GIVEN the manifest includes secret, config, ticket, token, cookie, or key-bearing effects
- WHEN receipts, logs, or diagnostics are rendered
- THEN Aspen MUST NOT print raw secret material and MUST include only opaque handles, hashes, or redacted summaries
