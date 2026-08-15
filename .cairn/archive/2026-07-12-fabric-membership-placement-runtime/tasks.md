## Phase 1: Membership and observation models

- [x] [serial] Add canonical node descriptors, locality and fault-domain labels, membership views, source profiles, freshness, authority refs, and non-claims. r[molten.fabric_membership.membership_views] r[molten.fabric_membership.locality]
- [x] [serial] Add pure validation for member identity, view ordering and epochs, descriptor compatibility, label schemas, freshness, eligibility, and source-profile claims. r[molten.fabric_membership.membership_views]
- [x] [parallel] Add positive view fixtures and negative duplicate-member, stale-view, incompatible-node, forged-source, malformed-label, and overclaim fixtures. r[molten.fabric_membership.membership_views] r[molten.fabric_membership.locality]

## Phase 2: Failure observation and placement core

- [x] [serial] Add pluggable bounded failure observations with detector profile, time basis, freshness, confidence, and non-authoritative outcomes. r[molten.fabric_membership.failure_detector]
- [x] [serial] Implement a deterministic pure placement planner over role requirements, membership, resources, locality, anti-affinity, policy, current assignments, failure observations, and tie-break input. r[molten.fabric_membership.placement]
- [x] [parallel] Add satisfied and unsatisfiable placement fixtures, deterministic tie-break tests, capacity exhaustion, degraded placement, and split-view negative tests. r[molten.fabric_membership.placement]

## Phase 3: Recruitment and fencing

- [x] [serial] Add canonical propose, reserve, assign, acknowledge, activate, drain, replace, release, and fail transitions for extension-owned roles. r[molten.fabric_membership.recruitment]
- [x] [parallel] Bind role assignments to service generation, assignment epoch, fencing token, authority profile, resource reservation, and durable state where selected. r[molten.fabric_membership.fencing]
- [x] [parallel] Deny stale assignments and test duplicate recruitment, delayed acknowledgements, stale tokens, weak-profile overclaims, and replacement races. r[molten.fabric_membership.recruitment] r[molten.fabric_membership.fencing]

## Phase 4: Providers, drain, and evidence

- [x] [serial] Implement static or policy-managed live providers and deterministic-simulation providers behind the same membership and placement contracts. r[molten.fabric_membership.live_sim_parity]
- [x] [parallel] Integrate graceful drain, bounded state handoff or checkpoint, failure replacement, uncertain ownership, and cleanup with system-extension supervision. r[molten.fabric_membership.drain_replace]
- [x] [parallel] Add bounded view, suspicion, placement-plan, assignment, fencing, drain, and replacement evidence plus operator readback. r[molten.fabric_membership.evidence]
- [x] [parallel] Enforce separation among connectivity, suspicion, membership, placement, assignment, and consistency authority. r[molten.fabric_membership.authority_separation]

## Phase 5: Validation

- [x] [serial] Run membership, detector, deterministic placement, locality, resource, fencing, recruitment, split-view, drain, replacement, provider conformance, and cleanup tests. r[molten.fabric_membership.final_validation]
- [x] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.fabric_membership.final_validation]
