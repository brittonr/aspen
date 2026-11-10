//! Fault-specific readback through actual public status and released durable journals.

pub fn receipt(output: &str) -> crate::producer::Result<preserves::IOValue> {
    Ok(molten::preserves_rail::parse_text(crate::driver::flow::value(
        output,
        "node-service-host-receipt-v1",
    )?)?)
}

pub fn live_status(
    node: &crate::driver::Node,
    fixture: &crate::producer::Node,
    tick: u64,
) -> crate::producer::Result<preserves::IOValue> {
    let request = crate::driver::flow::authorize(
        &crate::driver::process::host_of(node),
        fixture,
        &crate::driver::operations::wire::operation("status", vec![]),
        tick,
    )?;
    // Retained diagnostic status uses the otherwise unused load nonce role. The final delivery
    // keeps its own exact operation nonce, even when observed at the same logical tick.
    crate::driver::process::sign(&crate::driver::process::host_of(node), fixture, &request, tick, 0)?;
    let (success, output) =
        crate::driver::process::invoke(&crate::driver::process::host_of(node), "status", tick, None)?;
    if !success {
        return Err("actual fault status denied".into());
    }
    receipt(&output)
}

pub fn status(receipt: &preserves::IOValue) -> crate::producer::Result<[preserves::IOValue; 4]> {
    let fields = molten::preserves_rail::simple_record_fields(receipt, "node-service-host-receipt-v1", 5)?;
    let value = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let fields = molten::preserves_rail::simple_record_fields(&value, "node-service-host-status-v1", 4)?;
    Ok(std::array::from_fn(|index| molten::preserves_rail::value_to_iovalue(&fields[index])))
}

pub fn member(receipt: &preserves::IOValue) -> crate::producer::Result<[preserves::IOValue; 10]> {
    let status = status(receipt)?;
    let members = molten::preserves_rail::required_sequence_field(&status[2], "actual members")?;
    let [member] = members.as_slice() else {
        return Err("one actual service member required".into());
    };
    let value = molten::preserves_rail::value_to_iovalue(member);
    let fields = molten::preserves_rail::simple_record_fields(&value, "host-member-status-v1", 10)?;
    Ok(std::array::from_fn(|index| molten::preserves_rail::value_to_iovalue(&fields[index])))
}

pub fn require_unknown(receipt: &preserves::IOValue, reference: &str) -> crate::producer::Result<()> {
    let status = status(receipt)?;
    let work = molten::preserves_rail::required_sequence_field(&status[3], "actual work")?;
    let mut has_found_work = false;
    for entry in work.iter() {
        let value = molten::preserves_rail::value_to_iovalue(entry);
        let fields = molten::preserves_rail::simple_record_fields(&value, "host-work-status-v1", 10)?;
        if crate::driver::operations::wire::text(&fields[1])? == reference {
            if has_found_work || crate::driver::operations::wire::text(&fields[2])? != "unknown" {
                return Err("possibly performed work was completed, requeued, or duplicated".into());
            }
            has_found_work = true;
        }
    }
    if !has_found_work {
        return Err("exact uncertain work missing".into());
    }
    Ok(())
}

pub fn native(
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
        .map_err(|error| format!("actual fault journal unavailable: {error:?}"))?
        .ok_or_else(|| "actual native journal missing".into())
}

pub fn unchanged(
    before: &molten::system_extension::NativeInstanceRecord,
    after: &molten::system_extension::NativeInstanceRecord,
) -> crate::producer::Result<()> {
    if before.unresolved != after.unresolved
        || before.completed_operations != after.completed_operations
        || before.completed_operation_refs != after.completed_operation_refs
        || before.state_ref != after.state_ref
        || before.callback_sequence != after.callback_sequence
        || before.lifecycle.generation != after.lifecycle.generation
        || before.lifecycle.incarnation != after.lifecycle.incarnation
        || after.unresolved.iter().any(|operation| operation.is_retry_permitted)
    {
        return Err("source-admitted reopen changed predecessor fence or native operation inventory".into());
    }
    Ok(())
}

pub fn values(
    fixture: &crate::producer::Node,
) -> crate::producer::Result<molten::system_extension::CapabilityCallbackValuePort> {
    Ok(molten::system_extension::CapabilityCallbackValuePort::new(
        fixture
            .root
            .ledger()?
            .open_subdir(&molten::node_state::RelativePath::parse(&fixture.runtime.callback_namespace)?)?,
    )?)
}

pub fn materialize(fixture: &crate::producer::Node, reference: &str) -> crate::producer::Result<preserves::IOValue> {
    let value = molten::system_extension::NativeCallbackValuePort::materialize(
        &mut values(fixture)?,
        reference,
        molten::collaboration_scope::MAX_RECORD_BYTES,
    )
    .map_err(|error| format!("actual callback value unavailable: {error:?}"))?;
    Ok(molten::preserves_rail::parse_canonical_bytes(&value.bytes)?)
}
