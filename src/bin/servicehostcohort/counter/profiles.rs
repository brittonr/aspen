pub(super) struct Selected {
    pub time: molten::fabric_time::Profile,
    pub membership: molten::fabric_membership::Profile,
    pub policy: molten::core_api::coordination_delivery::DeliveryPolicy,
    pub descriptors: Vec<molten::fabric::FabricPortDescriptor>,
}

pub(super) fn build(
    context: &super::super::service::Context<'_>,
    operator: &super::super::operator::Authority,
    placement: &molten::addressable_actor::command::placement::Selection,
) -> molten::error::Result<Selected> {
    let source = context.source;
    let time_ref = super::super::source::record(&source.root, "counter-live-time-selection-v1", vec![
        molten::preserves_rail::string(&source.validation.selected_output_validation_ref),
        molten::preserves_rail::string("live-logical-monotonic-128-timers-bounded-v1"),
    ])?;
    let time = molten::fabric_time::admit_profile(&time_descriptor(time_ref))?;
    super::super::source::retain(&source.root, &time.value)?;
    let membership_ref = super::super::source::record(&source.root, "counter-static-membership-selection-v1", vec![
        molten::preserves_rail::string(&placement.node_id),
        molten::preserves_rail::string(&placement.authority_ref),
        molten::preserves_rail::string(&source.validation.selected_output_validation_ref),
    ])?;
    let membership = molten::fabric_membership::profile(&molten::fabric_membership::MembershipSourceProfile {
        schema: molten::fabric_membership::MEMBERSHIP_SOURCE_PROFILE_SCHEMA.to_string(),
        profile_id: "counter-static-membership-v1".to_string(),
        profile_ref: membership_ref,
        provider_kind: molten::fabric_membership::MembershipProviderKind::Static,
        authority_strength: molten::fabric_membership::MembershipAuthorityStrength::OperatorDeclared,
        authority_scope: placement.node_id.clone(),
        max_view_age_ticks: 100,
        non_claims: molten::fabric_membership::REQUIRED_MEMBERSHIP_NON_CLAIMS.to_vec(),
    })?;
    super::super::source::retain(&source.root, &membership.value)?;
    let grant = |ability: &str| {
        operator.grant(
            &source.root,
            &format!("molten://addressable-counter/{}", placement.service_id),
            ability,
            &placement.authority_ref,
        )
    };
    let policy = delivery_policy(grant("complete")?, grant("expire")?, grant("redrive")?, grant("retain")?);
    molten::core_api::coordination_delivery::validate_delivery_policy(&policy)
        .map_err(|issues| super::super::invalid(format!("counter delivery policy denied: {issues:?}")))?;
    let mut descriptors = delivery_descriptors(context);
    descriptors.extend(
        molten::fabric_time::port_descriptors(&time)
            .into_iter()
            .filter(|descriptor| descriptor.port_id == molten::fabric_time::FABRIC_TIMER_PORT_ID),
    );
    descriptors.extend(molten::fabric_membership::port_descriptors(&membership));
    for descriptor in &descriptors {
        let (_, value) = molten::fabric::canonical_port_descriptor(descriptor)?;
        super::super::source::retain(&source.root, &value)?;
    }
    Ok(Selected {
        time,
        membership,
        policy,
        descriptors,
    })
}

pub(super) fn time_descriptor(reference: String) -> molten::fabric_time::TimeProfileDescriptor {
    molten::fabric_time::TimeProfileDescriptor {
        schema: molten::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: "counter-live-time-v1".to_string(),
        profile_ref: reference,
        kind: molten::fabric_time::TimeProfileKind::Live,
        supported_domains: molten::fabric_time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: 1_000_000_000_000,
        max_uncertainty_ticks: 1,
        max_timers: 128,
        max_runnables: 128,
        max_entropy_request_bytes: 1,
        max_entropy_total_bytes: 1,
        max_scheduler_concurrency: 1,
        max_scheduler_queue_depth: 128,
        fairness_bound_turns: None,
        scheduler_policy: molten::fabric_time::SchedulerPolicy {
            ordering: molten::fabric_time::SchedulerOrdering::Fifo,
            replay: molten::fabric_time::SchedulerReplayPolicy::RecordedChoiceRequired,
            overload: molten::fabric_time::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: molten::fabric_time::TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: molten::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

pub(super) fn delivery_policy(
    complete: String,
    expire: String,
    redrive: String,
    retain: String,
) -> molten::core_api::coordination_delivery::DeliveryPolicy {
    molten::core_api::coordination_delivery::DeliveryPolicy {
        schema: molten::core_api::coordination_delivery::DELIVERY_POLICY_SCHEMA.to_string(),
        policy_id: "counter-durable-delivery-v1".to_string(),
        visibility_timeout_ticks: 100,
        maximum_attempts: 2,
        retry_base_delay_ticks: 5,
        retry_maximum_delay_ticks: 5,
        retry_backoff: molten::core_api::coordination_delivery::DeliveryBackoff::Fixed,
        ordering: molten::core_api::coordination_delivery::DeliveryOrdering::StrictFifo,
        dead_letter_queue_id: "counter-dead-letter".to_string(),
        dead_letter_retention_ticks: 100,
        ready_capacity: 128,
        in_flight_capacity: 128,
        retry_capacity: 128,
        dead_letter_capacity: 128,
        metadata_byte_limit: 1024,
        status_item_limit: 128,
        completion_authority_ref: complete,
        expiry_authority_ref: expire,
        redrive_authority_ref: redrive,
        retention_authority_ref: retain,
        retryable_failure_classes: ["transient".to_string()].into_iter().collect(),
        poison_failure_classes: ["poison".to_string()].into_iter().collect(),
        poison_item_handling: molten::core_api::coordination_delivery::PoisonItemHandling::DeadLetter,
        non_claims: molten::core_api::coordination_delivery::required_delivery_non_claims(),
    }
}

fn delivery_descriptors(context: &super::super::service::Context<'_>) -> Vec<molten::fabric::FabricPortDescriptor> {
    let specifications = [
        (
            molten::core_api::coordination_delivery::PORT_CONSISTENCY,
            molten::fabric::FabricPortClass::Consistency,
            molten::fabric::FabricAuthority::Consistency,
            molten::fabric::FabricResource::Concurrency,
        ),
        (
            molten::core_api::coordination_delivery::PORT_DURABLE_STATE,
            molten::fabric::FabricPortClass::DurableState,
            molten::fabric::FabricAuthority::DurableState,
            molten::fabric::FabricResource::StorageBytes,
        ),
        (
            molten::core_api::coordination_delivery::PORT_LOGICAL_TIME,
            molten::fabric::FabricPortClass::Time,
            molten::fabric::FabricAuthority::Time,
            molten::fabric::FabricResource::LogicalTime,
        ),
        (
            molten::core_api::coordination_delivery::PORT_RESOURCE,
            molten::fabric::FabricPortClass::Resources,
            molten::fabric::FabricAuthority::Resources,
            molten::fabric::FabricResource::QueueDepth,
        ),
        (
            molten::core_api::coordination_delivery::PORT_OBSERVABILITY,
            molten::fabric::FabricPortClass::Evidence,
            molten::fabric::FabricAuthority::Evidence,
            molten::fabric::FabricResource::Diagnostics,
        ),
    ];
    specifications
        .into_iter()
        .map(|(port, class, authority, resource)| molten::fabric::FabricPortDescriptor {
            schema: molten::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: port.to_string(),
            version: "v1".to_string(),
            class,
            operation_classes: vec!["coordination-delivery".to_string()],
            input_schema_refs: vec![molten::preserves_rail::content_ref_from_bytes(
                molten::core_api::coordination_delivery::DELIVERY_REQUEST_SCHEMA.as_bytes(),
            )],
            output_schema_refs: vec![molten::preserves_rail::content_ref_from_bytes(
                molten::core_api::coordination_delivery::DELIVERY_STATE_SCHEMA.as_bytes(),
            )],
            authority_requirements: vec![authority],
            resource_requirements: vec![resource],
            determinism: molten::fabric::DeterminismClass::ExternalEffect,
            replay: molten::fabric::ReplayClass::RecordedEffectRequired,
            implementation_profile: "counter-existing-owner-delivery-v1".to_string(),
            conformance_refs: vec![context.source.validation.selected_output_validation_ref.clone()],
            non_claims: molten::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        })
        .collect()
}
