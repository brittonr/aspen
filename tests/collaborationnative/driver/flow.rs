//! Status, submission and delivery through the public service-host operations.

/// Bootstrap authenticates its real load; hosted operations preview only the retained owner.
pub fn authorize(
    host: &super::process::Host<'_>,
    fixture: &crate::producer::Node,
    operation: &preserves::IOValue,
    tick: u64,
) -> crate::producer::Result<molten::node_service_hosting::OperationRequest> {
    fixture.root.write(
        &molten::node_state::RelativePath::parse("operation.preserves")?,
        &molten::preserves_rail::canonical_bytes(operation)?,
    )?;
    if host.control.is_some() {
        let (success, output) = super::process::invoke(host, command(operation), tick, Some("--preview"))?;
        if !success {
            return Err(format!("retained operation preview denied at tick {tick}: {output}").into());
        }
        let exact = request(value(&output, "node-service-host-operation-request-v1")?)?;
        super::process::sign(host, fixture, &exact, tick, 1)?;
        return Ok(exact);
    }
    let (success, output) = super::process::invoke(host, command(operation), tick, Some("--preview-load"))?;
    if !success {
        return Err(format!("genuine source/bundle load preview denied at tick {tick}: {output}").into());
    }
    let load = request(value(&output, "node-service-host-operation-request-v1")?)?;
    super::process::sign(host, fixture, &load, tick, 0)?;
    let (success, output) = super::process::invoke(host, command(operation), tick, Some("--preview"))?;
    if !success {
        return Err(format!("load-authorized operation preview denied at tick {tick}: {output}").into());
    }
    let exact = request(value(&output, "node-service-host-operation-request-v1")?)?;
    super::process::sign(host, fixture, &exact, tick, 1)?;
    super::process::sign(host, fixture, &load, tick, 2)?;
    Ok(exact)
}

fn command(operation: &preserves::IOValue) -> &'static str {
    let text = molten::preserves_rail::to_text(operation).unwrap_or_default();
    for name in [
        "install",
        "status",
        "supervise",
        "submit",
        "readmit",
        "deliver",
        "upgrade",
        "reconcile",
        "scope-observe",
    ] {
        if text.contains(&format!("node-service-host-{name}-v1")) {
            return name;
        }
    }
    "status"
}

pub fn value<'a>(text: &'a str, tag: &str) -> crate::producer::Result<&'a str> {
    text.lines()
        .find(|line| line.trim_start().starts_with(&format!("<{tag}")))
        .ok_or_else(|| format!("missing {tag} in node output").into())
}

pub fn request(value: &str) -> crate::producer::Result<molten::node_service_hosting::OperationRequest> {
    let parsed = molten::preserves_rail::parse_text(value)?;
    let fields = molten::preserves_rail::simple_record_fields(&parsed, "node-service-host-operation-request-v1", 5)?;
    Ok(molten::node_service_hosting::OperationRequest {
        operation: super::operations::wire::text(&fields[0])?,
        subject: super::operations::wire::text(&fields[1])?,
        logical_tick: super::operations::wire::number(&fields[2])?,
        state_ref: super::operations::wire::text(&fields[3])?,
        input_ref: super::operations::wire::text(&fields[4])?,
    })
}

/// Claim durable work for the current instance; delivery is refused without the current claim.
pub fn readmit(
    node: &super::Node,
    fixture: &crate::producer::Node,
    work: &super::operations::Work,
    tick: u64,
) -> crate::producer::Result<String> {
    let operation = super::operations::wire::operation("readmit", vec![
        molten::preserves_rail::string(crate::producer::SERVICE),
        molten::preserves_rail::string(&work.presentation.work_ref),
    ]);
    authorize(&super::process::host_of(node), fixture, &operation, tick)?;
    let (success, output) = super::process::invoke(&super::process::host_of(node), "readmit", tick, None)?;
    if !success {
        return Err(format!("work readmission denied at tick {tick}: {output}").into());
    }
    // Both surfaces return the actual receipt; the claim is its value field.
    let receipt = molten::preserves_rail::parse_text(value(&output, "node-service-host-receipt-v1")?)?;
    let fields = molten::preserves_rail::simple_record_fields(&receipt, "node-service-host-receipt-v1", 5)?;
    let claim = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let claim = molten::preserves_rail::simple_record_fields(&claim, "node-service-host-claim-v1", 4)?;
    Ok(molten::preserves_rail::required_string_field(&claim[0], "claim ref")?)
}

pub fn submit(
    node: &super::Node,
    fixture: &crate::producer::Node,
    member: &super::operations::Member,
    intent: &super::operations::Intent,
    tick: u64,
) -> crate::producer::Result<super::operations::Work> {
    let payload = molten::system_extension::NativeCallbackValue {
        value_ref: intent.prepared.effect_ref.clone(),
        bytes: intent.prepared.staged_bytes.clone(),
    };
    let work_ref =
        molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("collaboration-native-work-v1", vec![
            molten::preserves_rail::string(crate::producer::SERVICE),
            molten::preserves_rail::string(&payload.value_ref),
        ]))?;
    let work = molten::system_extension::WorkPresentation {
        work_ref,
        class: molten::system_extension::WorkClass::Durable,
        generation: member.generation,
        incarnation: member.incarnation,
    };
    let operation = super::operations::wire::operation("submit", vec![
        molten::preserves_rail::string(crate::producer::SERVICE),
        super::operations::wire::work(&work),
        super::operations::wire::payload(&payload),
    ]);
    authorize(&super::process::host_of(node), fixture, &operation, tick)?;
    let (success, output) = super::process::invoke(&super::process::host_of(node), "submit", tick, None)?;
    if !success {
        return Err(format!("work submission denied at tick {tick}: {output}").into());
    }
    Ok(super::operations::Work {
        presentation: work,
        payload,
        actor: usize::try_from(node.index).map_err(|_| "cohort position exceeds usize")?,
        submit_tick: tick,
    })
}

/// Everything one delivery needs; the public signature stays small.
pub struct Delivery<'a> {
    pub node: &'a super::Node,
    pub fixture: &'a crate::producer::Node,
    pub member: &'a super::operations::Member,
    pub work: &'a super::operations::Work,
    pub intent: &'a super::operations::Intent,
    pub claim_ref: &'a str,
    pub tick: u64,
}

/// Deliver the staged command as durable work with exact route and scope authority.
pub fn deliver(input: &Delivery<'_>) -> crate::producer::Result<(bool, String)> {
    prepare_delivery(input)?;
    execute_delivery(input)
}

/// Prepare real independent grants before a case introduces its owned boundary fault.
pub fn prepare_delivery(
    input: &Delivery<'_>,
) -> crate::producer::Result<molten::node_service_hosting::OperationRequest> {
    let Delivery {
        node,
        fixture,
        member,
        work,
        intent,
        claim_ref,
        tick,
    } = *input;
    let service = fixture.bundle.services.first().ok_or("bundle service")?;
    let manifest = service.admit_manifest()?;
    let key = molten::fabric::FabricPortKey {
        port_id: molten::collaboration_scope::COMMIT_PORT_ID.to_string(),
        version: molten::collaboration_scope::COMMIT_PORT_VERSION.to_string(),
    };
    let actual = manifest.binding_for(&key).ok_or("admitted commit port binding")?;
    let effect = molten::system_extension::TypedEffectRequest {
        target: molten::system_extension::EffectTarget::FabricPort(key),
        operation: molten::collaboration_scope::COMMIT_OPERATION.to_string(),
        input_schema_ref: molten::collaboration_scope::COMMIT_INPUT_SCHEMA.to_string(),
        output_schema_ref: molten::collaboration_scope::COMMIT_OUTPUT_SCHEMA.to_string(),
        request_ref: work.payload.value_ref.clone(),
        generation: member.generation,
        incarnation: member.incarnation,
        accounted_bytes: work.payload.bytes.len() as u64,
    };
    let binding = fixture.effects.bindings.first().ok_or("effect configuration binding")?;
    let route = crate::authority::host::route_effect_request(
        &fixture.bundle_ref,
        &fixture.effects,
        molten::node_service_host::effects::authority::RouteInput {
            binding,
            actual_binding: actual,
            effect: &effect,
        },
        tick,
    )?;
    super::process::sign(&super::process::host_of(node), fixture, &route, tick, 3)?;
    let operation = super::operations::wire::operation("deliver", vec![
        molten::preserves_rail::string(crate::producer::SERVICE),
        super::operations::wire::work(&work.presentation),
        delivery(fixture, member, work, claim_ref, tick)?,
    ]);
    // Trust is frozen at the final observation; delivery is attempted exactly once.
    super::scope::publish(node, fixture, intent, &work.payload.value_ref)?;
    authorize(&super::process::host_of(node), fixture, &operation, tick)
}

/// Execute precisely once; preparation is never repeated after a possibly performed effect.
pub fn execute_delivery(input: &Delivery<'_>) -> crate::producer::Result<(bool, String)> {
    super::process::invoke(&super::process::host_of(input.node), "deliver", input.tick, None)
}

fn delivery(
    fixture: &crate::producer::Node,
    member: &super::operations::Member,
    work: &super::operations::Work,
    claim_ref: &str,
    tick: u64,
) -> crate::producer::Result<preserves::IOValue> {
    let service = fixture.bundle.services.first().ok_or("bundle service")?;
    let request_ref = molten::preserves_rail::canonical_hash(&molten::preserves_rail::record(
        "collaboration-native-delivery-v1",
        vec![
            molten::preserves_rail::string(&work.presentation.work_ref),
            molten::preserves_rail::u64_value(tick),
        ],
    ))?;
    let ingress = molten::preserves_rail::record("node-service-host-ingress-v1", vec![
        molten::preserves_rail::string(&request_ref),
        molten::preserves_rail::string(crate::producer::reference("collaboration-native-endpoint")),
        molten::preserves_rail::string(crate::producer::reference("collaboration-native-peer")),
        molten::preserves_rail::string(crate::producer::SERVICE),
        molten::preserves_rail::string(manifest_ref(service)?),
        molten::preserves_rail::u64_value(member.generation),
        molten::preserves_rail::string(&service.executable.authority_ref),
        molten::preserves_rail::string(&service.executable.policy_ref),
        molten::preserves_rail::string(&service.executable.resource_ref),
        molten::preserves_rail::string(&service.host_profile.transport_profile_ref),
        super::operations::wire::payload(&work.payload),
        molten::preserves_rail::u64_value(work.payload.bytes.len() as u64),
        molten::preserves_rail::string("materialized-native-v3"),
    ]);
    Ok(molten::preserves_rail::record("node-service-host-delivery-v1", vec![
        molten::preserves_rail::sequence(vec![molten::preserves_rail::string(claim_ref)]),
        ingress,
    ]))
}

fn manifest_ref(service: &molten::node_service_host::codec::ExtensionInput) -> crate::producer::Result<String> {
    Ok(service.admit_manifest()?.manifest_ref().to_string())
}
