# Fabric Membership Placement Specification

## Purpose

Defines the `fabric-membership-placement` capability.

## Requirements

### Requirement: Membership views are canonical and source-scoped
r[molten.fabric_membership.membership_views] Aspen MUST represent a membership view as a canonical ordered set of member identities and node-descriptor refs bound to a view id or epoch, source profile, freshness, authority evidence, eligibility policy, and non-claims. Static, policy-managed, consistency-backed, and deterministic-simulation sources MAY be provided as distinct profiles. A view MUST NOT claim stronger agreement or freshness than its source profile establishes.

#### Scenario: Admitted view is consumed
- GIVEN a membership provider emits a canonical view with valid source evidence and compatible node descriptors
- WHEN an extension requests eligible members
- THEN the port returns the ordered view and its scope without promoting connectivity observations into membership authority.

#### Scenario: Stale view fails a fresh-view requirement
- GIVEN an extension requires a view fresher than the provider's recorded boundary
- WHEN placement admission evaluates the view
- THEN it denies or selects an explicit degraded policy
- AND does not label the stale view current.

### Requirement: Locality and fault domains are typed
r[molten.fabric_membership.locality] Aspen MUST represent locality, failure domains, hardware classes, runtime features, capacity, and policy labels through typed, schema-versioned node descriptors. Placement constraints MUST identify whether labels are authoritative, operator-declared, measured, or advisory. Unknown required labels or incompatible schemas MUST make the constraint unsatisfied.

#### Scenario: Anti-affinity spans declared fault domains
- GIVEN eligible nodes have authoritative zone labels and a role requires replicas in distinct zones
- WHEN placement runs
- THEN selected nodes use distinct declared zone values or placement returns an unsatisfied constraint.

#### Scenario: Advisory label cannot satisfy authority requirement
- GIVEN a node reports an unverified locality hint
- WHEN a hard placement constraint requires an authoritative locality label
- THEN that node is not eligible for the constraint.

### Requirement: Failure detectors produce observations, not membership authority
r[molten.fabric_membership.failure_detector] Aspen MUST expose bounded failure-detector observations with subject, detector profile, observation class, time basis, freshness, confidence or threshold state, supporting event refs, and non-claims. A detector observation MUST NOT directly add or remove membership, revoke capability, transfer ownership, or commit a replacement assignment.

#### Scenario: Suspicion informs placement policy
- GIVEN a detector reports a member suspected under its declared profile
- WHEN placement policy evaluates a new assignment
- THEN it may avoid the member according to explicit policy while preserving the source observation.

#### Scenario: Partition does not become proven failure
- GIVEN peers are mutually unreachable during a simulated partition
- WHEN failure detection runs
- THEN it may report suspicion or unavailability
- AND it does not claim that either process stopped or lost authority.

### Requirement: Placement planning is deterministic and explainable
r[molten.fabric_membership.placement] Aspen MUST implement placement as a pure deterministic function over a declared membership view, role requirements, current assignments, resource inventory, typed locality and anti-affinity constraints, policy decisions, failure observations, and deterministic tie-break input. It MUST return either a canonical plan with reasons and residual capacity or structured unsatisfied constraints without side effects. Because replicas share one requirement, the search MUST consider each candidate set once rather than every ordering of it, and a request that cannot have enough distinct anti-affinity values MUST be reported unsatisfied before searching.

#### Scenario: Identical inputs produce identical plan
- GIVEN identical canonical planner inputs and tie-break stream
- WHEN placement runs repeatedly
- THEN it returns the same assignments, scores, reasons, and residual-capacity model.

#### Scenario: Impossible placement is explicit
- GIVEN a role requires more distinct eligible fault domains than the view provides
- WHEN placement runs
- THEN it returns an unsatisfied hard constraint
- AND does not silently colocate roles or weaken anti-affinity.

#### Scenario: Too few anti-affinity values are reported before searching
- GIVEN a modest view whose candidates, after excluding values held by active assignments of the same service and role, have fewer distinct values for an anti-affinity key than requested replicas
- WHEN placement runs
- THEN it returns unsatisfied anti-affinity constraints rather than exhausting the search bound.

#### Scenario: Search over candidate sets preserves plans
- GIVEN any request that the previous ordering search answered within its step bound
- WHEN placement runs
- THEN it returns the identical plan or unsatisfied outcome.

#### Scenario: Jointly infeasible constraints still respect the search bound
- GIVEN anti-affinity keys that each have enough distinct values but cannot all be satisfied together across a large view
- WHEN placement runs
- THEN it reports the search limit rather than searching without bound.

### Requirement: Role recruitment has an explicit lifecycle
r[molten.fabric_membership.recruitment] Aspen MUST host canonical propose, reserve, assign, acknowledge, activate, drain, replace, release, fail, and quarantine transitions for extension-owned role kinds. Each transition MUST bind extension and service identity, role identity, node identity, service generation, assignment epoch, resource reservation, placement-plan ref, authority profile, and current lifecycle state.

#### Scenario: Role activates after acknowledgement
- GIVEN a valid placement plan, resource reservation, current assignment epoch, and admitted target node
- WHEN the node acknowledges successful role initialization
- THEN the assignment may enter active state and emit canonical activation evidence.

#### Scenario: Duplicate delayed acknowledgement is rejected
- GIVEN an assignment has been replaced or released
- WHEN its delayed acknowledgement arrives
- THEN transition validation rejects it as stale
- AND it cannot reactivate the old assignment.

### Requirement: Assignment fencing is profile-scoped
r[molten.fabric_membership.fencing] Aspen MUST bind assignments and effectful role operations to assignment epochs and fencing tokens issued by an admitted authority profile. Consumers MUST validate fencing at declared mutation or ownership boundaries. Every profile MUST state whether enforcement is process-local, node-local durable, quorum-ordered, or external; weaker profiles MUST NOT claim stronger distributed exclusion.

#### Scenario: Current token permits fenced mutation
- GIVEN a role operation presents the current token for its assignment and the target effect port enforces that authority profile
- WHEN fencing validation runs
- THEN the operation may proceed subject to its other capabilities and preconditions.

#### Scenario: Stale token denies
- GIVEN a replacement assignment has advanced the enforced epoch
- WHEN the old role submits a fenced mutation
- THEN the mutation denies before the effect
- AND records the stale assignment ref.

### Requirement: Drain and replacement are distinct bounded workflows
r[molten.fabric_membership.drain_replace] Aspen MUST distinguish planned drain from failure replacement. Drain MUST stop new assignments or admitted work, invoke extension-owned handoff or checkpoint hooks when selected, observe a bounded grace policy, and release resources. Failure replacement MUST record missing acknowledgements, uncertain state or ownership, selected recovery policy, and any availability or safety degradation.

#### Scenario: Planned node drain completes
- GIVEN an active node enters planned drain with sufficient replacement capacity
- WHEN its roles checkpoint or hand off and stop within grace
- THEN replacements activate under newer assignment epochs and the old reservations release.

#### Scenario: Failed role has uncertain ownership
- GIVEN a role becomes unreachable before acknowledging drain or release
- WHEN replacement policy runs
- THEN evidence records uncertain old-process status and the fencing profile required before replacement effects become authoritative.

### Requirement: Live and simulated providers preserve one contract
r[molten.fabric_membership.live_sim_parity] Aspen MUST provide live and deterministic-simulation membership, failure-observation, resource-inventory, and assignment providers that preserve the same canonical views, placement inputs, lifecycle transitions, fencing validation, and failure classes. Placement core code MUST run unchanged across profiles.

#### Scenario: Simulation injects membership change
- GIVEN a deterministic run changes a canonical membership view and failure observations
- WHEN placement and recruitment execute
- THEN they consume the same values and transitions used by live providers.

#### Scenario: Provider-specific authority is visible
- GIVEN a simulation-owned view is deterministic but not externally authoritative
- WHEN evidence is rendered
- THEN its source profile and non-claims remain distinct from a live quorum-backed provider.

### Requirement: Connectivity, membership, placement, and authority remain separate
r[molten.fabric_membership.authority_separation] Aspen MUST keep transport connectivity, failure suspicion, membership inclusion, placement recommendation, committed assignment, capability authority, and consistency ordering as separate canonical facts. No one fact MUST silently imply another.

#### Scenario: Connected node is not automatically eligible
- GIVEN a peer has an authenticated transport session but lacks the current membership view or required node evidence
- WHEN placement evaluates it
- THEN it is not eligible merely because it is connected.

#### Scenario: Placement plan is not assignment
- GIVEN a pure placement plan selects a node
- WHEN no admitted assignment authority commits the plan
- THEN the node does not start the role and the plan remains advisory.

### Requirement: Membership and placement evidence is bounded
r[molten.fabric_membership.evidence] Aspen MUST emit canonical evidence for admitted views, material failure observations, placement plans, unsatisfied constraints, assignment transitions, fencing changes, drains, replacements, and aggregate resource state. Operator readback MUST expose current refs and bounded reasons without leaking secrets or treating observations as stronger authority.

#### Scenario: Operator explains assignment
- GIVEN a role is active
- WHEN an authorized operator requests placement status
- THEN readback shows its view, plan, constraints, assignment epoch, fencing profile, resource reservation, lifecycle state, and evidence refs.

### Requirement: Membership and placement validation covers success and failure
r[molten.fabric_membership.final_validation] Aspen MUST include positive and negative tests for membership sources, stale and split views, node descriptor schemas, locality, failure observations, deterministic placement, unsatisfied constraints, resources, recruitment races, stale epochs and tokens, weak-profile overclaims, drain, failure replacement, provider conformance, and cleanup.

#### Scenario: Diverse placement fixture passes
- GIVEN eligible nodes satisfy capacity, locality, anti-affinity, policy, and authority constraints
- WHEN placement and recruitment run
- THEN the canonical plan and assignment lifecycle validate.

#### Scenario: Split observations do not fabricate one view
- GIVEN two providers expose conflicting non-authoritative observations
- WHEN no authority profile reconciles them
- THEN validation preserves the conflict or denies placement requiring agreement
- AND does not fabricate a globally agreed membership view.

### Requirement: Pending intents and Quarantined role assignments have explicit recovery paths without bricking role slots
r[molten.fabric_membership.recoverable_pending_and_quarantined_slots] `AssignmentPersistence` MUST support inspecting, aborting definite pre-effect failures, and reconciling uncertain pending intents. Role assignment succession MUST permit a fenced, strictly higher-epoch successor (or explicit fenced release) after `AssignmentState::Quarantined` so role slots cannot be permanently bricked.

#### Scenario: Pre-effect failure rolls back pending intent cleanly
- GIVEN `record_intent` persisted a pending intent and `run_role_effect` failed with `effect_may_have_happened == false`
- WHEN pre-effect abort runs
- THEN the pending intent MUST clear and subsequent transitions on that role slot MUST proceed.

#### Scenario: Fenced successor replaces a Quarantined assignment
- GIVEN an assignment slot entered `AssignmentState::Quarantined`
- WHEN an authorized higher-epoch successor presents valid fencing/reconciliation evidence
- THEN initialization MUST admit the successor rather than failing permanently on `predecessor.state != Released`.

### Requirement: Placement ranks healthy nodes before degraded nodes and failure reduction is deterministic per time basis
r[molten.fabric_membership.healthy_first_placement_and_time_basis_reduction] Under `allow_degraded = true`, `PlacementSearch::select` MUST rank healthy nodes strictly ahead of `Suspected`/`Unavailable` nodes regardless of `preference_score`. Failure-observation reduction MUST be independent of input slice ordering and MUST NOT compare raw ticks across unrelated `time_basis_ref`s.

#### Scenario: Higher-preference Unavailable node does not displace healthy node
- GIVEN one `Unavailable` node with `preference_score = 100` and enough healthy nodes with `preference_score = 10`
- WHEN `PlacementSearch::select` runs with `allow_degraded = true`
- THEN it MUST select healthy nodes and produce a non-degraded plan.

#### Scenario: Reordered observations produce identical reduction and mismatched time bases do not mix
- GIVEN identical observations supplied in reversed order or observations with distinct `time_basis_ref`s
- WHEN failure reduction runs
- THEN reordered equal-basis observations MUST yield identical output and distinct time bases MUST NOT compare raw ticks.
