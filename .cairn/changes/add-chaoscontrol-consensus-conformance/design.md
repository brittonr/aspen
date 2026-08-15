## Context

The accepted consensus spec already requires canonical command envelopes, deterministic apply, idempotent client sessions, snapshots, recovery, observability, integration tests, and Hegel properties.

The active `fabric-whole-system-simulation` change defines deterministic simulation and a claim ladder. It does not provide external KVM evidence for the production-shaped consensus executable.

ChaosControl owns deterministic VM execution, fault schedules, replay, and the generic SMR chain checker. Molten owns the state machine, observer path, packaging, authority, policy, and evidence import.

## Decisions

### 1. Exercise Molten code, not a harness model

**Choice:** The guest profile runs the same admitted Molten consensus engine, application state-machine core, command schemas, durable adapter contracts, and observation projection used by the selected live profile.

A fixture-only consensus implementation can provide negative harness tests. It cannot satisfy Molten conformance.

**Rationale:** Passing a separate model would prove only that model.

### 2. Project a semantic chain from committed application transitions

**Choice:** The cohort binds a canonical initial-state ref. After each committed apply, Molten projects group ref, replica ref, command index, operation ref, command ref, prior chain digest, next chain digest, application-state ref, and lifecycle generation.

Accepted conformance uses lossless observation mode with bounded dropped-event accounting. An observation gap blocks conformance as an observer failure. It does not become a consensus-safety failure.

The chain digest uses the exact versioned ChaosControl transition contract. It supplements Molten state refs and receipts. It does not replace them.

**Rationale:** The generic checker needs implementation-neutral observations while Molten retains its canonical state and evidence model.

### 3. Preserve operation identity through uncertain results

**Choice:** A logical command keeps its Molten client-session and sequence identity across retries. The adapter maps outcomes to acknowledged, definitely rejected, or indefinite.

Timeout, disconnect, or process loss cannot become definite non-execution evidence. Recovery observations resolve whether an operation joined the committed history.

**Rationale:** Distributed clients cannot infer non-execution from every error.

### 4. Consume an immutable producer contract

**Choice:** Molten pins the ChaosControl package revision, workload schema ref, hash profile ref, and accepted evidence classes. Rust validates all imported data.

Nix can materialize the exact guest and harness closure. Implementation must update flake inputs through Nix commands, never by editing `flake.lock`.

**Rationale:** A sibling checkout path is not a durable product contract.

### 5. Start with a bounded crash-fault matrix

**Choice:** The initial matrix includes a no-fault control, message loss and reordering, temporary partition, majority loss, leader crash, follower crash, restart, and snapshot catch-up.

Disk corruption and clock anomalies remain unsupported until the selected Molten adapter profile defines their exact semantics. Byzantine faults remain out of scope.

**Rationale:** The matrix must not claim faults that the consumer cannot model or observe honestly.

### 6. Evaluate safety continuously and liveness conditionally

**Choice:** Chain and canonical application-state equality at each command index are continuous safety properties. Idempotent operation application and monotonic committed state are additional Molten invariants.

Liveness starts only after a declared stable quorum, inactive disruptive faults, admitted lifecycle state, and bounded virtual progress horizon.

**Rationale:** Quorum loss can stop progress without permitting divergent committed state.

### 7. Import external evidence without transferring authority

**Choice:** A pure admission core validates producer identity, schema, workload profile, Molten artifact refs, observer identity, fault outcomes, bounds, observation summary, verdicts, replay class, and non-claims.

The shell reads the external bundle and stores a canonical Molten import receipt. Admission requires the declared observation mode and complete dropped-event accounting.

Imported evidence cannot grant policy, authority, resources, provenance, retention, deployment, or release eligibility.

**Rationale:** External execution evidence is useful only when identity and role remain explicit.

### 8. Keep simulation and VM evidence separate

**Choice:** Whole-system simulation and ChaosControl profiles can share operation corpora, invariant identifiers, and expected failure classes. Their receipts retain different evidence-profile labels.

A matching simulation result cannot substitute for VM evidence. A matching VM result cannot prove simulator conformance.

**Rationale:** Shared semantics improve comparison without collapsing environmental claims.

### 9. Preserve the functional core and shell boundary

**Choice:** Chain projection inputs, operation-outcome mapping, evidence admission, claim classification, and cross-profile comparison remain pure functions.

Consensus execution, guest startup, Nix materialization, filesystem access, KVM control, bundle import, and receipt persistence remain shells.

**Rationale:** Molten can test policy and semantic decisions without external infrastructure.

## Dependencies and blocker

Implementation is blocked until ChaosControl archives a versioned SMR chain-workload contract and accepted producer evidence classes. Foundation artifacts and local negative fixtures can proceed before that handoff.

The integration also requires a production-shaped multi-node Molten consensus guest profile. If that profile cannot use admitted cross-process transport, the KVM tasks remain blocked rather than substituting an in-process model.

## Risks / Trade-offs

- Observer defects can hide application defects. Observer-path identity and direct state-ref comparison remain required.
- External KVM campaigns can be expensive. Cheap pure and packaging rails remain separate from behavior campaigns.
- Equal replica chains do not prove command authorization or application-level validity. Molten gates remain mandatory.
- Shared workload names can imply evidence parity. Every receipt keeps an explicit profile and non-claim set.
