// Explicit membership and time inputs for the fabric boundary compatibility fixtures.
//
// `inputs.rs`, `ports.rs`, and `cases.rs` are shared verbatim by the pre-migration fixture
// generator (run once at `3de348149^`, recorded in the change evidence) and by
// `tests/fabricboundarycompat.rs`. They use only APIs present before and after the fabric
// port/adapter migration.

pub const NOW_TICKS: u64 = 100;
const OBSERVED_AT_TICKS: u64 = 90;
const VALID_UNTIL_TICKS: u64 = 110;
const MAX_VIEW_AGE_TICKS: u64 = 20;
const VIEW_EPOCH: u64 = 7;
const SERVICE_GENERATION: u64 = 3;
const ASSIGNMENT_EPOCH: u64 = 11;
const FENCING_TOKEN: u64 = 41;
const CPU_CAPACITY: u64 = 2_000;
const MEMORY_CAPACITY: u64 = 8_000;
const STORAGE_CAPACITY: u64 = 16_000;
const TIME_PROFILE_LIMIT: u64 = 128;
const TIME_ENTROPY_TOTAL_LIMIT: u64 = 1_024;
const TIME_CONCURRENCY_LIMIT: u64 = 4;
const TIME_QUEUE_LIMIT: u64 = 8;

/// Deterministic evidence ref for an input label: BLAKE3 over the label bytes.
pub fn input_ref(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

pub fn compatibility_ref() -> String {
    input_ref("fabric-boundary-compatibility-v1")
}

pub fn membership_source_profile() -> molten::fabric_membership::MembershipSourceProfile {
    molten::fabric_membership::MembershipSourceProfile {
        schema: molten::fabric_membership::MEMBERSHIP_SOURCE_PROFILE_SCHEMA.to_string(),
        profile_id: "fabric-boundary-membership-profile-v1".to_string(),
        profile_ref: input_ref("membership-profile"),
        provider_kind: molten::fabric_membership::MembershipProviderKind::Static,
        authority_strength: molten::fabric_membership::MembershipAuthorityStrength::OperatorDeclared,
        authority_scope: "cluster-a".to_string(),
        max_view_age_ticks: MAX_VIEW_AGE_TICKS,
        non_claims: molten::fabric_membership::REQUIRED_MEMBERSHIP_NON_CLAIMS.to_vec(),
    }
}

fn node_descriptor(node_id: &str, zone: &str) -> molten::fabric_membership::NodeDescriptor {
    molten::fabric_membership::NodeDescriptor {
        schema: molten::fabric_membership::NODE_DESCRIPTOR_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        descriptor_ref: input_ref(&format!("descriptor-{node_id}")),
        compatibility_ref: compatibility_ref(),
        labels: vec![molten::fabric_membership::NodeLabel {
            key: "zone".to_string(),
            value: zone.to_string(),
            authority: molten::fabric_membership::LabelAuthority::Authoritative,
            evidence_ref: input_ref(&format!("descriptor-{node_id}-zone")),
        }],
        runtime_features: vec!["system-extension-v1".to_string()],
        capacity: molten::fabric_membership::ResourceAmount {
            cpu_millis: CPU_CAPACITY,
            memory_bytes: MEMORY_CAPACITY,
            storage_bytes: STORAGE_CAPACITY,
        },
    }
}

pub fn node_descriptors() -> Vec<molten::fabric_membership::NodeDescriptor> {
    vec![node_descriptor("node-a", "zone-a"), node_descriptor("node-b", "zone-b")]
}

pub fn membership_view(
    profile: &molten::fabric_membership::MembershipSourceProfile,
    descriptors: &[molten::fabric_membership::NodeDescriptor],
) -> molten::fabric_membership::MembershipView {
    molten::fabric_membership::MembershipView {
        schema: molten::fabric_membership::MEMBERSHIP_VIEW_SCHEMA.to_string(),
        view_id: "view-a".to_string(),
        epoch: VIEW_EPOCH,
        source_profile_ref: profile.profile_ref.clone(),
        source_evidence_ref: input_ref("membership-source-evidence"),
        authority_ref: input_ref("membership-authority"),
        eligibility_policy_ref: input_ref("membership-eligibility-policy"),
        observed_at_ticks: OBSERVED_AT_TICKS,
        valid_until_ticks: VALID_UNTIL_TICKS,
        members: descriptors
            .iter()
            .map(|descriptor| molten::fabric_membership::MembershipMember {
                node_id: descriptor.node_id.clone(),
                descriptor_ref: descriptor.descriptor_ref.clone(),
                eligibility_ref: input_ref(&format!("eligibility-{}", descriptor.node_id)),
            })
            .collect(),
    }
}

pub fn assignment_proposal() -> molten::fabric_membership::AssignmentProposal {
    molten::fabric_membership::AssignmentProposal {
        assignment_id: "assignment-a".to_string(),
        extension_id: "extension-a".to_string(),
        service_id: "service-a".to_string(),
        role_id: "replica-0".to_string(),
        role_kind: "replica".to_string(),
        node_id: "node-a".to_string(),
        service_generation: SERVICE_GENERATION,
        assignment_epoch: ASSIGNMENT_EPOCH,
        fencing_token: FENCING_TOKEN,
        fencing_profile_ref: input_ref("fencing-profile"),
        resource_reservation_ref: input_ref("resource-reservation"),
        placement_plan_ref: input_ref("placement-plan"),
        authority_ref: input_ref("assignment-authority"),
        durable_state_ref: Some(input_ref("assignment-durable-state")),
        predecessor_assignment_ref: None,
        predecessor_epoch: None,
    }
}

pub fn reserve_command(
    proposal: &molten::fabric_membership::AssignmentProposal,
) -> molten::fabric_membership::AssignmentCommand {
    molten::fabric_membership::AssignmentCommand {
        kind: molten::fabric_membership::AssignmentCommandKind::Reserve,
        assignment_id: proposal.assignment_id.clone(),
        service_generation: proposal.service_generation,
        assignment_epoch: proposal.assignment_epoch,
        fencing_token: proposal.fencing_token,
        transition_ref: input_ref("assignment-reserve"),
        uncertain_old_owner: false,
    }
}

pub fn time_profile_descriptor() -> molten::fabric_time::TimeProfileDescriptor {
    molten::fabric_time::TimeProfileDescriptor {
        schema: molten::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: "fabric-boundary-simulation-time".to_string(),
        profile_ref: input_ref("time-profile"),
        kind: molten::fabric_time::TimeProfileKind::DeterministicSimulation,
        supported_domains: molten::fabric_time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: TIME_PROFILE_LIMIT,
        max_uncertainty_ticks: TIME_PROFILE_LIMIT,
        max_timers: TIME_PROFILE_LIMIT,
        max_runnables: TIME_PROFILE_LIMIT,
        max_entropy_request_bytes: TIME_PROFILE_LIMIT,
        max_entropy_total_bytes: TIME_ENTROPY_TOTAL_LIMIT,
        max_scheduler_concurrency: TIME_CONCURRENCY_LIMIT,
        max_scheduler_queue_depth: TIME_QUEUE_LIMIT,
        fairness_bound_turns: None,
        scheduler_policy: molten::fabric_time::SchedulerPolicy {
            ordering: molten::fabric_time::SchedulerOrdering::Fifo,
            replay: molten::fabric_time::SchedulerReplayPolicy::Deterministic,
            overload: molten::fabric_time::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: molten::fabric_time::TimeEvidenceMode::Aggregate,
        non_claims: molten::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}
