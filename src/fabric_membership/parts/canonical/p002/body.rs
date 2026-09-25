
impl ExtensionMembershipPlacementContext {
    pub fn from_host<E: SystemExtensionExecutor>(
        host: &crate::system_extension::SystemExtensionHost<E>,
        profile: &CanonicalMembershipProfile,
    ) -> crate::error::Result<Self> {
        let mut bound_ports = Vec::with_capacity(MEMBERSHIP_PORT_COUNT);
        for port_id in membership_port_ids() {
            let key = crate::fabric::FabricPortKey {
                port_id: port_id.to_string(),
                version: FABRIC_MEMBERSHIP_PORT_VERSION.to_string(),
            };
            if let Some(binding) = host.manifest().binding_for(&key) {
                if binding.binding.implementation_profile != profile.profile.profile_id {
                    return Err(crate::error::MoltenError::invalid_harness(format!(
                        "system-extension membership profile {} does not match {}",
                        binding.binding.implementation_profile, profile.profile.profile_id
                    )));
                }
                bound_ports.push(port_id.to_string());
            }
        }
        if bound_ports.is_empty() {
            return Err(crate::error::MoltenError::invalid_harness(
                "system extension has no admitted membership or placement fabric port binding",
            ));
        }
        Ok(Self {
            service_id: host.manifest().manifest().service_id.clone(),
            generation: host.state().generation,
            profile_id: profile.profile.profile_id.clone(),
            source_profile_ref: profile.profile.profile_ref.clone(),
            bound_ports,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_test_snapshot(
        service_id: &str,
        generation: u64,
        profile: &CanonicalMembershipProfile,
        bound_ports: Vec<String>,
    ) -> Self {
        Self {
            service_id: service_id.to_string(),
            generation,
            profile_id: profile.profile.profile_id.clone(),
            source_profile_ref: profile.profile.profile_ref.clone(),
            bound_ports,
        }
    }

    pub fn admit_plan(
        &self,
        profile: &CanonicalMembershipProfile,
        view: &CanonicalMembershipView,
        service_id: &str,
        generation: u64,
    ) -> crate::error::Result<()> {
        self.admit_scope(profile, FABRIC_PLACEMENT_PORT_ID, service_id, generation)?;
        if view.admitted.profile.profile_ref != self.source_profile_ref {
            return Err(crate::error::MoltenError::invalid_harness(
                "placement view uses a substituted membership source profile",
            ));
        }
        Ok(())
    }

    pub fn admit_assignment(
        &self,
        profile: &CanonicalMembershipProfile,
        assignment: &RoleAssignment,
    ) -> crate::error::Result<()> {
        self.admit_scope(profile, FABRIC_ASSIGNMENT_PORT_ID, &assignment.service_id, assignment.service_generation)?;
        let issues = validate_assignment(assignment);
        if !issues.is_empty() {
            return Err(validation_error("extension assignment", &issues));
        }
        Ok(())
    }

    fn admit_scope(
        &self,
        profile: &CanonicalMembershipProfile,
        port_id: &str,
        service_id: &str,
        generation: u64,
    ) -> crate::error::Result<()> {
        if self.profile_id != profile.profile.profile_id {
            return Err(crate::error::MoltenError::invalid_harness("membership profile substitution denied"));
        }
        if !self.bound_ports.iter().any(|bound| bound == port_id) {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "membership or placement port {port_id} is not bound to the system extension"
            )));
        }
        if self.service_id != service_id {
            return Err(crate::error::MoltenError::invalid_harness("membership service identity mismatch"));
        }
        if self.generation != generation {
            return Err(crate::error::MoltenError::invalid_harness(
                "membership or placement operation uses a stale service generation",
            ));
        }
        Ok(())
    }
}

// r[impl molten.fabric_membership.live_sim_parity]
pub fn fabric_membership_port_descriptors(
    profile: &CanonicalMembershipProfile,
) -> Vec<crate::fabric::FabricPortDescriptor> {
    let (provider_determinism, provider_replay) = provider_replay_classes(profile.profile.provider_kind);
    let definitions = membership_port_definitions(provider_determinism, provider_replay);
    let mut descriptors = Vec::with_capacity(MEMBERSHIP_PORT_COUNT);
    for (port_id, class, operations, output_schema, determinism, replay, authorities) in definitions {
        descriptors.push(crate::fabric::FabricPortDescriptor {
            schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: port_id.to_string(),
            version: FABRIC_MEMBERSHIP_PORT_VERSION.to_string(),
            class,
            operation_classes: operations.into_iter().map(str::to_string).collect(),
            input_schema_refs: vec![MEMBERSHIP_SOURCE_PROFILE_SCHEMA.to_string()],
            output_schema_refs: vec![output_schema.to_string()],
            authority_requirements: authorities,
            resource_requirements: vec![
                crate::fabric::FabricResource::Memory,
                crate::fabric::FabricResource::LogicalTime,
            ],
            determinism,
            replay,
            implementation_profile: profile.profile.profile_id.clone(),
            conformance_refs: vec![profile.admission_ref.clone(), profile.profile.profile_ref.clone()],
            non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        });
    }
    descriptors
}

/// A membership port's id, class, operations, output schema, determinism, replay, and authorities.
type PortDefinition = (
    &'static str,
    crate::fabric::FabricPortClass,
    Vec<&'static str>,
    &'static str,
    crate::fabric::DeterminismClass,
    crate::fabric::ReplayClass,
    Vec<crate::fabric::FabricAuthority>,
);

fn provider_replay_classes(
    provider_kind: MembershipProviderKind,
) -> (crate::fabric::DeterminismClass, crate::fabric::ReplayClass) {
    match provider_kind {
        MembershipProviderKind::DeterministicSimulation => (
            crate::fabric::DeterminismClass::DeterministicWithRecordedInputs,
            crate::fabric::ReplayClass::Recompute,
        ),
        MembershipProviderKind::Static => (
            crate::fabric::DeterminismClass::DeterministicWithRecordedInputs,
            crate::fabric::ReplayClass::Recompute,
        ),
        MembershipProviderKind::PolicyManaged | MembershipProviderKind::ConsistencyBacked => {
            (crate::fabric::DeterminismClass::ExternalEffect, crate::fabric::ReplayClass::RecordedEffectRequired)
        }
    }
}

fn membership_port_definitions(
    provider_determinism: crate::fabric::DeterminismClass,
    provider_replay: crate::fabric::ReplayClass,
) -> [PortDefinition; 4] {
    [
        (
            FABRIC_MEMBERSHIP_PORT_ID,
            crate::fabric::FabricPortClass::Membership,
            vec!["snapshot", "eligible-members", "readback"],
            MEMBERSHIP_VIEW_SCHEMA,
            provider_determinism,
            provider_replay,
            vec![
                crate::fabric::FabricAuthority::Membership,
                crate::fabric::FabricAuthority::Policy,
            ],
        ),
        (
            FABRIC_FAILURE_OBSERVATION_PORT_ID,
            crate::fabric::FabricPortClass::Membership,
            vec!["observe", "reduce", "readback"],
            FAILURE_OBSERVATION_SCHEMA,
            provider_determinism,
            provider_replay,
            vec![
                crate::fabric::FabricAuthority::Membership,
                crate::fabric::FabricAuthority::Time,
            ],
        ),
        (
            FABRIC_PLACEMENT_PORT_ID,
            crate::fabric::FabricPortClass::Placement,
            vec!["plan", "explain", "compare"],
            PLACEMENT_PLAN_SCHEMA,
            crate::fabric::DeterminismClass::Pure,
            crate::fabric::ReplayClass::Recompute,
            vec![
                crate::fabric::FabricAuthority::Placement,
                crate::fabric::FabricAuthority::Policy,
                crate::fabric::FabricAuthority::Resources,
            ],
        ),
        (
            FABRIC_ASSIGNMENT_PORT_ID,
            crate::fabric::FabricPortClass::Placement,
            vec![
                "propose",
                "reserve",
                "assign",
                "acknowledge",
                "activate",
                "drain",
                "replace",
                "release",
            ],
            ROLE_ASSIGNMENT_SCHEMA,
            crate::fabric::DeterminismClass::ExternalEffect,
            crate::fabric::ReplayClass::RecordedEffectRequired,
            vec![
                crate::fabric::FabricAuthority::Placement,
                crate::fabric::FabricAuthority::Supervision,
                crate::fabric::FabricAuthority::DurableState,
            ],
        ),
    ]
}

fn membership_port_ids() -> [&'static str; MEMBERSHIP_PORT_COUNT] {
    [
        FABRIC_MEMBERSHIP_PORT_ID,
        FABRIC_FAILURE_OBSERVATION_PORT_ID,
        FABRIC_PLACEMENT_PORT_ID,
        FABRIC_ASSIGNMENT_PORT_ID,
    ]
}

fn role_requirements_value(requirements: &RoleRequirements) -> preserves::IOValue {
    let required_labels = requirements
        .required_labels
        .iter()
        .map(|constraint| {
            crate::preserves_rail::record("fabric-required-label-v1", vec![
                crate::preserves_rail::string(&constraint.key),
                optional_string(constraint.value.as_deref()),
                crate::preserves_rail::string(constraint.minimum_authority.as_str()),
            ])
        })
        .collect();
    let preferred_labels = requirements
        .preferred_labels
        .iter()
        .map(|preference| {
            crate::preserves_rail::record("fabric-preferred-label-v1", vec![
                crate::preserves_rail::string(&preference.key),
                crate::preserves_rail::string(&preference.value),
                crate::preserves_rail::string(preference.minimum_authority.as_str()),
                crate::preserves_rail::u64_value(u64::from(preference.weight)),
            ])
        })
        .collect();
    crate::preserves_rail::record("fabric-role-requirements-v1", vec![
        field("extension-id", crate::preserves_rail::string(&requirements.extension_id)),
        field("service-id", crate::preserves_rail::string(&requirements.service_id)),
        field("role-kind", crate::preserves_rail::string(&requirements.role_kind)),
        field("replica-count", crate::preserves_rail::u64_value(u64::from(requirements.replica_count))),
        field("per-replica", resource_value(requirements.per_replica)),
        field("required-features", strings_value(requirements.required_features.iter().map(String::as_str))),
        field("required-labels", crate::preserves_rail::sequence(required_labels)),
        field("preferred-labels", crate::preserves_rail::sequence(preferred_labels)),
        field(
            "anti-affinity-label-keys",
            strings_value(requirements.anti_affinity_label_keys.iter().map(String::as_str)),
        ),
        field("distinct-nodes", crate::preserves_rail::bool_value(requirements.distinct_nodes)),
        field("avoid-suspected", crate::preserves_rail::bool_value(requirements.avoid_suspected)),
        field("allow-degraded", crate::preserves_rail::bool_value(requirements.allow_degraded)),
    ])
}
