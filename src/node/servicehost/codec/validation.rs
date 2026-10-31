pub(super) fn bundle(bundle: &super::Bundle) -> crate::error::Result<()> {
    if bundle.services.is_empty()
        || bundle.services.len() > super::MAX_SERVICES
        || bundle.replacements.len() > super::MAX_SERVICES
    {
        return Err(super::wire::error("bundle requires 1..=64 services and at most 64 replacements"));
    }
    let mut services = std::collections::BTreeSet::new();
    let mut storage = std::collections::BTreeSet::new();
    let mut caches = std::collections::BTreeSet::new();
    for input in &bundle.services {
        if !services.insert(&input.manifest.service_id) {
            return Err(super::wire::error("duplicate service identity"));
        }
        if !storage.insert(&input.storage_namespace) || !caches.insert(&input.cache_namespace) {
            return Err(super::wire::error("distinct services must not alias storage or cache namespaces"));
        }
        extension(input)?;
        let instance_id = crate::system_extension::native_identity_ref(&[
            "native-instance-v2",
            &input.manifest.extension_id,
            &input.manifest.service_id,
            &input.executable.manifest_ref,
        ]);
        if input.context.instance_id != instance_id {
            return Err(super::wire::error("base service context must match native initial instance identity"));
        }
    }
    let mut replacements = std::collections::BTreeSet::new();
    for replacement in &bundle.replacements {
        crate::preserves_rail::ContentRef::parse(&replacement.previous_manifest_ref)?;
        if !replacements.insert(&replacement.previous_manifest_ref) {
            return Err(super::wire::error("duplicate exact replacement admission"));
        }
        extension(&replacement.service)?;
    }
    replacement_chains(bundle)?;
    profile_capacity(
        bundle
            .services
            .iter()
            .chain(bundle.replacements.iter().map(|replacement| &replacement.service))
            .map(|input| (&input.host_profile, input.manifest.service_id.as_str())),
    )?;
    Ok(())
}

fn replacement_chains(bundle: &super::Bundle) -> crate::error::Result<()> {
    // Every known manifest is a base service or a replacement, each bounded by `MAX_SERVICES`.
    let known_limit = super::MAX_SERVICES.saturating_mul(2);
    let mut known = std::collections::BTreeMap::new();
    let mut identities = std::collections::BTreeSet::new();
    for service in &bundle.services {
        if known.len() >= known_limit {
            return Err(super::wire::error("replacement ancestry exceeds bundle bound"));
        }
        identities.insert(&service.executable.manifest_ref);
        known.insert(service.executable.manifest_ref.as_str(), service);
    }
    for replacement in &bundle.replacements {
        // A same-manifest generation upgrade adds no ancestry edge. It still has
        // to resolve to an existing root and preserve that root's incarnation.
        if replacement.previous_manifest_ref == replacement.service.executable.manifest_ref {
            continue;
        }
        if !identities.insert(&replacement.service.executable.manifest_ref) {
            return Err(super::wire::error("duplicate or cyclic replacement manifest identity"));
        }
    }
    let mut pending = bundle.replacements.iter().collect::<Vec<_>>();
    while !pending.is_empty() {
        let previous = pending.len();
        let mut next = Vec::with_capacity(previous);
        for replacement in pending {
            if let Some(original) = known.get(replacement.previous_manifest_ref.as_str()).copied() {
                same_incarnation(original, &replacement.service)?;
                if known.len() >= known_limit {
                    return Err(super::wire::error("replacement ancestry exceeds bundle bound"));
                }
                known.insert(replacement.service.executable.manifest_ref.as_str(), original);
            } else {
                next.push(replacement);
            }
        }
        if next.len() == previous {
            return Err(super::wire::error("replacement ancestry is unknown or cyclic"));
        }
        pending = next;
    }
    Ok(())
}

fn profile_capacity<'a>(
    inputs: impl Iterator<Item = (&'a crate::system_extension::NativeHostProfile, &'a str)>,
) -> crate::error::Result<()> {
    let mut profiles = std::collections::BTreeMap::new();
    for (profile, member) in inputs {
        let (known, members) = profiles
            .entry(profile.profile_ref.as_str())
            .or_insert_with(|| (profile, std::collections::BTreeSet::new()));
        if *known != profile {
            return Err(super::wire::error("one native profile reference names conflicting profiles"));
        }
        members.insert(member);
        if members.len() > profile.max_instances {
            return Err(super::wire::error("aggregate hosted native profile instance capacity exceeded"));
        }
    }
    Ok(())
}

fn same_incarnation(original: &super::ExtensionInput, next: &super::ExtensionInput) -> crate::error::Result<()> {
    if original.manifest.service_id != next.manifest.service_id
        || original.manifest.extension_id != next.manifest.extension_id
        || original.context.instance_id != next.context.instance_id
        || original.storage_namespace != next.storage_namespace
        || original.cache_namespace != next.cache_namespace
    {
        return Err(super::wire::error("replacement must preserve original service, instance and durable roots"));
    }
    Ok(())
}

fn extension(input: &super::ExtensionInput) -> crate::error::Result<()> {
    relative(&input.artifact_path)?;
    relative(&input.storage_namespace)?;
    relative(&input.cache_namespace)?;
    absolute(&input.resolved.executable_path)?;
    absolute(&input.resolved.workspace_path)?;
    let admitted = input.admit_manifest()?;
    if !admitted.manifest().declares_callback(crate::system_extension::CallbackKind::Checkpoint)
        || !admitted.manifest().declares_callback(crate::system_extension::CallbackKind::Recover)
    {
        return Err(super::wire::error("durable node hosting requires admitted checkpoint and recover callbacks"));
    }
    let manifest_ref = admitted.manifest_ref();
    if input.executable.manifest_ref != manifest_ref || input.context.manifest_ref != manifest_ref {
        return Err(super::wire::error("native template manifest reference mismatch"));
    }
    if input.manifest.service_id != input.request.service_id
        || input.manifest.extension_id != input.request.extension_id
        || input.context.service_id != input.manifest.service_id
        || input.context.extension_id != input.manifest.extension_id
        || input.context.executable_ref != input.executable.executable_ref
        || input.resolved.executable_artifact_ref != input.request.executable_artifact_ref
        || input.resolved.executable_identity_ref != input.request.executable_identity_ref
        || input.resolved.workspace_ref != input.request.workspace_ref
        || input.resolved.stdin_ref != input.request.stdin_ref
    {
        return Err(super::wire::error("native execution template identity mismatch"));
    }
    let profile = crate::fabric_execution::admit_profile(&input.execution_profile)?;
    crate::fabric_execution::admit_request(
        &profile,
        &input.request,
        &input.authority,
        input.resources,
        input.request.generation,
    )?;
    Ok(())
}

fn relative(value: &str) -> crate::error::Result<()> {
    molten_node_host::node_state::RelativePath::parse(value)
        .map_err(|error| super::wire::error(format!("invalid state-root relative path: {error}")))?;
    Ok(())
}

fn absolute(value: &std::path::Path) -> crate::error::Result<()> {
    if !value.is_absolute()
        || value
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir | std::path::Component::CurDir))
    {
        return Err(super::wire::error("resolved template path must be absolute and traversal-free"));
    }
    Ok(())
}

impl super::ExtensionInput {
    /// Reconstructs genuine canonical admission from the tier and actual port registry.
    pub fn admit_manifest(&self) -> crate::error::Result<crate::system_extension::AdmittedManifest> {
        let tier = crate::fabric::canonical_extension_tier_admission(&self.tier)?;
        crate::system_extension::admit_manifest(&self.manifest, &self.descriptors, &tier, &[
            crate::system_extension::ExecutionProfile::NativeProcess,
        ])
    }
}

#[cfg(test)]
mod capacity_tests {
    #[test]
    fn reference_count_bounds_encode_as_numbers() {
        // `max_policy_refs` bounds a reference collection; the count itself is not a reference.
        let profile = profile();
        let encoded = <crate::system_extension::NativeHostProfile as super::super::wire::Wire>::encode(&profile)
            .expect("numeric reference bounds encode");
        let decoded = <crate::system_extension::NativeHostProfile as super::super::wire::Wire>::decode(&encoded)
            .expect("numeric reference bounds decode");
        assert_eq!(decoded.max_policy_refs, profile.max_policy_refs);
        assert!(super::super::wire::field(&crate::preserves_rail::u64_value(16), "max_policy_refs").is_ok());
        assert!(super::super::wire::field(&crate::preserves_rail::u64_value(16), "policy_refs").is_err());
    }

    #[test]
    fn replacement_alternatives_share_member_slot_but_not_another_members_instances() {
        let profile = profile();
        assert!(super::profile_capacity([(&profile, "member-a"), (&profile, "member-a")].into_iter()).is_ok());
        assert!(super::profile_capacity([(&profile, "member-a"), (&profile, "member-b")].into_iter()).is_err());
        let mut conflicting = profile.clone();
        conflicting.max_instances = 2;
        assert!(super::profile_capacity([(&profile, "member-a"), (&conflicting, "member-b")].into_iter()).is_err());
    }

    fn profile() -> crate::system_extension::NativeHostProfile {
        let reference = crate::preserves_rail::content_ref_from_bytes(b"bounded-capacity-test-profile");
        crate::system_extension::NativeHostProfile {
            schema: crate::system_extension::NATIVE_HOST_PROFILE_SCHEMA.to_string(),
            profile_id: "capacity-test".to_string(),
            profile_ref: reference.clone(),
            execution_profile_ref: reference.clone(),
            transport_profile_ref: reference,
            alpn: crate::system_extension::NATIVE_ALPN.to_string(),
            framing: crate::system_extension::NATIVE_FRAMING.to_string(),
            max_callback_input_bytes: 4096,
            max_callback_output_bytes: 4096,
            max_diagnostic_bytes: 4096,
            max_materialized_value_bytes: 4096,
            max_instances: 1,
            max_unresolved_operations: 16,
            max_port_bindings: 16,
            max_policy_refs: 16,
            max_materialized_values: 16,
            is_local_live_pilot: true,
            requires_materialized_values: true,
            non_claims: crate::system_extension::REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
        }
    }
}
