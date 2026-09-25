
impl ExtensionDurabilityContext {
    pub fn from_host<E: SystemExtensionExecutor>(
        host: &crate::system_extension::SystemExtensionHost<E>,
        profile: &CanonicalDurableProfile,
    ) -> crate::error::Result<Self> {
        let mut bound_ports = Vec::with_capacity(DURABILITY_PORT_COUNT);
        for port_id in durability_port_ids() {
            let key = crate::fabric::FabricPortKey {
                port_id: port_id.to_string(),
                version: FABRIC_DURABILITY_PORT_VERSION.to_string(),
            };
            if let Some(binding) = host.manifest().binding_for(&key) {
                if binding.binding.implementation_profile != profile.profile.profile_id {
                    return Err(crate::error::MoltenError::invalid_harness(format!(
                        "system-extension durability profile {} does not match {}",
                        binding.binding.implementation_profile, profile.profile.profile_id
                    )));
                }
                bound_ports.push(port_id.to_string());
            }
        }
        if bound_ports.is_empty() {
            return Err(crate::error::MoltenError::invalid_harness(
                "system extension has no admitted durable-state fabric port binding",
            ));
        }
        Ok(Self {
            service_id: host.manifest().manifest().service_id.clone(),
            generation: host.state().generation,
            profile_id: profile.profile.profile_id.clone(),
            max_operation_bytes: profile.profile.max_operation_bytes,
            bound_ports,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_test_snapshot(
        service_id: &str,
        generation: u64,
        profile: &CanonicalDurableProfile,
        bound_ports: Vec<String>,
    ) -> Self {
        Self {
            service_id: service_id.to_string(),
            generation,
            profile_id: profile.profile.profile_id.clone(),
            max_operation_bytes: profile.profile.max_operation_bytes,
            bound_ports,
        }
    }

    pub fn admit_operation(
        &self,
        profile: &CanonicalDurableProfile,
        port_id: &str,
        service_id: &str,
        generation: u64,
        operation_bytes: u64,
    ) -> crate::error::Result<()> {
        if self.profile_id != profile.profile.profile_id {
            return Err(crate::error::MoltenError::invalid_harness("durability profile substitution denied"));
        }
        if !self.bound_ports.iter().any(|bound| bound == port_id) {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "durability port {port_id} is not bound to the system extension"
            )));
        }
        if self.service_id != service_id {
            return Err(crate::error::MoltenError::invalid_harness("durability service identity mismatch"));
        }
        if self.generation != generation {
            return Err(crate::error::MoltenError::invalid_harness(
                "durability operation uses a stale service generation",
            ));
        }
        if operation_bytes > self.max_operation_bytes {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "durability operation bytes {operation_bytes} exceed {}",
                self.max_operation_bytes
            )));
        }
        Ok(())
    }
}

fn durable_profile_value(profile: &DurableStateProfile) -> preserves::IOValue {
    crate::preserves_rail::record(DURABILITY_PROFILE_RECORD, vec![
        crate::preserves_rail::string(DURABLE_STATE_PROFILE_SCHEMA),
        field("profile-id", crate::preserves_rail::string(&profile.profile_id)),
        field("declared-profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("adapter-kind", crate::preserves_rail::string(profile.adapter_kind.as_str())),
        field("durability-levels", strings_value(profile.supported_levels.iter().map(|level| level.as_str()))),
        field("max-namespaces", crate::preserves_rail::u64_value(profile.max_namespaces)),
        field("max-log-records", crate::preserves_rail::u64_value(profile.max_log_records)),
        field("max-ordered-entries", crate::preserves_rail::u64_value(profile.max_ordered_entries)),
        field("max-operation-bytes", crate::preserves_rail::u64_value(profile.max_operation_bytes)),
        field("max-namespace-bytes", crate::preserves_rail::u64_value(profile.max_namespace_bytes)),
        field("max-batch-operations", crate::preserves_rail::u64_value(profile.max_batch_operations)),
        field("max-snapshots", crate::preserves_rail::u64_value(profile.max_snapshots)),
        field("max-effect-transactions", crate::preserves_rail::u64_value(profile.max_effect_transactions)),
        field("non-claims", strings_value(profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "canonical-profile",
            "atomicity-domain-explicit",
            "durability-boundary-explicit",
            "local-only-non-claims-complete",
        ]),
    ])
}

fn durability_port_ids() -> [&'static str; DURABILITY_PORT_COUNT] {
    [
        FABRIC_DURABLE_LOG_PORT_ID,
        FABRIC_ORDERED_STORE_PORT_ID,
        FABRIC_SNAPSHOT_PORT_ID,
        FABRIC_EFFECT_TRANSACTION_PORT_ID,
    ]
}

fn field(name: &str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record("field", vec![crate::preserves_rail::string(name), value])
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.map(crate::preserves_rail::string).collect())
}

fn checks(values: &[&str]) -> preserves::IOValue {
    field("checks", strings_value(values.iter().copied()))
}

fn optional_u64(value: Option<u64>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn count(value: usize) -> crate::error::Result<u64> {
    u64::try_from(value).map_err(|_| crate::error::MoltenError::invalid_harness("durability collection count overflow"))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} validation denied: {issues:?}"))
}
