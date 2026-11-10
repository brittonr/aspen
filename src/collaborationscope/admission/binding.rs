use super::*;

pub const EXTENSION_ID: &str = "molten.collaboration-scope";

/// Captured only from an active, already admitted system-extension host.
#[derive(Debug)]
pub struct Installation {
    generation: u64,
    service: String,
    manifest: String,
    group: String,
    time_profile: String,
    max_bytes: u64,
    bindings: Vec<String>,
    consumed_reads: std::collections::BTreeSet<String>,
}

/// A fresh quorum read plus admitted logical time. It is not clonable and is
/// consumed at the protected admission boundary, never decoded from a snapshot.
pub struct CurrentState {
    pub(super) currentness: Currentness,
    pub(super) state: StateRef,
}

impl CurrentState {
    pub const fn currentness(&self) -> &Currentness {
        &self.currentness
    }
}

impl Installation {
    // r[impl molten.collaboration_scope.versioned_extension]
    pub fn from_host<E: crate::system_extension::SystemExtensionExecutor>(
        host: &crate::system_extension::Dispatcher<E>,
        group: &crate::fabric_consistency::Group,
    ) -> crate::error::Result<Self> {
        let manifest = host.manifest().manifest();
        if manifest.extension_id != EXTENSION_ID
            || manifest.state_schema != SCHEMA
            || host.state().phase != crate::system_extension::LifecyclePhase::Running
        {
            return Err(denied(Denial::Stale));
        }
        if group.extension_id != manifest.extension_id
            || group.service_id != manifest.service_id
            || group.service_generation != host.state().generation
            || group.application_manifest_ref != host.manifest().manifest_ref()
            || group.lifecycle != crate::fabric_consistency::GroupLifecycle::Active
        {
            return Err(denied(Denial::Stale));
        }
        let bindings = host.manifest().required_port_bindings();
        for class in [
            crate::fabric::FabricPortClass::Consistency,
            crate::fabric::FabricPortClass::DurableState,
            crate::fabric::FabricPortClass::Time,
            crate::fabric::FabricPortClass::Authority,
            crate::fabric::FabricPortClass::Resources,
            crate::fabric::FabricPortClass::Evidence,
        ] {
            if !bindings.iter().any(|binding| binding.binding.class == class) {
                return Err(denied(Denial::Missing));
            }
        }
        let time = bindings
            .iter()
            .find(|binding| binding.binding.class == crate::fabric::FabricPortClass::Time)
            .ok_or_else(|| denied(Denial::Missing))?;
        Ok(Self {
            generation: host.state().generation,
            service: manifest.service_id.clone(),
            group: group.binding_ref.clone(),
            manifest: host.manifest().manifest_ref().to_string(),
            time_profile: time.binding.implementation_profile.clone(),
            max_bytes: manifest.resources.max_inflight_bytes,
            bindings: bindings.iter().map(|binding| binding.binding_ref.clone()).collect(),
            consumed_reads: std::collections::BTreeSet::new(),
        })
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn validate_replica(
        &self,
        replica: &crate::fabric_consistency::raft::ReplicaState,
    ) -> crate::error::Result<()> {
        if replica.profile.service_generation != self.generation
            || replica.profile.group_binding_ref != self.group
            || replica.lifecycle != crate::fabric_consistency::raft::ReplicaLifecycle::Running
            || replica.last_applied != replica.commit_index
        {
            return Err(denied(Denial::Stale));
        }
        Ok(())
    }

    /// Validate locally received quorum evidence without consuming it or creating authority.
    /// Callers must not accept a wire projection as a control-port observation.
    pub fn observe_currentness(&self, source: &CurrentInput<'_>) -> crate::error::Result<Currentness> {
        self.validate_replica(source.replica)?;
        if source.time_profile.profile_id != self.time_profile {
            return Err(denied(Denial::Domain));
        }
        let crate::fabric_consistency::raft::ReplicaObservationKind::Read {
            request_ref,
            mode,
            disposition,
            observed_index,
        } = &source.observation.kind
        else {
            return Err(denied(Denial::Partition));
        };
        if request_ref != source.request.as_str()
            || *mode != crate::fabric_consistency::ReadMode::Linearizable
            || *disposition != crate::fabric_consistency::raft::ReadDisposition::Current
            || *observed_index != source.replica.last_applied
        {
            return Err(denied(Denial::Partition));
        }
        let proof = source
            .ledger
            .records()
            .iter()
            .rev()
            .find(|record| {
                record.kind == crate::fabric_consistency::raft::ReplicaKind::ReadCurrentness
                    && record.source_ref == source.observation.receipt_ref
            })
            .ok_or_else(|| denied(Denial::Partition))?;
        if proof.term != source.replica.current_term
            || proof.index != *observed_index
            || proof.quorum_evidence_ref.is_none()
            || proof.quorum_members.len() < 2
        {
            return Err(denied(Denial::Partition));
        }
        let logical = crate::fabric_time::TimeValue::Logical(crate::fabric_time::LogicalEventTime {
            profile_ref: source.time_profile.profile_ref.clone(),
            position: *observed_index,
        });
        crate::fabric_time::validate_time_value(source.time_profile, &logical).map_err(|_| denied(Denial::Domain))?;
        Ok(Currentness {
            service_generation: self.generation,
            consistency_epoch: source.replica.current_term,
            committed_index: *observed_index,
            logical_now: *observed_index,
            revocation: source.revocation.clone(),
            observation: molten_core::nominal::EvidenceRef::new(source.observation.receipt_ref.clone())
                .map_err(|_| denied(Denial::Domain))?,
        })
    }

    /// Consume locally received quorum evidence at the protected admission boundary.
    pub fn current_scope(&mut self, source: &CurrentInput<'_>) -> crate::error::Result<CurrentState> {
        let currentness = self.observe_currentness(source)?;
        if self.consumed_reads.len() >= MAX_OPERATIONS {
            return Err(denied(Denial::Bound));
        }
        if !self.consumed_reads.insert(source.observation.receipt_ref.clone()) {
            return Err(denied(Denial::Replay));
        }
        Ok(CurrentState {
            currentness,
            state: source.state.clone(),
        })
    }

    pub fn status_value(&self) -> preserves::IOValue {
        crate::preserves_rail::record("collaboration-installation-v1", vec![
            crate::preserves_rail::string(&self.service),
            crate::preserves_rail::string(&self.manifest),
            crate::preserves_rail::u64_value(self.generation),
            crate::preserves_rail::u64_value(self.max_bytes),
            crate::preserves_rail::sequence(self.bindings.iter().map(crate::preserves_rail::string).collect()),
        ])
    }
}

pub struct CurrentInput<'a> {
    pub replica: &'a crate::fabric_consistency::raft::ReplicaState,
    pub observation: &'a crate::fabric_consistency::raft::ReplicaObservation,
    pub ledger: &'a crate::fabric_consistency::raft::ReplicaLedger,
    pub request: &'a RequestRef,
    pub time_profile: &'a crate::fabric_time::AdmittedTimeProfile,
    pub revocation: &'a molten_core::nominal::RevocationRef,
    pub state: &'a StateRef,
}
