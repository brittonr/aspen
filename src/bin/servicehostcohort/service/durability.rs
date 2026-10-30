pub(super) fn build(
    context: &super::Context<'_>,
    member: &str,
) -> molten::error::Result<(
    molten::fabric_durability::DurableStateProfile,
    molten::fabric_durability::DurableNamespaceDescriptor,
)> {
    let reference = super::super::source::record(&context.source.root, "experimental-live-redb-profile-v1", vec![
        molten::preserves_rail::string("live-redb"),
        molten::preserves_rail::u64_value(4096),
        molten::preserves_rail::u64_value(1_048_576),
        molten::preserves_rail::u64_value(134_217_728),
        molten::preserves_rail::string(&context.source.validation.selected_output_validation_ref),
    ])?;
    let levels = vec![
        molten::fabric_durability::DurabilityLevel::Buffered,
        molten::fabric_durability::DurabilityLevel::ProcessLoss,
        molten::fabric_durability::DurabilityLevel::MachineLoss,
    ];
    let profile = molten::fabric_durability::DurableStateProfile {
        schema: molten::fabric_durability::DURABLE_STATE_PROFILE_SCHEMA.to_string(),
        profile_id: "live-redb".to_string(),
        profile_ref: reference.clone(),
        adapter_kind: molten::fabric_durability::DurableAdapterKind::LiveRedb,
        supported_levels: levels.clone(),
        max_namespaces: 3,
        max_log_records: 4096,
        max_ordered_entries: 256,
        max_operation_bytes: 1_048_576,
        max_namespace_bytes: 134_217_728,
        max_batch_operations: 64,
        max_snapshots: 64,
        max_effect_transactions: 256,
        non_claims: molten::fabric_durability::REQUIRED_DURABILITY_NON_CLAIMS.to_vec(),
    };
    let schema_ref = super::super::source::record(&context.source.root, "native-instance-journal-schema-v3", vec![
        molten::preserves_rail::string(molten::system_extension::NATIVE_INSTANCE_STATE_SCHEMA),
        molten::preserves_rail::string(&context.source.validation.selected_output_validation_ref),
    ])?;
    let namespace = molten::fabric_durability::DurableNamespaceDescriptor {
        schema: molten::fabric_durability::DURABLE_STATE_NAMESPACE_SCHEMA.to_string(),
        profile_ref: reference,
        adapter_id: "experimental-native-journal".to_string(),
        namespace_id: member.to_string(),
        generation: 1,
        value_schema_ref: schema_ref,
        atomicity_domain: molten::fabric_durability::AtomicityDomain {
            domain_id: format!("{member}.journal-domain"),
            adapter_id: "experimental-native-journal".to_string(),
            namespace_id: member.to_string(),
            generation: 1,
            object_classes: vec![
                molten::fabric_durability::DurableObjectClass::LogRecord,
                molten::fabric_durability::DurableObjectClass::Checkpoint,
            ],
            max_operations: 64,
            max_bytes: 1_048_576,
            supported_levels: levels,
        },
        retention_authority_ref: None,
        quota_bytes: 134_217_728,
    };
    molten::fabric_durability::durable_profile(&profile)?;
    Ok((profile, namespace))
}
