# Persistent Node Identity Delta: Iroh Endpoint Identity Persistence

### Requirement: Iroh endpoint secret backends are explicit
r[molten.node_identity.iroh_secret_backend_model] Molten MUST model Iroh endpoint identity sources as explicit key metadata, managed secret backend metadata, persisted node-state file metadata, generate-and-persist policy, or fail-closed denial policy, and MUST bind the selected source class into startup identity receipts without exposing private key material.

#### Scenario: Startup selects persisted endpoint key
- GIVEN a node state root contains an admitted persisted Iroh endpoint secret and matching public identity metadata
- WHEN startup identity resolution runs
- THEN the decision selects the persisted-file source class and emits a receipt that binds only redacted source metadata and the endpoint public identity ref.

#### Scenario: Missing required backend denies
- GIVEN profile policy requires a managed secret backend and no usable backend metadata is available
- WHEN identity resolution runs
- THEN startup denies before generating a replacement key or advertising an endpoint.

### Requirement: Iroh identity resolution is pure before secret IO effects
r[molten.node_identity.iroh_identity_resolution_core] Molten MUST decide Iroh identity source selection with a pure core over explicit metadata facts, expected node identity refs, policy refs, and platform capability facts before shell code generates keys, persists files, rotates identity, or accepts drift.

#### Scenario: First boot generation is policy admitted
- GIVEN no explicit key, backend key, or persisted file exists and profile policy permits first-boot generation
- WHEN the pure resolution core evaluates the metadata facts
- THEN it returns a generate-and-persist decision with receipt inputs and required permission expectations.

#### Scenario: Malformed key metadata denies
- GIVEN explicit or persisted key metadata is malformed or inconsistent with the expected endpoint public identity
- WHEN the pure resolution core evaluates it
- THEN the decision denies with diagnostics before the live endpoint is started.

### Requirement: Persisted Iroh secrets use restricted permissions
r[molten.node_identity.iroh_identity_permissions] Shell-owned Iroh endpoint secret persistence MUST use owner-only permissions or the strongest supported platform restriction, and MUST emit deterministic diagnostics when requested restrictions cannot be verified or applied.

#### Scenario: Owner-only file is accepted
- GIVEN a generated Iroh endpoint secret is persisted in the node state root with owner-only access on a platform that supports it
- WHEN startup verifies the persisted source metadata
- THEN the identity receipt records a restricted-permission pass without revealing the secret bytes.

#### Scenario: Unsafe permissions deny or require policy
- GIVEN a persisted Iroh endpoint secret is readable outside the owner boundary
- WHEN startup identity resolution evaluates it
- THEN startup denies or requires explicit recovery policy before using the secret.

### Requirement: Endpoint identity drift requires admitted recovery
r[molten.node_identity.iroh_identity_drift_recovery] Molten MUST detect when an existing node identity scope observes a different Iroh endpoint public key than the prior admitted identity record, and MUST deny startup or require admitted rotation/recovery evidence before accepting the changed endpoint identity.

#### Scenario: Restart preserves endpoint identity
- GIVEN a node has an admitted identity record and persisted endpoint secret
- WHEN the node restarts with the same state root and no rotation policy
- THEN startup observes the same endpoint public identity and emits a load receipt bound to the prior identity ref.

#### Scenario: Unadmitted drift denies startup
- GIVEN a node identity record binds one endpoint public key and startup observes a different endpoint public key for the same node scope
- WHEN no admitted rotation or recovery receipt is supplied
- THEN startup denies before advertising the new endpoint.

### Requirement: Iroh identity receipts redact secrets
r[molten.node_identity.iroh_identity_receipt_redaction] Startup, replay, peer-bootstrap, drift, denial, and rotation receipts that mention Iroh identity MUST bind endpoint public identity refs, source class, policy refs, permission diagnostics, prior/new identity refs where applicable, and redacted source metadata, and MUST NOT include private key bytes, bearer tokens, raw ticket material, or sensitive local path contents.

#### Scenario: Receipt exposes public identity only
- GIVEN startup loads a valid endpoint secret
- WHEN the startup identity receipt is emitted
- THEN the receipt includes the endpoint public identity ref and source class
- AND the receipt excludes private key bytes and raw credential material.

#### Scenario: Replay does not require private key
- GIVEN replay validates startup identity evidence
- WHEN replay inspects the identity receipt
- THEN replay uses public identity refs and receipt bindings without requiring access to the private endpoint key.