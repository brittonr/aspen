## ADDED Requirements

### Requirement: Live reliability cohorts use immutable contracts
r[molten.testing.live_reliability_cohort] Molten MUST bind immutable ChaosControl semantic-history, checker, model, OnixOS live-reliability, OnixOS native-service, Molten build, service profile, generator, deployment, and adapter identities. Mutable sibling paths MUST NOT become evidence-bearing runtime inputs.

#### Scenario: Exact live cohort is admitted
- GIVEN every producer, consumer, schema, model, build, deployment, adapter, and bound identity is complete
- WHEN cohort preflight runs
- THEN it MUST produce one exact cohort ref and bounded execution plan.

#### Scenario: Mutable checkout is the only identity
- GIVEN configuration names only a workspace-relative producer checkout
- WHEN evidence-bearing preflight runs
- THEN it MUST reject the cohort before build, deployment, or network effects.

### Requirement: Live reliability evidence imports fail closed
r[molten.testing.live_reliability_evidence] Molten MUST validate external producer identity, cluster and artifact refs, profile, generator plan and choices, history completeness, operation outcomes, fault stages, recovery observations, checker cohort, verdict, witness, teardown, and non-claims. The importer MUST emit a canonical external-evidence receipt or a typed rejection.

#### Scenario: Complete invalid run imports
- GIVEN a complete live bundle contains a model violation and matching bounded witness
- WHEN offline import runs
- THEN Molten MUST retain the invalid result as external failure evidence without granting authority.

#### Scenario: Selected fault is presented as observed
- GIVEN a bundle records fault selection without matching effect observation
- WHEN import validation runs
- THEN it MUST reject the observed-fault claim or classify the run as incomplete.

### Requirement: Reliability profiles remain separate
r[molten.testing.live_reliability_claim_boundary] Molten MAY share operation corpora and invariant names across simulation, NixOS VM, ChaosControl KVM, and OnixOS live runs. Every profile MUST retain distinct environment, adapter, fault, time, checker, and non-claim facts. No profile MAY substitute for another.

#### Scenario: All profiles return valid
- GIVEN all admitted profiles evaluate matching operation corpora without a reported violation
- WHEN evidence is assembled
- THEN each result MUST retain its own bounded claim and MUST NOT become universal consensus evidence.

#### Scenario: Live result is promoted to release proof
- GIVEN a consumer labels one bounded live result as production readiness or release eligibility
- WHEN claim admission runs
- THEN it MUST reject the promoted role.

### Requirement: Live reliability preserves a functional core
r[molten.testing.live_reliability_boundary] Generator planning, workload projection, outcome mapping, profile and cohort admission, evidence import, cross-profile comparison, and claim classification MUST be pure deterministic logic. Service calls, OnixOS control, files, network, checkers, persistence, and rendering MUST remain shells.

#### Scenario: Import decision runs in memory
- GIVEN parsed live evidence and expected cohort facts
- WHEN evidence admission runs
- THEN it MUST return the same result without files, environment, processes, network, wall clocks, OnixOS, or external tools.

### Requirement: Live reliability covers positive and negative paths
r[molten.testing.live_reliability_validation] The change MUST include positive and negative tests for profiles, cohorts, public clients, histories, outcomes, faults, recovery, checker results, evidence import, profile separation, and claim boundaries.

#### Scenario: Selected live validation rail runs
- GIVEN valid no-fault and recovery fixtures plus stale-read, lost-write, malformed-history, incomplete-recovery, checker-disagreement, teardown-gap, and overclaim fixtures
- WHEN validation runs
- THEN valid fixtures MUST pass and each negative fixture MUST fail with its expected stable class.
