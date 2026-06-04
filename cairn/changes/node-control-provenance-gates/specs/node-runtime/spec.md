# Node Runtime Delta: Control Provenance Gates

### Requirement: Provenance evidence is canonical and explicit
r[molten.node_control_provenance.spec.canonical_evidence] Node control provenance evidence MUST be represented as canonical Preserves artifacts that bind artifact refs, source refs, builder/toolchain refs, review/test/source-gate refs, policy refs, and trust state; node control requests MUST carry explicit evidence refs for side-effecting operations.

#### Scenario: Request binds provenance evidence
- GIVEN a node control install or run request
- WHEN the request is serialized
- THEN it contains an evidence ref sequence
- AND the parser accepts earlier requests as empty evidence for replay compatibility.

### Requirement: Install requires admitted provenance
r[molten.node_control_provenance.spec.install_gate] A node control install MUST evaluate admitted provenance for the payload ref before writing a registry artifact.

#### Scenario: Missing provenance denies install
- GIVEN a running node and an install request with a payload ref but no provenance evidence
- WHEN the request is dispatched directly or by the control loop
- THEN dispatch emits a provenance receipt and a denying control receipt
- AND no node-control artifact is installed.

### Requirement: Run requires admitted job provenance
r[molten.node_control_provenance.spec.run_gate] A node control run MUST evaluate admitted provenance for the job ref inside the execution request before job execution side effects.

#### Scenario: Reviewed job provenance passes run
- GIVEN a running node, a job execution request, an admission receipt, and reviewed provenance for the job ref
- WHEN the run request is dispatched
- THEN the provenance receipt passes
- AND the job execution receipt is emitted as a later subreceipt.

### Requirement: Provenance trust state fails closed
r[molten.node_control_provenance.spec.trust_state] Node control provenance evaluation MUST NOT treat hashes alone as trust and MUST deny missing, malformed, mismatched, sandbox-only-for-node-control, or denied trust states before side effects.

#### Scenario: Tampered provenance denies
- GIVEN a provenance record bound to a different artifact ref
- WHEN a node control install request presents that evidence for another payload
- THEN the provenance gate denies with a diagnostic naming the artifact mismatch
- AND the install side effect is not executed.
