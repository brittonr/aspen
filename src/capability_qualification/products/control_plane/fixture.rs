//! Fixed declarations of the qualification live Raft group: the admitted group,
//! static membership, replica profile, port bindings, durable namespace, time
//! profile and Iroh transport. Every reference and endpoint secret is derived
//! from a fixed label, so each re-execution binds the same group, membership
//! and replica endpoint identities.

pub(super) mod iroh;

pub(super) const NODE_A: &str = "node-a";
pub(super) const NODE_B: &str = "node-b";
pub(super) const NODE_C: &str = "node-c";
pub(super) const NODES: [&str; crate::fabric_consistency::raft::STATIC_VOTER_COUNT] = [NODE_A, NODE_B, NODE_C];
/// The service generation and fencing epoch start above the initial epoch so a
/// stale-generation or stale-fencing frame is still well formed on the wire.
pub(super) const SERVICE_GENERATION: u64 = 2;
pub(super) const FENCING_EPOCH: u64 = 2;
const CONFIG_EPOCH: u64 = 1;
const COMMAND_BYTE_LIMIT: u64 = 4_096;
const IN_FLIGHT_LIMIT: u32 = 8;
const HEARTBEAT_TICKS: u64 = 2;
const ELECTION_MIN_TICKS: u64 = 4;
const ELECTION_MAX_TICKS: u64 = 8;
/// Timer events are never drained by the replica peers: every election and
/// heartbeat is an explicit, phase-signalled event, so no timing-dependent
/// campaign can start.
pub(super) const TICK_DURATION: std::time::Duration = std::time::Duration::from_secs(10);
const REQUEST_RETENTION_ENTRIES: u64 = 2_048;
const EFFECTS_PER_STEP: usize = 16;
const TIME_PROFILE_LIMIT: u64 = 128;
const TIME_ENTROPY_TOTAL_LIMIT: u64 = 1_024;
const TIME_CONCURRENCY_LIMIT: u64 = 4;
const TIME_QUEUE_LIMIT: u64 = 8;
const DURABLE_LIMIT: u64 = 32;
/// The live Raft durability port never compacts its log and brackets durable
/// transitions with recovery-intent hard-state records.
const DURABLE_LOG_RECORDS: u64 = 128;
const DURABLE_OPERATION_BYTES: u64 = 1_024;
const DURABLE_NAMESPACE_BYTES: u64 = 16_384;
const DURABLE_GENERATION: u64 = 1;
const NON_CLAIM: &str = "live-startup-does-not-prove-production-consensus";

/// A fixed, labeled content reference of this qualification group.
pub(super) fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(format!("aspen-qualification-live-raft/{label}").as_bytes())
}

/// The canonical active group, admitted through the production plan/outcome lifecycle.
pub(super) fn group() -> crate::error::Result<crate::fabric_consistency::Group> {
    let declared = crate::fabric_consistency::canonical_group(crate::fabric_consistency::GroupInput {
        group_id: "group:qualification-live-raft".to_string(),
        extension_id: "extension-qualification-live-raft".to_string(),
        service_id: "service-qualification-live-raft".to_string(),
        service_generation: SERVICE_GENERATION,
        application_manifest_ref: reference("application-manifest"),
        engine_algorithm_profile: crate::fabric_consistency::raft::LIVE_RAFT_ALGORITHM_PROFILE.to_string(),
        engine_implementation_profile: crate::fabric_consistency::raft::LIVE_RAFT_IMPLEMENTATION_PROFILE.to_string(),
        membership_ref: reference("membership"),
        config_epoch: CONFIG_EPOCH,
        placement_ref: reference("placement"),
        fencing_ref: reference("fencing"),
        fencing_epoch: FENCING_EPOCH,
        resource_profile_ref: reference("resources"),
        policy_refs: vec![reference("policy")],
        non_claims: vec![NON_CLAIM.to_string()],
        supported_read_modes: vec![crate::fabric_consistency::ReadMode::Linearizable],
        max_command_bytes: COMMAND_BYTE_LIMIT,
        max_in_flight_operations: IN_FLIGHT_LIMIT,
    })?;
    let plan = crate::fabric_consistency::plan(&declared, crate::fabric_consistency::PortCommandInput {
        request_ref: reference("open-request"),
        binding_ref: declared.binding_ref.clone(),
        group_id: declared.group_id.clone(),
        extension_id: declared.extension_id.clone(),
        service_id: declared.service_id.clone(),
        service_generation: declared.service_generation,
        application_manifest_ref: declared.application_manifest_ref.clone(),
        engine_algorithm_profile: declared.engine_algorithm_profile.clone(),
        engine_implementation_profile: declared.engine_implementation_profile.clone(),
        membership_ref: declared.membership_ref.clone(),
        config_epoch: declared.config_epoch,
        placement_ref: declared.placement_ref.clone(),
        fencing_ref: declared.fencing_ref.clone(),
        fencing_epoch: declared.fencing_epoch,
        resource_profile_ref: declared.resource_profile_ref.clone(),
        policy_refs: declared.policy_refs.clone(),
        authority_refs: vec![reference("authority")],
        observed_in_flight_operations: 0,
        operation: crate::fabric_consistency::Request::Open {
            mode: crate::fabric_consistency::GroupOpenMode::Create,
        },
    })?;
    let outcome = crate::fabric_consistency::normalize(&declared, &plan, crate::fabric_consistency::Input {
        request_ref: plan.request_ref.clone(),
        binding_ref: declared.binding_ref.clone(),
        service_generation: declared.service_generation,
        config_epoch: declared.config_epoch,
        fencing_epoch: declared.fencing_epoch,
        kind: crate::fabric_consistency::Kind::Opened,
        result_ref: Some(reference("open-result")),
        evidence_refs: vec![reference("open-evidence")],
        diagnostics: Vec::new(),
    })?;
    crate::fabric_consistency::apply_outcome(&declared, &plan, &outcome)
}

fn membership(group: &crate::fabric_consistency::Group) -> crate::fabric_consistency::raft::StaticMembership {
    crate::fabric_consistency::raft::StaticMembership {
        membership_ref: group.membership_ref.clone(),
        config_epoch: group.config_epoch,
        voters: NODES.iter().map(ToString::to_string).collect(),
    }
}

/// The production live start plan of one replica.
pub(super) fn start_plan(
    group: &crate::fabric_consistency::Group,
    node_id: &str,
    timer_profile_ref: &str,
) -> crate::error::Result<crate::fabric_consistency::raft::ReplicaStartPlan> {
    crate::fabric_consistency::raft::plan_live_replica_start(crate::fabric_consistency::raft::ReplicaStartInput {
        group: group.clone(),
        node_id: node_id.to_string(),
        membership: membership(group),
        profile: replica_profile(group, node_id, timer_profile_ref),
        port_bindings: crate::fabric_consistency::raft::REQUIRED_REPLICA_PORTS
            .iter()
            .map(|(port_id, version)| crate::fabric_consistency::raft::ReplicaPortBinding {
                port_id: (*port_id).to_string(),
                version: (*version).to_string(),
                implementation_profile: "aspen-qualification-live-raft-v1".to_string(),
                binding_ref: reference(&format!("port-binding/{port_id}")),
            })
            .collect(),
    })
}

fn replica_profile(
    group: &crate::fabric_consistency::Group,
    node_id: &str,
    timer_profile_ref: &str,
) -> crate::fabric_consistency::raft::ReplicaProfile {
    crate::fabric_consistency::raft::ReplicaProfile {
        profile_ref: reference("replica-profile"),
        group_binding_ref: group.binding_ref.clone(),
        service_generation: group.service_generation,
        protocol_ref: reference("protocol"),
        durable_log_ref: reference(&format!("{node_id}/durable-log")),
        snapshot_store_ref: reference(&format!("{node_id}/snapshot-store")),
        timer_profile_ref: timer_profile_ref.to_string(),
        entropy_profile_ref: reference("entropy-profile"),
        placement_ref: group.placement_ref.clone(),
        fencing_ref: group.fencing_ref.clone(),
        fencing_epoch: group.fencing_epoch,
        supervision_ref: reference("supervision"),
        resource_profile_ref: group.resource_profile_ref.clone(),
        heartbeat_ticks: HEARTBEAT_TICKS,
        election_min_ticks: ELECTION_MIN_TICKS,
        election_max_ticks: ELECTION_MAX_TICKS,
        max_log_entries: crate::fabric_consistency::raft::MAX_REPLICA_LOG_ENTRIES,
        snapshot_threshold_entries: crate::fabric_consistency::raft::MAX_REPLICA_LOG_ENTRIES / 2,
        request_retention_entries: REQUEST_RETENTION_ENTRIES,
        max_message_entries: crate::fabric_consistency::raft::MAX_REPLICA_MESSAGE_ENTRIES,
        max_effects_per_step: EFFECTS_PER_STEP,
    }
}

/// The runtime identity the concrete adapter cohort must match exactly.
pub(super) fn runtime_identity(
    plan: &crate::fabric_consistency::raft::ReplicaStartPlan,
) -> crate::fabric_consistency::raft::ReplicaRuntimePortIdentity {
    let profile = &plan.state.profile;
    crate::fabric_consistency::raft::ReplicaRuntimePortIdentity {
        service_id: plan.service_id.clone(),
        service_generation: profile.service_generation,
        group_binding_ref: profile.group_binding_ref.clone(),
        application_manifest_ref: plan.application_manifest_ref.clone(),
        protocol_ref: profile.protocol_ref.clone(),
        durable_log_ref: profile.durable_log_ref.clone(),
        snapshot_store_ref: profile.snapshot_store_ref.clone(),
        timer_profile_ref: profile.timer_profile_ref.clone(),
        entropy_profile_ref: profile.entropy_profile_ref.clone(),
        membership_ref: plan.state.membership.membership_ref.clone(),
        placement_ref: profile.placement_ref.clone(),
        fencing_ref: profile.fencing_ref.clone(),
        supervision_ref: profile.supervision_ref.clone(),
        resource_profile_ref: profile.resource_profile_ref.clone(),
        fabric_binding_refs: plan.port_binding_refs.clone(),
    }
}

pub(super) fn command_schema_ref() -> String {
    reference("command-schema")
}

pub(super) fn time_profile() -> crate::error::Result<crate::fabric_time::Profile> {
    crate::fabric_time::admit_profile(&crate::fabric_time::TimeProfileDescriptor {
        schema: crate::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: "aspen-qualification-live-raft-time".to_string(),
        profile_ref: reference("time-profile"),
        kind: crate::fabric_time::TimeProfileKind::Live,
        supported_domains: crate::fabric_time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: TIME_PROFILE_LIMIT,
        max_uncertainty_ticks: TIME_PROFILE_LIMIT,
        max_timers: TIME_PROFILE_LIMIT,
        max_runnables: TIME_PROFILE_LIMIT,
        max_entropy_request_bytes: TIME_PROFILE_LIMIT,
        max_entropy_total_bytes: TIME_ENTROPY_TOTAL_LIMIT,
        max_scheduler_concurrency: TIME_CONCURRENCY_LIMIT,
        max_scheduler_queue_depth: TIME_QUEUE_LIMIT,
        fairness_bound_turns: None,
        scheduler_policy: crate::fabric_time::SchedulerPolicy {
            ordering: crate::fabric_time::SchedulerOrdering::Fifo,
            replay: crate::fabric_time::SchedulerReplayPolicy::RecordedChoiceRequired,
            overload: crate::fabric_time::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: crate::fabric_time::TimeEvidenceMode::Aggregate,
        non_claims: crate::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    })
}

pub(super) fn durable_profile() -> crate::error::Result<crate::fabric_durability::DurableProfile> {
    crate::fabric_durability::durable_profile(&crate::fabric_durability::DurableStateProfile {
        schema: crate::fabric_durability::DURABLE_STATE_PROFILE_SCHEMA.to_string(),
        profile_id: crate::fabric_durability::DurableAdapterKind::LiveRedb.as_str().to_string(),
        profile_ref: reference("durable-profile"),
        adapter_kind: crate::fabric_durability::DurableAdapterKind::LiveRedb,
        supported_levels: durability_levels(),
        max_namespaces: DURABLE_LIMIT,
        max_log_records: DURABLE_LOG_RECORDS,
        max_ordered_entries: DURABLE_LIMIT,
        max_operation_bytes: DURABLE_OPERATION_BYTES,
        max_namespace_bytes: DURABLE_NAMESPACE_BYTES,
        max_batch_operations: DURABLE_LIMIT,
        max_snapshots: DURABLE_LIMIT,
        max_effect_transactions: DURABLE_LIMIT,
        non_claims: crate::fabric_durability::REQUIRED_DURABILITY_NON_CLAIMS.to_vec(),
    })
}

pub(super) fn durable_namespace(node_id: &str) -> crate::fabric_durability::DurableNamespaceDescriptor {
    let adapter_id = format!("qualification-live-raft-{node_id}-adapter");
    let namespace_id = format!("qualification-live-raft-{node_id}-namespace");
    crate::fabric_durability::DurableNamespaceDescriptor {
        schema: crate::fabric_durability::DURABLE_STATE_NAMESPACE_SCHEMA.to_string(),
        profile_ref: reference("durable-profile"),
        adapter_id: adapter_id.clone(),
        namespace_id: namespace_id.clone(),
        generation: DURABLE_GENERATION,
        value_schema_ref: reference("durable-value-schema"),
        atomicity_domain: crate::fabric_durability::AtomicityDomain {
            domain_id: format!("qualification-live-raft-{node_id}-domain"),
            adapter_id,
            namespace_id,
            generation: DURABLE_GENERATION,
            object_classes: vec![
                crate::fabric_durability::DurableObjectClass::LogRecord,
                crate::fabric_durability::DurableObjectClass::OrderedValue,
                crate::fabric_durability::DurableObjectClass::Snapshot,
                crate::fabric_durability::DurableObjectClass::Checkpoint,
                crate::fabric_durability::DurableObjectClass::EffectTransaction,
            ],
            max_operations: DURABLE_LIMIT,
            max_bytes: DURABLE_OPERATION_BYTES,
            supported_levels: durability_levels(),
        },
        retention_authority_ref: Some(reference("durable-retention-authority")),
        quota_bytes: DURABLE_NAMESPACE_BYTES,
    }
}

fn durability_levels() -> Vec<crate::fabric_durability::DurabilityLevel> {
    vec![
        crate::fabric_durability::DurabilityLevel::Buffered,
        crate::fabric_durability::DurabilityLevel::ProcessLoss,
        crate::fabric_durability::DurabilityLevel::MachineLoss,
    ]
}
