//! A genuine native Request without a selected effect runtime must remain unrouted.

pub(super) fn run(cohort: &crate::producer::Cohort) -> crate::producer::Result<preserves::IOValue> {
    let fixture = &cohort.nodes[0];
    let initial_absence = absent(fixture)?;
    let (mut session, metadata_actor, started_absence) = prepare(cohort)?;
    let (delivered, live, delivered_absence) = deliver(cohort, &mut session)?;
    session.require_live()?;
    session.finish()?;
    let closed_absence = absent(fixture)?;
    let before = crate::driver::scenarios::admission::readback::native(fixture)?;
    let bridge = require_request(fixture, &session, &live, &before)?;
    let reopened_output =
        crate::driver::process::reopened_status(&session.nodes[0], fixture, session.work.submit_tick + 4)?;
    let reopened = crate::driver::scenarios::admission::readback::receipt(&reopened_output)?;
    crate::driver::scenarios::admission::readback::require_unknown(&reopened, &session.work.presentation.work_ref)?;
    let reopened_absence = absent(fixture)?;
    let after = crate::driver::scenarios::admission::readback::native(fixture)?;
    crate::driver::scenarios::admission::readback::unchanged(&before, &after)?;
    let reopened_bridge = require_request(fixture, &session, &reopened, &after)?;
    if bridge != reopened_bridge {
        return Err("source-admitted reopen changed the unconfigured native bridge state".into());
    }
    let ready = readiness(cohort, &session)?;
    Ok(molten::preserves_rail::record("collaboration-native-unconfigured-observation-v1", vec![
        molten::preserves_rail::string(cohort.signed_main.to_str().ok_or("source executable encoding")?),
        molten::preserves_rail::sequence(ready),
        molten::preserves_rail::record("historical-peer-scope-input-v1", vec![
            molten::preserves_rail::u64_value(session.nodes[metadata_actor].index),
            session.intent.observation.value(),
        ]),
        molten::preserves_rail::u64_value(session.nodes[0].index),
        session.native_status,
        molten::preserves_rail::string(&session.work.presentation.work_ref),
        molten::preserves_rail::string(session.intent.prepared.request.reference.as_str()),
        delivered,
        live,
        bridge,
        reopened,
        reopened_bridge,
        molten::system_extension::canonical_instance_record(&before)?.value,
        molten::system_extension::canonical_instance_record(&after)?.value,
        molten::preserves_rail::sequence(vec![
            initial_absence,
            started_absence,
            delivered_absence,
            closed_absence,
            reopened_absence,
        ]),
        molten::preserves_rail::string(
            "target-0-explicitly-unconfigured-peer-observation-is-historical-input-not-target-currentness-or-authority; no-scope-provider-or-protected-completion-observed; bounded-experimental-observations-not-production-or-global-exactly-once-proof",
        ),
    ]))
}

fn prepare(
    cohort: &crate::producer::Cohort,
) -> crate::producer::Result<(crate::driver::scenarios::Session, usize, preserves::IOValue)> {
    let fixture = &cohort.nodes[0];
    let nodes = crate::driver::scenarios::start_nodes(cohort, Some(0))?;
    let started_absence = absent(fixture)?;
    // These peers own real configured Scope/Raft applications. Their observation is historical
    // input to target 0's Request, never an assertion of target 0 currentness or authority.
    let (metadata_actor, tick, observation) = crate::driver::scope::select(&nodes, cohort, &[1, 2])
        .map_err(|error| format!("real configured peers 1/2 could not supply scope metadata; the current profile may not form its two-voter quorum with target 0 unconfigured: {error}"))?;
    if metadata_actor != 1 && metadata_actor != 2 {
        return Err("unconfigured target supplied its own scope metadata".into());
    }
    let (member, native_status) = crate::driver::status::membership(&nodes[0], fixture, tick + 1)?;
    let intent = crate::driver::operations::intent(&nodes[0].issuer, observation)?;
    for replica in &cohort.nodes {
        crate::driver::scope::publish_command(replica, &intent)?;
    }
    let work = crate::driver::flow::submit(&nodes[0], fixture, &member, &intent, member.tick + 1)?;
    if work.actor != 0 {
        return Err("unconfigured work was not submitted to its declared native target".into());
    }
    let claim_ref = crate::driver::flow::readmit(&nodes[0], fixture, &work, work.submit_tick + 1)?;
    let session = crate::driver::scenarios::Session {
        nodes,
        member,
        intent,
        work,
        claim_ref,
        native_status,
    };
    Ok((session, metadata_actor, started_absence))
}

fn deliver(
    cohort: &crate::producer::Cohort,
    session: &mut crate::driver::scenarios::Session,
) -> crate::producer::Result<(preserves::IOValue, preserves::IOValue, preserves::IOValue)> {
    let fixture = &cohort.nodes[0];
    session.require_live()?;
    crate::driver::flow::prepare_delivery(&session.delivery(cohort))?;
    let (success, output) = crate::driver::flow::execute_delivery(&session.delivery(cohort))?;
    if !success {
        return Err(format!("unconfigured native delivery was denied rather than unrouted: {output}").into());
    }
    let delivered = crate::driver::scenarios::admission::readback::receipt(&output)?;
    require_unrouted(&delivered)?;
    let live = crate::driver::scenarios::admission::readback::live_status(
        &session.nodes[0],
        fixture,
        session.work.submit_tick + 3,
    )?;
    crate::driver::scenarios::admission::readback::require_unknown(&live, &session.work.presentation.work_ref)?;
    let delivered_absence = absent(fixture)?;
    Ok((delivered, live, delivered_absence))
}

fn readiness(
    cohort: &crate::producer::Cohort,
    session: &crate::driver::scenarios::Session,
) -> crate::producer::Result<Vec<preserves::IOValue>> {
    let mut ready = Vec::with_capacity(session.nodes.len());
    for (owner, replica) in session.nodes.iter().zip(&cohort.nodes) {
        let name = owner.log.file_name().and_then(|name| name.to_str()).ok_or("owned serving log name")?;
        let bytes = replica.root.read(&molten::node_state::RelativePath::parse(name)?, 16 * 1024 * 1024)?;
        ready.push(molten::preserves_rail::parse_text(crate::driver::flow::value(
            std::str::from_utf8(&bytes)?,
            crate::driver::READY,
        )?)?);
    }
    Ok(ready)
}

/// Capability metadata only: deriving or opening a durable root would create the namespace.
fn absent(fixture: &crate::producer::Node) -> crate::producer::Result<preserves::IOValue> {
    let ledger = fixture.root.ledger_store()?;
    let mut resources = Vec::with_capacity(2);
    for (role, namespace) in [
        ("scope-application", &fixture.runtime.scope_store.storage_namespace),
        ("raft", &fixture.runtime.raft_store.storage_namespace),
    ] {
        if ledger.root().entry_kind_optional(&molten::local_store::Locator::parse(namespace)?)?.is_some() {
            return Err(format!("unconfigured native target created its {role} namespace").into());
        }
        resources.push(molten::preserves_rail::record("capability-namespace-observation-v1", vec![
            molten::preserves_rail::string(role),
            molten::preserves_rail::string(namespace),
            molten::preserves_rail::string("absent"),
        ]));
    }
    Ok(molten::preserves_rail::sequence(resources))
}

fn require_unrouted(receipt: &preserves::IOValue) -> crate::producer::Result<()> {
    let fields = molten::preserves_rail::simple_record_fields(receipt, "node-service-host-receipt-v1", 5)?;
    let delivered = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let fields = molten::preserves_rail::simple_record_fields(&delivered, "node-service-host-delivered-v1", 2)?;
    let outcome = molten::preserves_rail::value_to_iovalue(&fields[1]);
    let fields = molten::preserves_rail::simple_record_fields(&outcome, "node-service-host-effect-outcome-v1", 1)?;
    if crate::driver::operations::wire::text(&fields[0])? != "unrouted" {
        return Err("unconfigured native delivery did not report exact Unrouted disposition".into());
    }
    Ok(())
}

fn require_request(
    fixture: &crate::producer::Node,
    session: &crate::driver::scenarios::Session,
    status: &preserves::IOValue,
    native: &molten::system_extension::NativeInstanceRecord,
) -> crate::producer::Result<preserves::IOValue> {
    if native
        .completed_operations
        .iter()
        .any(|operation| operation.kind == molten::system_extension::NativeOperationKind::Effect)
        || native.unresolved.iter().any(|operation| operation.is_retry_permitted)
    {
        return Err("unconfigured target completed a protected effect or permitted an uncertain retry".into());
    }
    let receipt = work_receipt(status, &session.work.presentation.work_ref)?;
    if !native.completed_operations.iter().any(|operation| {
        operation.kind == molten::system_extension::NativeOperationKind::Ingress
            && operation.parent_ref == session.work.payload.value_ref
            && operation.state == molten::system_extension::NativeOperationState::Terminal
            && operation.terminal_ref.as_deref() == Some(receipt.as_str())
    }) {
        return Err("actual native journal does not bind the delivered staged payload".into());
    }
    let reference = native.state_ref.as_deref().ok_or("actual native bridge state missing")?;
    let member = crate::driver::scenarios::admission::readback::member(status)?;
    let states = molten::preserves_rail::required_sequence_field(&member[5], "actual native state")?;
    if states.len() != 1 || molten::preserves_rail::required_string_field(&states[0], "native state ref")? != reference
    {
        return Err("actual public native state differs from its journal".into());
    }
    let bridge = crate::driver::scenarios::admission::readback::materialize(fixture, reference)?;
    let fields = molten::preserves_rail::simple_record_fields(&bridge, "collaboration-native-bridge-v1", 5)?;
    let previous = crate::driver::scenarios::admission::readback::member(&session.native_status)?;
    let sequence = crate::driver::operations::wire::number(&fields[3])?;
    if sequence != native.callback_sequence
        || sequence != member[7].as_u64().ok_or("actual callback sequence")??
        || sequence
            != previous[7]
                .as_u64()
                .ok_or("initial callback sequence")??
                .checked_add(1)
                .ok_or("callback sequence overflow")?
        || crate::driver::operations::wire::number(&fields[1])? != session.member.generation
        || crate::driver::operations::wire::number(&fields[2])? != session.member.incarnation
    {
        return Err("unconfigured bridge did not retain exactly its genuine Request callback".into());
    }
    let last = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let last = molten::preserves_rail::simple_record_fields(&last, "optional", 1)?;
    if crate::driver::operations::wire::text(&last[0])? != session.intent.prepared.request.reference.as_str() {
        return Err("actual Request callback did not consume the intended staged request".into());
    }
    Ok(bridge)
}

/// Unknown work may retain its Request callback receipt; that is not an effect completion.
fn work_receipt(status: &preserves::IOValue, reference: &str) -> crate::producer::Result<String> {
    let status = crate::driver::scenarios::admission::readback::status(status)?;
    for work in molten::preserves_rail::required_sequence_field(&status[3], "actual work")?.iter() {
        let work = molten::preserves_rail::value_to_iovalue(work);
        let fields = molten::preserves_rail::simple_record_fields(&work, "host-work-status-v1", 10)?;
        if crate::driver::operations::wire::text(&fields[1])? != reference {
            continue;
        }
        let receipts = molten::preserves_rail::required_sequence_field(&fields[4], "Request observation receipt")?;
        let [receipt] = receipts.as_slice() else {
            return Err("unrouted Request lacks its one observed callback receipt".into());
        };
        return Ok(molten::preserves_rail::required_string_field(receipt, "Request observation receipt")?);
    }
    Err("unrouted Request work is absent".into())
}
