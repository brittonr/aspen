#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub group_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub service_generation: u64,
    pub application_manifest_ref: String,
    pub engine_algorithm_profile: String,
    pub engine_implementation_profile: String,
    pub membership_ref: String,
    pub config_epoch: u64,
    pub placement_ref: String,
    pub fencing_ref: String,
    pub fencing_epoch: u64,
    pub resource_profile_ref: String,
    pub policy_refs: Vec<String>,
    pub non_claims: Vec<String>,
    pub max_command_bytes: u64,
    pub max_in_flight_operations: u32,
}

impl Group {
    pub fn open(&self, authority_ref: &str) -> crate::error::Result<crate::fabric_consistency::Group> {
        let group = crate::fabric_consistency::canonical_group(crate::fabric_consistency::GroupInput {
            group_id: self.group_id.clone(),
            extension_id: self.extension_id.clone(),
            service_id: self.service_id.clone(),
            service_generation: self.service_generation,
            application_manifest_ref: self.application_manifest_ref.clone(),
            engine_algorithm_profile: self.engine_algorithm_profile.clone(),
            engine_implementation_profile: self.engine_implementation_profile.clone(),
            membership_ref: self.membership_ref.clone(),
            config_epoch: self.config_epoch,
            placement_ref: self.placement_ref.clone(),
            fencing_ref: self.fencing_ref.clone(),
            fencing_epoch: self.fencing_epoch,
            resource_profile_ref: self.resource_profile_ref.clone(),
            policy_refs: self.policy_refs.clone(),
            non_claims: self.non_claims.clone(),
            supported_read_modes: vec![crate::fabric_consistency::ReadMode::Linearizable],
            max_command_bytes: self.max_command_bytes,
            max_in_flight_operations: self.max_in_flight_operations,
        })?;
        let request_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "live-registry-group-open-v1",
            vec![
                crate::preserves_rail::string(&group.binding_ref),
                crate::preserves_rail::string(authority_ref),
            ],
        ))?;
        let plan = crate::fabric_consistency::plan(&group, crate::fabric_consistency::PortCommandInput {
            request_ref: request_ref.clone(),
            binding_ref: group.binding_ref.clone(),
            group_id: group.group_id.clone(),
            extension_id: group.extension_id.clone(),
            service_id: group.service_id.clone(),
            service_generation: group.service_generation,
            application_manifest_ref: group.application_manifest_ref.clone(),
            engine_algorithm_profile: group.engine_algorithm_profile.clone(),
            engine_implementation_profile: group.engine_implementation_profile.clone(),
            membership_ref: group.membership_ref.clone(),
            config_epoch: group.config_epoch,
            placement_ref: group.placement_ref.clone(),
            fencing_ref: group.fencing_ref.clone(),
            fencing_epoch: group.fencing_epoch,
            resource_profile_ref: group.resource_profile_ref.clone(),
            policy_refs: group.policy_refs.clone(),
            authority_refs: vec![authority_ref.into()],
            observed_in_flight_operations: 0,
            operation: crate::fabric_consistency::Request::Open {
                mode: crate::fabric_consistency::GroupOpenMode::Create,
            },
        })?;
        let outcome = crate::fabric_consistency::normalize(&group, &plan, crate::fabric_consistency::Input {
            request_ref,
            binding_ref: group.binding_ref.clone(),
            service_generation: group.service_generation,
            config_epoch: group.config_epoch,
            fencing_epoch: group.fencing_epoch,
            kind: crate::fabric_consistency::Kind::Opened,
            result_ref: Some(authority_ref.into()),
            evidence_refs: vec![authority_ref.into()],
            diagnostics: Vec::new(),
        })?;
        crate::fabric_consistency::apply_outcome(&group, &plan, &outcome)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Membership {
    pub membership_ref: String,
    pub config_epoch: u64,
    pub voters: Vec<String>,
}

impl Membership {
    pub fn decode(&self) -> crate::fabric_consistency::raft::StaticMembership {
        crate::fabric_consistency::raft::StaticMembership {
            membership_ref: self.membership_ref.clone(),
            config_epoch: self.config_epoch,
            voters: self.voters.clone(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replica {
    pub profile_ref: String,
    pub group_binding_ref: String,
    pub service_generation: u64,
    pub protocol_ref: String,
    pub durable_log_ref: String,
    pub snapshot_store_ref: String,
    pub timer_profile_ref: String,
    pub entropy_profile_ref: String,
    pub placement_ref: String,
    pub fencing_ref: String,
    pub fencing_epoch: u64,
    pub supervision_ref: String,
    pub resource_profile_ref: String,
    pub heartbeat_ticks: u64,
    pub election_min_ticks: u64,
    pub election_max_ticks: u64,
    pub max_log_entries: usize,
    pub max_message_entries: usize,
    pub max_effects_per_step: usize,
}

impl Replica {
    pub fn decode(&self) -> crate::fabric_consistency::raft::ReplicaProfile {
        crate::fabric_consistency::raft::ReplicaProfile {
            profile_ref: self.profile_ref.clone(),
            group_binding_ref: self.group_binding_ref.clone(),
            service_generation: self.service_generation,
            protocol_ref: self.protocol_ref.clone(),
            durable_log_ref: self.durable_log_ref.clone(),
            snapshot_store_ref: self.snapshot_store_ref.clone(),
            timer_profile_ref: self.timer_profile_ref.clone(),
            entropy_profile_ref: self.entropy_profile_ref.clone(),
            placement_ref: self.placement_ref.clone(),
            fencing_ref: self.fencing_ref.clone(),
            fencing_epoch: self.fencing_epoch,
            supervision_ref: self.supervision_ref.clone(),
            resource_profile_ref: self.resource_profile_ref.clone(),
            heartbeat_ticks: self.heartbeat_ticks,
            election_min_ticks: self.election_min_ticks,
            election_max_ticks: self.election_max_ticks,
            max_log_entries: self.max_log_entries,
            max_message_entries: self.max_message_entries,
            max_effects_per_step: self.max_effects_per_step,
        }
    }
}
