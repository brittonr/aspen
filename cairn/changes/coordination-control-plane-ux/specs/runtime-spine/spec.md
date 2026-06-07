# Runtime Spine Delta: Coordination Control-Plane UX

### Requirement: Coordination apply reports are canonical
r[molten.coordination_control_plane_ux.apply_report] Molten MUST emit canonical coordination apply reports that bind the manifest ref, final state ref, receipt refs, assertion refs, and supporting evidence refs.

#### Scenario: Batch apply report binds evidence
- GIVEN coordination request artifacts are applied through the control-plane runtime
- WHEN Molten writes the batch report
- THEN the report records the final coordination state ref
- AND the report binds every coordination receipt and supporting evidence ref.

### Requirement: Coordination show remains read-only
r[molten.coordination_control_plane_ux.readonly_show] Molten MUST summarize coordination artifacts without mutating coordination state or importing authority.

#### Scenario: Operator summarizes generated artifacts
- GIVEN a manifest, request, receipt, token, state snapshot, assertion, or apply report artifact
- WHEN the operator runs `molten test coordination show`
- THEN Molten prints a read-only summary
- AND no control-plane mutation is performed.

### Requirement: Manifest and request CLI emits canonical records
r[molten.coordination_control_plane_ux.manifest_request_cli] Molten MUST provide CLI commands that generate canonical coordination service manifests and requests with explicit operation-id, authority, policy, and resource refs.

#### Scenario: Request generation binds explicit evidence
- GIVEN an operator supplies service, operation, key, client session, operation id, authority refs, policy refs, resource refs, and an optional payload file
- WHEN `molten test coordination request` runs
- THEN it emits a canonical `coordination-request-v1`
- AND the request binds the supplied evidence refs without granting authority by itself.

### Requirement: Batch apply uses the control-plane runtime
r[molten.coordination_control_plane_ux.apply_batch_cli] Molten MUST apply coordination request files through the admitted control-plane runtime and MUST NOT mutate coordination state through ordinary actor messages or direct state edits.

#### Scenario: Queue request commits through apply
- GIVEN a coordination manifest and queue enqueue request artifact
- WHEN `molten test coordination apply` runs
- THEN the request is applied through the coordination control-plane state machine
- AND the output directory contains a coordination receipt and state evidence.

### Requirement: Duplicate operation ids replay without a second mutation
r[molten.coordination_control_plane_ux.idempotent_replay] Molten MUST preserve coordination operation-id idempotency when the CLI applies duplicate request artifacts in a batch.

#### Scenario: Duplicate request returns prior receipt
- GIVEN the same mutating coordination request appears twice in a batch
- WHEN the batch is applied
- THEN the second application returns the prior receipt ref
- AND the final state is not advanced a second time.

### Requirement: Coordination CLI behavior is tested
r[molten.coordination_control_plane_ux.cli_tests] Molten SHOULD cover coordination manifest, request, apply, show, and duplicate replay behavior in automated tests.

#### Scenario: CLI test exercises duplicate replay
- GIVEN the CLI test suite runs
- WHEN it applies the same coordination request twice
- THEN the apply report contains matching receipt refs
- AND the test observes a successful batch decision.

### Requirement: Coordination UX is documented
r[molten.coordination_control_plane_ux.docs] Molten SHOULD document the coordination control-plane UX and state that its receipts are evidence only.

#### Scenario: Operator reads the documentation
- GIVEN an operator reviews the Molten README or architecture notes
- WHEN they inspect coordination control-plane commands
- THEN the docs describe manifest, request, apply, and show usage
- AND the docs clarify that CLI artifacts do not grant authority, policy, resource, transport, or provenance trust.
