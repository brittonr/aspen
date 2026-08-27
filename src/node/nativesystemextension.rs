use std::collections::BTreeMap;

use crate::system_extension::AdmittedNativeHostProfile;
use crate::system_extension::NATIVE_INSTANCE_STATE_SCHEMA;
use crate::system_extension::NativeHostIssue;
use crate::system_extension::NativeInstanceRecord;
use crate::system_extension::admit_native_removal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeNodeRegistryIssue {
    Capacity { actual: usize, maximum: usize },
    DuplicateInstance(String),
    UnknownInstance(String),
    SchemaMismatch,
    ProfileMismatch,
    StaleGeneration { actual: u64, active: u64 },
    Removal(Vec<NativeHostIssue>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNodeServiceRegistry {
    profile_ref: String,
    max_instances: usize,
    instances: BTreeMap<String, NativeInstanceRecord>,
}

impl NativeNodeServiceRegistry {
    pub fn new(profile: &AdmittedNativeHostProfile) -> Self {
        Self {
            profile_ref: profile.profile.profile_ref.clone(),
            max_instances: profile.profile.max_instances,
            instances: BTreeMap::new(),
        }
    }

    // r[impl molten.system_extension.native_host.durability]
    // r[impl molten.system_extension.native_host.neutrality]
    pub fn install(&mut self, instance: NativeInstanceRecord) -> Result<(), NativeNodeRegistryIssue> {
        if instance.schema != NATIVE_INSTANCE_STATE_SCHEMA {
            return Err(NativeNodeRegistryIssue::SchemaMismatch);
        }
        if instance.profile_ref != self.profile_ref {
            return Err(NativeNodeRegistryIssue::ProfileMismatch);
        }
        if self.instances.contains_key(&instance.instance_id) {
            return Err(NativeNodeRegistryIssue::DuplicateInstance(instance.instance_id));
        }
        let next_count = self.instances.len().checked_add(1).ok_or(NativeNodeRegistryIssue::Capacity {
            actual: self.instances.len(),
            maximum: self.max_instances,
        })?;
        if next_count > self.max_instances {
            return Err(NativeNodeRegistryIssue::Capacity {
                actual: next_count,
                maximum: self.max_instances,
            });
        }
        self.instances.insert(instance.instance_id.clone(), instance);
        Ok(())
    }

    pub fn replace_recovered(&mut self, instance: NativeInstanceRecord) -> Result<(), NativeNodeRegistryIssue> {
        let Some(current) = self.instances.get(&instance.instance_id) else {
            return Err(NativeNodeRegistryIssue::UnknownInstance(instance.instance_id));
        };
        if instance.profile_ref != self.profile_ref {
            return Err(NativeNodeRegistryIssue::ProfileMismatch);
        }
        if instance.lifecycle.generation < current.lifecycle.generation {
            return Err(NativeNodeRegistryIssue::StaleGeneration {
                actual: instance.lifecycle.generation,
                active: current.lifecycle.generation,
            });
        }
        self.instances.insert(instance.instance_id.clone(), instance);
        Ok(())
    }

    pub fn get(&self, instance_id: &str) -> Option<&NativeInstanceRecord> {
        self.instances.get(instance_id)
    }

    pub fn inventory(&self) -> Vec<&NativeInstanceRecord> {
        self.instances.values().collect()
    }

    pub fn remove(&mut self, instance_id: &str) -> Result<NativeInstanceRecord, NativeNodeRegistryIssue> {
        let instance = self
            .instances
            .get(instance_id)
            .ok_or_else(|| NativeNodeRegistryIssue::UnknownInstance(instance_id.to_string()))?;
        admit_native_removal(instance).map_err(NativeNodeRegistryIssue::Removal)?;
        self.instances
            .remove(instance_id)
            .ok_or_else(|| NativeNodeRegistryIssue::UnknownInstance(instance_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_extension::*;

    const HASH: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const GENERATION: u64 = 1;
    const MAX_INSTANCES: usize = 1;

    fn profile() -> AdmittedNativeHostProfile {
        admit_native_host_profile(&NativeHostProfile {
            schema: NATIVE_HOST_PROFILE_SCHEMA.to_string(),
            profile_id: "node-native-host".to_string(),
            profile_ref: HASH.to_string(),
            execution_profile_ref: HASH.to_string(),
            transport_profile_ref: HASH.to_string(),
            alpn: NATIVE_ALPN.to_string(),
            framing: NATIVE_FRAMING.to_string(),
            max_callback_input_bytes: GENERATION,
            max_callback_output_bytes: GENERATION,
            max_diagnostic_bytes: GENERATION,
            max_instances: MAX_INSTANCES,
            max_unresolved_operations: MAX_INSTANCES,
            max_port_bindings: MAX_INSTANCES,
            max_policy_refs: MAX_INSTANCES,
            is_local_live_pilot: true,
            non_claims: REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
        })
        .expect("node native profile")
    }

    fn stopped(id: &str) -> NativeInstanceRecord {
        NativeInstanceRecord {
            schema: NATIVE_INSTANCE_STATE_SCHEMA.to_string(),
            instance_id: id.to_string(),
            extension_id: "extension".to_string(),
            service_id: "service".to_string(),
            manifest_ref: HASH.to_string(),
            executable_ref: HASH.to_string(),
            profile_ref: HASH.to_string(),
            state_schema_ref: HASH.to_string(),
            lifecycle: LifecycleState {
                generation: GENERATION,
                phase: LifecyclePhase::Stopped,
                restart_attempts: 0,
                health: HealthState::Stopped,
                checkpoint_ref: None,
            },
            usage: ResourceUsage::default(),
            callback_sequence: 0,
            event_sequence: 0,
            checkpoint_ref: None,
            unresolved: Vec::new(),
            completed_operations: Vec::new(),
            completed_operation_refs: Vec::new(),
            evidence_refs: Vec::new(),
            is_accepting_ingress: false,
        }
    }

    #[test]
    fn registry_installs_replaces_and_removes_without_service_name_branches() {
        let mut registry = NativeNodeServiceRegistry::new(&profile());
        registry.install(stopped("one")).expect("install instance");
        assert_eq!(registry.inventory().len(), 1);
        registry.replace_recovered(stopped("one")).expect("replace instance");
        assert_eq!(registry.remove("one").expect("remove instance").instance_id, "one");
    }

    #[test]
    fn registry_rejects_duplicate_capacity_unknown_and_stale_records() {
        let mut registry = NativeNodeServiceRegistry::new(&profile());
        registry.install(stopped("one")).expect("install instance");
        assert!(registry.install(stopped("one")).is_err());
        assert!(registry.install(stopped("two")).is_err());
        assert!(registry.remove("missing").is_err());
        let mut stale = stopped("one");
        stale.lifecycle.generation = 0;
        assert!(registry.replace_recovered(stale).is_err());
    }
}
