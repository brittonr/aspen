fn validate_observed_storage_denial(
    receipt: &ParsedFaultReceipt,
    descriptors: &FaultDescriptors<'_>,
    topology: &ParsedTopology,
    input: &NixosVmFaultEvidenceInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    let Some((descriptor, _)) = descriptors.values.iter().zip(descriptors.refs)
        .find(|(_, reference)| *reference == &receipt.descriptor_ref)
    else {
        push_if(diagnostics, true, "vm-fault-observed-denial-missing-descriptor")?;
        return Ok(());
    };
    push_if(diagnostics, descriptor.fault_kind != "permission-denied-state-root"
        || descriptor.command_profile != "node-status-health-write"
        || descriptor.expected_outcome != "deny-before-side-effects"
        || receipt.decision != "deny" || receipt.host_support != "supported",
        "vm-fault-observed-denial-wrong-operation")?;
    let (Some(attempt_value), Some(pre_value), Some(post_value)) = (
        input.attempt_value, input.pre_observation_value, input.post_observation_value,
    ) else {
        push_if(diagnostics, true, "vm-fault-observed-denial-missing-operation-observations")?;
        return Ok(());
    };
    let (Ok(attempt), Ok(pre), Ok(post)) = (
        parse_state_attempt(attempt_value),
        parse_state_observation(pre_value),
        parse_state_observation(post_value),
    ) else {
        push_if(diagnostics, true, "vm-fault-observed-denial-invalid-operation-observations")?;
        return Ok(());
    };
    let attempt_ref = crate::preserves_rail::canonical_hash(attempt_value)?;
    let pre_ref = crate::preserves_rail::canonical_hash(pre_value)?;
    let post_ref = crate::preserves_rail::canonical_hash(post_value)?;
    push_if(diagnostics,
        receipt.child_refs.as_slice() != std::slice::from_ref(&attempt_ref)
            || receipt.pre_fault_refs.as_slice() != std::slice::from_ref(&pre_ref)
            || receipt.injection_refs.as_slice() != std::slice::from_ref(&pre_ref)
            || receipt.post_fault_refs.as_slice() != std::slice::from_ref(&post_ref),
        "vm-fault-observed-denial-artifact-ref-mismatch")?;
    push_if(diagnostics,
        pre.phase != "pre" || post.phase != "post"
            || pre.target_node != descriptor.target_node || post.target_node != descriptor.target_node
            || !topology.nodes.contains(&descriptor.target_node)
            || topology.package_ref != format!("store:{}", topology.package_path)
            || pre.package_path != topology.package_path || post.package_path != topology.package_path
            || attempt.executable_path != format!("{}/bin/molten", topology.package_path)
            || pre.executable_ref != post.executable_ref || attempt.executable_ref != pre.executable_ref
            || pre.startup_ref != post.startup_ref || attempt.startup_ref != pre.startup_ref
            || pre.config_ref != post.config_ref || attempt.config_ref != pre.config_ref
            || pre.binding_ref != post.binding_ref || attempt.binding_ref != pre.binding_ref
            || pre.state_root != post.state_root || attempt.state_root != pre.state_root
            || pre.target != "health-receipt.preserves" || post.target != pre.target || attempt.target != pre.target,
        "vm-fault-observed-denial-binding-mismatch")?;
    const DAC_OVERRIDE: u64 = 1 << 1;
    const DAC_READ_SEARCH: u64 = 1 << 2;
    push_if(diagnostics,
        attempt.errno != 13 || attempt.uid == 0 || attempt.groups.len() > MAX_VM_EFFECTIVE_GROUPS
            || attempt.cap_eff & (DAC_OVERRIDE | DAC_READ_SEARCH) != 0
            || pre.root_uid != attempt.uid || pre.root_gid != attempt.gid
            || pre.root_mode & 0o222 != 0 || pre.root_mode != post.root_mode
            || pre.root_uid != post.root_uid || pre.root_gid != post.root_gid,
        "vm-fault-observed-denial-permissions-not-denied")?;
    push_if(diagnostics,
        !pre.target_absent || !post.target_absent || pre.tree_ref != post.tree_ref,
        "vm-fault-observed-denial-protected-state-changed")?;
    Ok(())
}

