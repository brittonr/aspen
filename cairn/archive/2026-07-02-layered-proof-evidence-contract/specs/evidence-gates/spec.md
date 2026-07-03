## ADDED Requirements

### Requirement: Layered proof contract
r[molten.evidence.layered_proof.contract] Molten SHOULD define a layered proof evidence contract that distinguishes pure-core proof, gate proof, replay proof, release proof, and operator readback evidence.

#### Scenario: Layered proof lists roles
- GIVEN a proof evidence bundle
- WHEN its proof layers are summarized
- THEN each layer is identified by role, subject ref, decision, child refs, and evidence-only caveats.

### Requirement: Pure-core proof evidence remains deterministic
r[molten.evidence.layered_proof.pure_core_receipts] Pure-core proof evidence MUST be derived from deterministic explicit inputs and MUST NOT depend on filesystem, network, process, clock, random, database, rendered logs, or ambient environment state.

#### Scenario: Pure-core proof rerenders same ref
- GIVEN the same explicit proof inputs
- WHEN pure-core proof rendering runs twice
- THEN both runs produce the same canonical ref.

### Requirement: Gate proof binds core evidence
r[molten.evidence.layered_proof.gate_receipts] Gate proof evidence SHOULD bind relevant pure-core evidence refs and MUST fail closed when required core evidence is stale, missing, denied, or scoped to the wrong subject.

#### Scenario: Stale core proof denies gate layer
- GIVEN a gate proof receipt naming a core proof ref for another subject
- WHEN layered validation runs
- THEN the gate layer is denied for wrong subject scope.

### Requirement: Replay proof binds gate and core refs
r[molten.evidence.layered_proof.replay_receipts] Replay proof evidence SHOULD bind canonical gate and core refs and compare deterministic artifacts by refs and declared variance rather than rendered logs.

#### Scenario: Replay binds gate receipt
- GIVEN a replay verification receipt
- WHEN layered validation inspects it
- THEN it names the gate receipt or core proof refs that were replayed.

### Requirement: Release proof binds lower layers
r[molten.evidence.layered_proof.release_receipts] Release proof evidence MUST bind the lower-layer proof refs required by release policy and MUST deny release promotion when a required lower layer is missing, stale, denied, or diagnostic-only.

#### Scenario: Missing replay layer denies release proof
- GIVEN a release proof requiring replay evidence
- WHEN the replay receipt ref is absent
- THEN release proof validation emits deny evidence.

### Requirement: Operator readbacks are non-normative
r[molten.evidence.layered_proof.operator_readbacks] Operator readbacks MUST remain rendered views over canonical receipts and MUST NOT override canonical decisions or serve as pass evidence unless a separate gate explicitly admits the underlying canonical refs.

#### Scenario: Readback cannot promote deny to pass
- GIVEN a readback that includes a denied gate receipt
- WHEN pass evidence is evaluated
- THEN the denied canonical receipt controls the decision.

### Requirement: Cross-layer boundary validation
r[molten.evidence.layered_proof.cross_layer_boundary] Layered proof validation MUST deny stale child refs, cyclic layer graphs, wrong-subject links, unsupported layer roles, and diagnostic/readback layers used as pass evidence.

#### Scenario: Cyclic layer graph denies
- GIVEN proof layers that reference each other cyclically
- WHEN layered validation runs
- THEN validation denies the graph with a cycle diagnostic.

### Requirement: Layered proof Hegel properties
r[molten.evidence.layered_proof.hegel_properties] Layered proof validation SHOULD include Hegel RS property tests for stable layer ordering, stale child denial, cycle denial, wrong-scope denial, diagnostic non-pass, and aggregate ref stability.

#### Scenario: Generated wrong-scope layer denies
- GIVEN Hegel RS generates a layer graph with a child subject mismatch
- WHEN layered validation runs
- THEN the graph cannot produce pass evidence.

### Requirement: Layered proof documentation
r[molten.evidence.layered_proof.docs] Documentation SHOULD explain the layered proof model, how each layer binds lower refs, and why evidence layers do not grant subsystem authority.

#### Scenario: Reviewer follows layer docs
- GIVEN a layered proof bundle
- WHEN a reviewer follows the documentation
- THEN they can trace from release evidence to replay, gate, and core refs without treating summaries as authority.
