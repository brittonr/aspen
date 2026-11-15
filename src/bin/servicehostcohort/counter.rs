mod fixtures;
mod profiles;
#[cfg(test)]
mod tests;

pub(super) const EXTENSION_ID: &str = "molten.addressable-counter";
pub(super) const STATE_SCHEMA: &str = "molten.addressable-counter.state.v1";

pub(super) struct Selection {
    key: molten::core_api::addressable_actor::ActorKey,
    placement: molten::addressable_actor::command::placement::Selection,
}

pub(super) fn selected_key(args: &super::Arguments) -> molten::error::Result<Option<Selection>> {
    let fields = [&args.actor_namespace_ref, &args.actor_type, &args.actor_key];
    if args.profile == super::Profile::NativeFixture {
        if fields.iter().any(|field| field.is_some()) || args.counter_placement.is_some() {
            return Err(super::invalid("actor selection requires explicit --profile counter"));
        }
        return Ok(None);
    }
    let required = |field: &Option<String>| {
        field.clone().ok_or_else(|| super::invalid("counter requires namespace, type and key"))
    };
    let key = molten::core_api::addressable_actor::ActorKey {
        schema: molten::core_api::addressable_actor::ACTOR_KEY_SCHEMA.to_string(),
        namespace_ref: required(&args.actor_namespace_ref)?,
        actor_type: required(&args.actor_type)?,
        key: required(&args.actor_key)?,
    };
    let issues = molten::core_api::addressable_actor::validate_actor_key(&key);
    if !issues.is_empty() {
        return Err(super::invalid(format!("counter actor key denied: {issues:?}")));
    }
    let path = args
        .counter_placement
        .as_ref()
        .ok_or_else(|| super::invalid("counter requires actual --counter-placement"))?;
    let bytes = super::source::read_input(path, 65_536)?;
    let placement: molten::addressable_actor::command::placement::Selection = serde_json::from_slice(&bytes)
        .map_err(|error| super::invalid(format!("invalid counter placement JSON: {error}")))?;
    placement.validate(&molten::core_api::addressable_actor::identify_actor_key(&key))?;
    Ok(Some(Selection { key, placement }))
}

pub(super) fn create(
    source: &super::source::Prepared,
    operator: &mut super::operator::Authority,
    policy_ref: &str,
    elapsed_ref: &str,
    selection: Selection,
) -> molten::error::Result<()> {
    let actor = molten::core_api::addressable_actor::identify_actor_key(&selection.key);
    let delivery_member = super::source::record(&source.root, "counter-delivery-member-selection-v1", vec![
        molten::preserves_rail::string(&actor),
        molten::preserves_rail::string(&source.artifact_ref),
        molten::preserves_rail::string("always-running-existing-coordination-delivery-owner-not-another-actor-key"),
    ])?;
    let context = super::service::Context::new(source, operator, policy_ref)?;
    let mut selected = profiles::build(&context, operator, &selection.placement)?;
    let delivery_policy_ref = molten::core_api::coordination_delivery::identify_delivery_policy(&selected.policy);
    let actor_group = group("molten.counter.actor", &actor, elapsed_ref);
    let delivery_group = group("molten.counter.delivery", &delivery_member, elapsed_ref);
    let actor_descriptors = molten::fabric_membership::port_descriptors(&selected.membership);
    let actor_service =
        super::service::build_counter(&context, &actor, &actor_group, actor_descriptors, &delivery_policy_ref)?;
    let delivery_service = super::service::build_counter(
        &context,
        &delivery_member,
        &delivery_group,
        std::mem::take(&mut selected.descriptors),
        &delivery_policy_ref,
    )?;
    let admitted = admit(&delivery_service)?;
    let delivery_manifest = delivery_manifest(&admitted, &selected)?;
    let profile = actor_profile(
        &context,
        &selection.placement,
        ActorServices {
            actor: &actor_service,
            delivery: &delivery_manifest,
            recovery: &actor_group,
        },
        &selected.time.profile.profile_ref,
    )?;
    let configuration = molten::addressable_actor::native::Configuration {
        actor_key: selection.key,
        profile,
        node_id: selection.placement.node_id.clone(),
        assignment_id: selection.placement.assignment_id.clone(),
        role_id: selection.placement.role_id.clone(),
        queue_id: delivery_member.clone(),
        delivery_member,
    };
    let bundle = molten::node_service_host::codec::Bundle {
        services: vec![actor_service, delivery_service],
        replacements: Vec::new(),
    };
    let bytes = molten::node_service_host::codec::encode(&bundle)?;
    let bundle_ref = molten::preserves_rail::content_ref_from_bytes(&bytes);
    source.root.write_new(&molten::node_state::RelativePath::parse("admission.preserves")?, &bytes)?;
    super::operations::publish_counter(&source.root, &actor_group, &delivery_group)?;
    operator.publish(&source.root, &bundle_ref, &source.validation)?;
    let configuration_ref = fixtures::publish(source, fixtures::Inputs {
        configuration: &configuration,
        placement: &selection.placement,
        selected: &selected,
        delivery: &delivery_manifest,
        bundle_ref: &bundle_ref,
    })?;
    publish_receipt(source, &configuration.profile, &selected, &delivery_manifest, AuthoredReferences {
        bundle: &bundle_ref,
        configuration: &configuration_ref,
    })
}

struct AuthoredReferences<'a> {
    bundle: &'a str,
    configuration: &'a str,
}

fn publish_receipt(
    source: &super::source::Prepared,
    profile: &molten::core_api::addressable_actor::AddressableActorProfile,
    selected: &profiles::Selected,
    delivery: &molten::core_api::coordination_delivery::DeliveryManifest,
    references: AuthoredReferences<'_>,
) -> molten::error::Result<()> {
    let receipt = molten::preserves_rail::record("experimental-addressable-counter-cohort-authored-v1", vec![
        molten::preserves_rail::string(references.bundle),
        molten::preserves_rail::string(references.configuration),
        molten::preserves_rail::string(molten::core_api::addressable_actor::identify_addressable_actor_profile(
            profile,
        )),
        molten::preserves_rail::string(molten::core_api::coordination_delivery::identify_delivery_manifest(delivery)),
        molten::preserves_rail::string(&selected.time.profile_ref),
        molten::preserves_rail::string(&selected.membership.admission_ref),
        molten::preserves_rail::string(&source.validation.selected_output_validation_ref),
        molten::preserves_rail::string(&source.validation.expected_startup_validation_ref),
        molten::preserves_rail::string(&source.callback_ref),
        molten::preserves_rail::string(
            "authorship-only-execution-evidence-missing-no-live-pass-or-production-admission",
        ),
    ]);
    let receipt_bytes = molten::preserves_rail::canonical_bytes(&receipt)?;
    source
        .root
        .write_new(&molten::node_state::RelativePath::parse("authoring-receipt.preserves")?, &receipt_bytes)?;
    super::source::retain(&source.root, &receipt)?;
    println!("{}", molten::preserves_rail::to_text(&receipt)?);
    Ok(())
}

fn group(id: &str, member: &str, elapsed: &str) -> molten::system_extension::RecoveryGroupDeclaration {
    molten::system_extension::RecoveryGroupDeclaration {
        group_id: id.to_string(),
        members: vec![molten::system_extension::RecoveryGroupMember {
            member_id: member.to_string(),
            restart_class: molten::system_extension::RestartClass::Permanent,
            after: Vec::new(),
        }],
        window_ticks: 100,
        backoff_ticks: 5,
        max_restarts_per_member: 4,
        max_restarts_per_group: 3,
        elapsed_source_ref: elapsed.to_string(),
    }
}

struct ActorServices<'a> {
    actor: &'a molten::node_service_host::codec::ExtensionInput,
    delivery: &'a molten::core_api::coordination_delivery::DeliveryManifest,
    recovery: &'a molten::system_extension::RecoveryGroupDeclaration,
}

fn actor_profile(
    context: &super::service::Context<'_>,
    placement: &molten::addressable_actor::command::placement::Selection,
    services: ActorServices<'_>,
    time_ref: &str,
) -> molten::error::Result<molten::core_api::addressable_actor::AddressableActorProfile> {
    let group = services.recovery;
    let supervision = super::source::record(&context.source.root, "counter-recovery-group-selection-v1", vec![
        molten::preserves_rail::string(&group.group_id),
        molten::preserves_rail::string(&group.members[0].member_id),
        molten::preserves_rail::string("permanent"),
        molten::preserves_rail::u64_value(group.window_ticks),
        molten::preserves_rail::u64_value(group.backoff_ticks),
        molten::preserves_rail::u64_value(group.max_restarts_per_member),
        molten::preserves_rail::u64_value(group.max_restarts_per_group),
        molten::preserves_rail::string(&group.elapsed_source_ref),
    ])?;
    let profile = molten::core_api::addressable_actor::AddressableActorProfile {
        schema: molten::core_api::addressable_actor::ACTOR_PROFILE_SCHEMA.to_string(),
        profile_id: "addressable-counter-v1".to_string(),
        profile_version: molten::core_api::addressable_actor::ADDRESSABLE_ACTOR_PROFILE_VERSION,
        reference_source: molten::core_api::addressable_actor::ActorReferenceSource {
            repository: molten::core_api::addressable_actor::RIVET_ACTORS_REPOSITORY.to_string(),
            revision: molten::core_api::addressable_actor::RIVET_ACTORS_REVISION.to_string(),
            license: molten::core_api::addressable_actor::RIVET_ACTORS_LICENSE.to_string(),
            selected_concepts: [
                "keyed-addressability",
                "generation-fenced-runtime",
                "sleep-intent-and-rewake-separation",
                "persisted-state-and-scheduled-events",
                "runtime-and-durable-survival-separation",
            ]
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        },
        system_extension_profile_ref: services.actor.host_profile.profile_ref.clone(),
        placement_profile_ref: placement.fencing_profile_ref.clone(),
        delivery_profile_ref: molten::core_api::coordination_delivery::identify_delivery_manifest(services.delivery),
        durable_state_profile_ref: services.actor.durability_profile.profile_ref.clone(),
        time_profile_ref: time_ref.to_string(),
        resource_profile_ref: context.profiles.resource_ref.clone(),
        supervision_profile_ref: supervision,
        authority_profile_ref: context.grant_ref.clone(),
        evidence_profile_ref: context.evidence_ref.clone(),
        idle_after_ticks: 5,
        maximum_drain_items: 16,
        survival: molten::core_api::addressable_actor::standard_actor_survival_matrix(),
        non_claims: molten::core_api::addressable_actor::required_addressable_actor_non_claims(),
    };
    let issues = molten::core_api::addressable_actor::validate_addressable_actor_profile(&profile);
    if !issues.is_empty() {
        return Err(super::invalid(format!("counter actor profile denied: {issues:?}")));
    }
    Ok(profile)
}

fn admit(
    service: &molten::node_service_host::codec::ExtensionInput,
) -> molten::error::Result<molten::system_extension::AdmittedManifest> {
    let tier = molten::fabric::canonical_extension_tier_admission(&service.tier)?;
    molten::system_extension::admit_manifest(&service.manifest, &service.descriptors, &tier, &[
        molten::system_extension::ExecutionProfile::NativeProcess,
    ])
}

fn delivery_manifest(
    admitted: &molten::system_extension::AdmittedManifest,
    selected: &profiles::Selected,
) -> molten::error::Result<molten::core_api::coordination_delivery::DeliveryManifest> {
    let bindings = molten::core_api::coordination_delivery::REQUIRED_DELIVERY_PORTS
        .into_iter()
        .map(|port| {
            let key = molten::fabric::FabricPortKey {
                port_id: port.to_string(),
                version: "v1".to_string(),
            };
            let binding = admitted
                .binding_for(&key)
                .ok_or_else(|| super::invalid("counter delivery lacks exact host binding"))?;
            Ok((port.to_string(), binding.binding_ref.clone()))
        })
        .collect::<molten::error::Result<_>>()?;
    let manifest = molten::core_api::coordination_delivery::DeliveryManifest {
        schema: molten::core_api::coordination_delivery::DELIVERY_MANIFEST_SCHEMA.to_string(),
        extension_id: admitted.manifest().extension_id.clone(),
        service_id: admitted.manifest().service_id.clone(),
        service_generation: 1,
        implementation_ref: admitted.manifest().implementation_ref.clone(),
        time_profile_ref: selected.time.profile.profile_ref.clone(),
        policy_ref: molten::core_api::coordination_delivery::identify_delivery_policy(&selected.policy),
        port_bindings: bindings,
        non_claims: molten::core_api::coordination_delivery::required_delivery_non_claims(),
    };
    molten::core_api::coordination_delivery::validate_delivery_manifest(&manifest, &selected.policy)
        .map_err(|issues| super::invalid(format!("counter delivery manifest denied: {issues:?}")))?;
    Ok(manifest)
}

pub(super) fn requirement(descriptor: &molten::fabric::FabricPortDescriptor) -> molten::fabric::FabricPortRequirement {
    molten::fabric::FabricPortRequirement {
        port_id: descriptor.port_id.clone(),
        version: descriptor.version.clone(),
        class: descriptor.class,
        operation_classes: descriptor.operation_classes.clone(),
        input_schema_refs: descriptor.input_schema_refs.clone(),
        output_schema_refs: descriptor.output_schema_refs.clone(),
        allowed_authorities: descriptor.authority_requirements.clone(),
        available_resources: descriptor.resource_requirements.clone(),
        expected_determinism: descriptor.determinism,
        expected_replay: descriptor.replay,
        expected_profile: descriptor.implementation_profile.clone(),
    }
}
