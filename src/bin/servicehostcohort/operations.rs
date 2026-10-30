pub(super) fn publish(
    root: &molten::node_state::DirectoryRoot,
    bundle: &molten::node_service_host::codec::Bundle,
) -> molten::error::Result<()> {
    let main = bundle
        .services
        .first()
        .and_then(|service| service.manifest.recovery_group.as_ref())
        .ok_or_else(|| super::invalid("authored cohort has no primary recovery group"))?;
    let sibling = bundle
        .services
        .last()
        .and_then(|service| service.manifest.recovery_group.as_ref())
        .ok_or_else(|| super::invalid("authored cohort has no sibling recovery group"))?;
    write(root, "install-main.preserves", "install", vec![molten::preserves_rail::string(&main.group_id)])?;
    write(root, "install-sibling.preserves", "install", vec![molten::preserves_rail::string(
        &sibling.group_id,
    )])?;
    write(root, "status.preserves", "status", Vec::new())?;
    write(root, "supervise.preserves", "supervise", vec![
        molten::preserves_rail::string(&main.group_id),
        molten::preserves_rail::string(&main.members[0].member_id),
        molten::preserves_rail::bool_value(false),
        molten::preserves_rail::string("retryable"),
    ])?;
    let replacement = bundle
        .replacements
        .first()
        .ok_or_else(|| super::invalid("authored cohort has no exact successor"))?;
    let replacement_ref =
        molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("host-native-replacement-v1", vec![
            molten::preserves_rail::string(&replacement.service.executable.manifest_ref),
            molten::preserves_rail::string(&replacement.service.executable.executable_ref),
            molten::preserves_rail::string(&replacement.service.host_profile.profile_ref),
        ]))?;
    write(root, "upgrade.preserves", "upgrade", vec![
        molten::preserves_rail::string(&replacement.service.manifest.service_id),
        molten::preserves_rail::string(replacement_ref),
    ])?;
    root.write_new(
        &molten::node_state::RelativePath::parse("startup-selector.preserves")?,
        &molten::preserves_rail::canonical_bytes(&molten::preserves_rail::record(
            "node-service-host-startup-v1",
            vec![
                molten::preserves_rail::string("admission.preserves"),
                molten::preserves_rail::u64_value(1),
            ],
        ))?,
    )
}

fn write(
    root: &molten::node_state::DirectoryRoot,
    path: &str,
    operation: &str,
    fields: Vec<preserves::IOValue>,
) -> molten::error::Result<()> {
    root.write_new(
        &molten::node_state::RelativePath::parse(path)?,
        &molten::preserves_rail::canonical_bytes(&preserves::IOValue::record(
            preserves::IOValue::symbol(format!("node-service-host-{operation}-v1")),
            fields,
        ))?,
    )
}
