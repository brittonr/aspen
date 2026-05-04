## ADDED Requirements

### Requirement: Full Aspen hardening audit inventory [r[aspen-hardening-audit.inventory]]

Aspen MUST maintain a reproducible hardening audit inventory that enumerates every security authority boundary in scope for the audit.

#### Scenario: Inventory covers request and handler surfaces [r[aspen-hardening-audit.inventory.request-handler-surfaces]]

- GIVEN the audit inventory is generated or reviewed
- WHEN request and handler surfaces are inspected
- THEN it MUST include `ClientRpcRequest`, `CiRequest`, authenticated request decoding, direct handler dispatch, native/proxy handlers, blob paths, and feature-gated handlers
- AND each row MUST identify whether the surface is public, capability-protected, admin-protected, domain-capability-protected, or fail-closed

#### Scenario: Inventory covers persisted authority surfaces [r[aspen-hardening-audit.inventory.persisted-authority-surfaces]]

- GIVEN the audit inventory is generated or reviewed
- WHEN persisted authority surfaces are inspected
- THEN it MUST include token files, cluster tickets, bootstrap/root tokens, federation credentials, reserved KV prefixes, SNIX store/cache keys, secrets/trust material, dogfood receipts, CI artifacts, and logs
- AND each row MUST identify redaction and permission expectations

#### Scenario: Inventory records feature coverage [r[aspen-hardening-audit.inventory.feature-coverage]]

- GIVEN a boundary is feature-gated
- WHEN the audit records its evidence
- THEN the inventory MUST record the exact feature set and command used to exercise that boundary

### Requirement: Authorization matrix and fail-closed classification [r[aspen-hardening-audit.authorization-matrix]]

Aspen MUST provide an audit authorization matrix that maps protected operations to their expected `Operation` and accepted capability shape.

#### Scenario: Non-public requests classify to operations [r[aspen-hardening-audit.authorization-matrix.non-public-classification]]

- GIVEN a non-public client or CI request variant
- WHEN request classification is audited
- THEN the request MUST map to an explicit `Operation` or a fail-closed administrative operation before handler dispatch
- AND intentional public exemptions MUST be listed with rationale

#### Scenario: Domain authority is not granted by generic prefixes [r[aspen-hardening-audit.authorization-matrix.domain-specific-authority]]

- GIVEN a domain-specific boundary such as SNIX, federation, deploy, jobs/CI, Forge, secrets/trust, or cluster admin
- WHEN generic KV or Forge capabilities are presented
- THEN the audit MUST prove those generic capabilities do not authorize the domain-specific operation unless the matrix explicitly defines that equivalence

#### Scenario: Batch and conditional operations include all protected keys [r[aspen-hardening-audit.authorization-matrix.batch-condition-keys]]

- GIVEN a batch or conditional operation contains protected read, write, condition, scan, or watch keys
- WHEN authorization is computed
- THEN the resulting operation MUST include every protected key whose state can be disclosed or mutated

### Requirement: Authenticated dispatch and proxy boundary evidence [r[aspen-hardening-audit.dispatch-proxy-evidence]]

Aspen MUST prove authenticated request handling runs the intended auth gate before protected handler logic and preserves proxy security semantics.

#### Scenario: Auth runs before protected handler dispatch [r[aspen-hardening-audit.dispatch-proxy-evidence.auth-before-dispatch]]

- GIVEN a protected request reaches the RPC dispatch path
- WHEN source-order or runtime evidence is inspected
- THEN authentication and presenter binding MUST occur before the protected handler executes

#### Scenario: Proxy eligibility fails closed [r[aspen-hardening-audit.dispatch-proxy-evidence.proxy-eligibility]]

- GIVEN a request is eligible for cross-cluster proxy discovery or forwarding
- WHEN the presented token is key-bound, generic bearer, expired, too broad, missing the federation-proxy proof, or otherwise outside proxy policy
- THEN the proxy path MUST reject it before forwarding credential material

#### Scenario: Federation scopes remain pull/push-specific [r[aspen-hardening-audit.dispatch-proxy-evidence.federation-scopes]]

- GIVEN a federation read-shaped or write-shaped request
- WHEN authorization is audited
- THEN read-shaped requests MUST require `FederationPull`
- AND write-shaped requests MUST require `FederationPush`
- AND generic Forge/KV scopes MUST NOT authorize those operations

### Requirement: Credential redaction and safe evidence [r[aspen-hardening-audit.credential-redaction]]

Aspen MUST prove that audit evidence, operator outputs, logs, errors, fixtures, and receipts do not expose credential material.

#### Scenario: Synthetic secret fixtures are redacted [r[aspen-hardening-audit.credential-redaction.synthetic-fixtures]]

- GIVEN synthetic token, ticket, bearer, private-key, API-key, password, and connection-string fixtures
- WHEN representative CLI output, logs, errors, receipts, and debug formatting are produced
- THEN secret-like values MUST be absent or replaced with redaction markers
- AND evidence MUST record paths, lengths, hashes, or stable identifiers instead of credential values

#### Scenario: Real secret material is not read for audit evidence [r[aspen-hardening-audit.credential-redaction.no-real-secret-read]]

- GIVEN an audit command collects evidence
- WHEN it encounters a path that may contain a real cluster ticket, token, private key, password, or connection string
- THEN the command MUST NOT read or print the secret value
- AND it MAY record existence, owner-only permission metadata, size, or a redacted placeholder

#### Scenario: Receipt artifacts do not require log scraping [r[aspen-hardening-audit.credential-redaction.receipt-artifacts]]

- GIVEN a dogfood, CI, deploy, or diagnostic receipt
- WHEN an operator uses it for follow-up triage
- THEN the receipt MUST include run/artifact identifiers needed for follow-up without requiring credential-bearing log scraping

### Requirement: Execution, network, and supply-chain hardening evidence [r[aspen-hardening-audit.execution-network-supply-chain]]

Aspen MUST include execution sandbox, network/transport, and build supply-chain boundaries in the hardening audit.

#### Scenario: Execution boundaries are audited [r[aspen-hardening-audit.execution-network-supply-chain.execution-boundaries]]

- GIVEN jobs, CI, native build, SNIX build/eval, plugin, VM, shell worker, or fallback subprocess paths exist
- WHEN execution boundaries are audited
- THEN the evidence MUST identify command allowlists, sandbox/isolation mode, filesystem/network assumptions, artifact identity, and failure reporting behavior

#### Scenario: Network boundaries stay Iroh-first [r[aspen-hardening-audit.execution-network-supply-chain.network-boundaries]]

- GIVEN client, node-to-node, federation, blob, snapshot, metrics, or cache-gateway traffic is audited
- WHEN transport boundaries are reviewed
- THEN protected control-plane operations MUST remain on the intended Iroh/Raft/auth path
- AND any HTTP/cache gateway exposure MUST be documented as data-plane or compatibility scope with explicit auth and redaction expectations

#### Scenario: Supply-chain inputs are traceable [r[aspen-hardening-audit.execution-network-supply-chain.supply-chain-inputs]]

- GIVEN Nix flakes, vendored dependencies, build scripts, code generators, or external tool inputs affect Aspen builds
- WHEN supply-chain evidence is audited
- THEN the audit MUST record pinned inputs, update paths, verification commands, and any unsafe/public-unsafe API surfaces that require review

### Requirement: Findings become verified remediation slices [r[aspen-hardening-audit.remediation]]

Aspen MUST turn hardening audit findings into prioritized, reviewable remediation work with evidence-backed acceptance.

#### Scenario: Finding report is actionable [r[aspen-hardening-audit.remediation.actionable-report]]

- GIVEN the audit identifies a finding
- WHEN the finding is entered into the report
- THEN it MUST include severity, affected boundary, source handle, evidence handle, expected control, observed gap, remediation owner or follow-up change, and verification plan

#### Scenario: High-risk findings get focused follow-up [r[aspen-hardening-audit.remediation.high-risk-follow-up]]

- GIVEN a finding can bypass authorization, expose credential material, execute outside the expected sandbox, or corrupt trust evidence
- WHEN remediation is planned
- THEN it MUST be split into a focused OpenSpec child change or direct verified commit before the umbrella audit is archived

#### Scenario: Audit acceptance gate is explicit [r[aspen-hardening-audit.remediation.acceptance-gate]]

- GIVEN the full hardening audit is ready for closeout
- WHEN acceptance is evaluated
- THEN the change MUST have strict OpenSpec validation, `git diff --check`, audit inventory, threat model, finding report, negative evidence for high-risk boundaries, and recorded verification commands
