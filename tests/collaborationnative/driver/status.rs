//! Public status observations: membership convergence and durable work completion.

pub(super) mod commit;

/// Observe the exact work item's durable completion through the retained owner's public status.
pub fn completion(
    node: &super::Node,
    fixture: &crate::producer::Node,
    work_ref: &str,
    tick: u64,
) -> crate::producer::Result<String> {
    let operation = super::operations::wire::operation("status", Vec::new());
    let mut last = String::from("absent");
    for attempt in 0..COMPLETION_ATTEMPTS {
        let at = tick + attempt;
        super::flow::authorize(&super::process::host_of(node), fixture, &operation, at)?;
        let (success, output) = super::process::invoke(&super::process::host_of(node), "status", at, None)?;
        if !success {
            return Err(format!("status denied at tick {at}: {output}").into());
        }
        if let Some(receipt) = completed(&output, work_ref)? {
            return Ok(receipt);
        }
        last = state_of(&output, work_ref)?.unwrap_or_else(|| "absent".to_string());
        std::thread::sleep(std::time::Duration::from_millis(STATUS_PAUSE_MS));
    }
    let log = std::fs::read_to_string(&node.log).unwrap_or_default();
    let tail: String = log.chars().rev().take(2_000).collect::<String>().chars().rev().collect();
    Err(format!(
        "work {work_ref} remained {last} within {COMPLETION_ATTEMPTS} status observations\nserve log tail: {tail}"
    )
    .into())
}

/// The observed work state for the exact item, so a timeout names what it saw.
fn state_of(output: &str, work_ref: &str) -> crate::producer::Result<Option<String>> {
    let receipt = molten::preserves_rail::parse_text(super::flow::value(output, "node-service-host-receipt-v1")?)?;
    let fields = molten::preserves_rail::simple_record_fields(&receipt, "node-service-host-receipt-v1", 5)?;
    let status = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let status = molten::preserves_rail::simple_record_fields(&status, "node-service-host-status-v1", 4)?;
    let work = molten::preserves_rail::required_sequence_field(&status[3], "host status work")?;
    for item in work.iter() {
        let record = molten::preserves_rail::value_to_iovalue(item);
        let entry = molten::preserves_rail::simple_record_fields(&record, "host-work-status-v1", 10)?;
        if molten::preserves_rail::required_string_field(&entry[1], "work ref")? == work_ref {
            return Ok(Some(molten::preserves_rail::required_string_field(&entry[2], "work state")?));
        }
    }
    Ok(None)
}

/// The completion receipt for the exact work item, read from a status receipt.
pub fn completed(output: &str, work_ref: &str) -> crate::producer::Result<Option<String>> {
    let receipt = molten::preserves_rail::parse_text(super::flow::value(output, "node-service-host-receipt-v1")?)?;
    let fields = molten::preserves_rail::simple_record_fields(&receipt, "node-service-host-receipt-v1", 5)?;
    let status = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let status = molten::preserves_rail::simple_record_fields(&status, "node-service-host-status-v1", 4)?;
    let work = molten::preserves_rail::required_sequence_field(&status[3], "host status work")?;
    for item in work.iter() {
        let record = molten::preserves_rail::value_to_iovalue(item);
        let entry = molten::preserves_rail::simple_record_fields(&record, "host-work-status-v1", 10)?;
        if molten::preserves_rail::required_string_field(&entry[1], "work ref")? != work_ref {
            continue;
        }
        if molten::preserves_rail::required_string_field(&entry[2], "work state")? != "completed" {
            return Ok(None);
        }
        let receipt = molten::preserves_rail::required_sequence_field(&entry[4], "completion receipt")?;
        if receipt.len() != 1 {
            return Err("completed work lacks its one durable completion receipt".into());
        }
        return Ok(Some(molten::preserves_rail::required_string_field(&receipt[0], "completion receipt")?));
    }
    Ok(None)
}

/// Reject acknowledged-but-unrouted or effect-free delivery as evidence for the one commit effect.
pub fn routed_completion(output: &str) -> crate::producer::Result<String> {
    let receipt = molten::preserves_rail::parse_text(super::flow::value(output, "node-service-host-receipt-v1")?)?;
    let fields = molten::preserves_rail::simple_record_fields(&receipt, "node-service-host-receipt-v1", 5)?;
    let delivered = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let delivered = molten::preserves_rail::simple_record_fields(&delivered, "node-service-host-delivered-v1", 2)?;
    let outcome = molten::preserves_rail::value_to_iovalue(&delivered[1]);
    let outcome = molten::preserves_rail::simple_record_fields(&outcome, "node-service-host-effect-outcome-v1", 2)?;
    if molten::preserves_rail::required_string_field(&outcome[0], "effect disposition")? != "completed" {
        return Err("the actual commit effect did not complete".into());
    }
    let refs = molten::preserves_rail::required_sequence_field(&outcome[1], "effect completion refs")?;
    let [completion] = refs.as_slice() else {
        return Err("the one approved commit effect requires exactly one completion callback".into());
    };
    let completion = molten::preserves_rail::required_string_field(completion, "effect completion ref")?;
    molten::preserves_rail::validate_content_ref(&completion)?;
    Ok(completion)
}

/// Membership converges only after the cohort's live transports connect, so the observation is
/// retried under an explicit attempt bound; every attempt is a fresh authorized operation.
const STATUS_ATTEMPTS: u64 = 60;
const STATUS_PAUSE_MS: u64 = 5_000;
/// Completion is a durable observation after delivery; each observation authorizes its exact
/// operation without reloading the admitted source or reinstalling the selected runtime.
const COMPLETION_ATTEMPTS: u64 = 12;

/// The active member's generation and incarnation from a current status observation.
pub fn membership(
    node: &super::Node,
    fixture: &crate::producer::Node,
    tick: u64,
) -> crate::producer::Result<(super::operations::Member, preserves::IOValue)> {
    let operation = super::operations::wire::operation("status", Vec::new());
    for attempt in 0..STATUS_ATTEMPTS {
        let at = tick + attempt;
        super::flow::authorize(&super::process::host_of(node), fixture, &operation, at)?;
        let (success, output) = super::process::invoke(&super::process::host_of(node), "status", at, None)?;
        if !success {
            return Err(format!("status denied at tick {at}: {output}").into());
        }
        let receipt = molten::preserves_rail::parse_text(super::flow::value(&output, "node-service-host-receipt-v1")?)?;
        if let Some(member) = member(&receipt, at)? {
            return Ok((member, receipt));
        }
        std::thread::sleep(std::time::Duration::from_millis(STATUS_PAUSE_MS));
    }
    Err(format!("node {} reported no group member within {STATUS_ATTEMPTS} attempts", node.index).into())
}

/// The first member of a status receipt, or `None` while the group has not converged.
fn member(receipt: &preserves::IOValue, tick: u64) -> crate::producer::Result<Option<super::operations::Member>> {
    let fields = molten::preserves_rail::simple_record_fields(receipt, "node-service-host-receipt-v1", 5)?;
    let status = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let status = molten::preserves_rail::simple_record_fields(&status, "node-service-host-status-v1", 4)?;
    let members = molten::preserves_rail::required_sequence_field(&status[2], "host status members")?;
    let Some(member) = members.first() else {
        return Ok(None);
    };
    let member_value = molten::preserves_rail::value_to_iovalue(member);
    let member = molten::preserves_rail::simple_record_fields(&member_value, "host-member-status-v1", 10)?;
    Ok(Some(super::operations::Member {
        generation: super::operations::wire::number(&member[1])?,
        incarnation: super::operations::wire::number(&member[2])?,
        tick,
    }))
}

/// Read actual native and application journals only after all owning processes have closed.
pub(super) fn observed_commit(
    fixture: &crate::producer::Node,
    intent: &super::operations::Intent,
    completion: &str,
) -> crate::producer::Result<preserves::IOValue> {
    let native = latest_instance(fixture)?;
    let readiness = molten::system_extension::service_readiness(
        &native.manifest_ref,
        &native.extension_id,
        &native.service_id,
        &native.lifecycle,
        completion,
    )?;
    if !readiness.ready
        || !native.evidence_refs.contains(&readiness.readiness_ref)
        || !native.is_accepting_ingress
        || native
            .unresolved
            .iter()
            .any(|operation| operation.kind == molten::system_extension::NativeOperationKind::Effect)
    {
        return Err("the actual completion callback did not retain healthy quiescent native evidence".into());
    }
    let (committed, completion_value) = commit::materialize(fixture, &native, &intent.prepared.effect_ref)?;
    let state = super::scope::persisted(fixture)?;
    let molten::collaboration_scope::Mutation::Create(expected) = &intent.prepared.request.mutation else {
        return Err("consumer requires its declared create-scope operation".into());
    };
    if state.scopes.get(&expected.reference) != Some(expected) {
        return Err("the actual application did not retain the requested scope and membership".into());
    }
    let proposal = commit::require(fixture, intent, &committed, &state)?;
    Ok(molten::preserves_rail::record("collaboration-native-observation-v1", vec![
        molten::system_extension::canonical_instance_record(&native)?.value,
        readiness.value,
        completion_value,
        committed,
        proposal,
        molten::collaboration_scope::state_value(&state),
    ]))
}

pub(super) fn latest_instance(
    fixture: &crate::producer::Node,
) -> crate::producer::Result<molten::system_extension::NativeInstanceRecord> {
    let service = fixture.bundle.services.first().ok_or("native service")?;
    let root = molten::local_store::DurableStoreRoot::from_ledger(
        &fixture.root.ledger_store()?,
        &molten::local_store::Locator::parse(&service.storage_namespace)?,
    )?;
    let adapter = molten::fabric_durability::RedbDurableStateAdapter::open_root(
        root,
        molten::fabric_durability::durable_profile(&service.durability_profile)?,
        service.namespace.clone(),
    )?;
    let journal = molten::system_extension::Durable::new(adapter);
    molten::system_extension::NativeHostJournal::latest_instance(&journal, &service.context.instance_id)
        .map_err(|error| format!("native journal readback failed: {error:?}"))?
        .ok_or_else(|| "actual native instance was not durably retained".into())
}
