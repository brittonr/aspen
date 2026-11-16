// Capacity readback canonical projection.
fn allocation_readback_value(row: &molten_core::compute_capacity::AllocationReadback) -> preserves::IOValue {
    crate::preserves_rail::record("compute-allocation-readback-v1", vec![
        crate::preserves_rail::string(&row.id),
        crate::preserves_rail::string(&row.consumer_id),
        crate::preserves_rail::u64_value(row.owner_generation),
        crate::preserves_rail::string(granularity_name(row.granularity)),
        crate::preserves_rail::string(status_name(row.status)),
        crate::preserves_rail::bool_value(row.is_logically_fenced),
        crate::preserves_rail::string(enforcement_name(row.enforcement)),
    ])
}

fn parse_allocation_readback(value: &Field) -> Result<molten_core::compute_capacity::AllocationReadback> {
    let f = fields(value, "compute-allocation-readback-v1", 7)?;
    Ok(molten_core::compute_capacity::AllocationReadback {
        id: text(&f[0], "allocation id")?,
        consumer_id: text(&f[1], "consumer id")?,
        owner_generation: number(&f[2], "owner generation")?,
        granularity: granularity(&f[3])?,
        status: status(&f[4])?,
        is_logically_fenced: boolean(&f[5], "logically fenced")?,
        enforcement: enforcement(&f[6])?,
    })
}

fn validate_readback(readback: &molten_core::compute_capacity::CapacityReadback) -> Result<()> {
    token(&readback.host_id, "host id")?;
    bounded(&readback.qualification_refs, "qualification refs", true)?;
    if readback.revision == 0 || readback.allocations.len() > molten_core::compute_capacity::MAX_ITEMS {
        return Err(invalid("invalid readback revision or allocation bound"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in &readback.allocations {
        token(&row.id, "allocation id")?;
        token(&row.consumer_id, "consumer id")?;
        if !seen.insert(&row.id)
            || row.owner_generation == 0
            || row.is_logically_fenced != (row.status != molten_core::compute_capacity::ReservationStatus::Released)
        {
            return Err(invalid("invalid readback owner or logical fence"));
        }
    }
    Ok(())
}

// r[impl molten.compute_capacity.durable_reconciliation]
// r[impl molten.compute_capacity.readback]
pub fn readback(readback: &molten_core::compute_capacity::CapacityReadback) -> Result<Record> {
    validate_readback(readback)?;
    materialize(crate::preserves_rail::record(READBACK_RECORD, vec![
        crate::preserves_rail::string(&readback.host_id),
        crate::preserves_rail::u64_value(readback.revision),
        crate::preserves_rail::bool_value(readback.is_fresh),
        crate::preserves_rail::string(health_name(readback.health)),
        strings(&readback.qualification_refs),
        crate::preserves_rail::sequence(readback.allocations.iter().map(allocation_readback_value).collect()),
    ]))
}

// r[impl molten.compute_capacity.durable_reconciliation]
// r[impl molten.compute_capacity.readback]
pub fn parse_readback(value: &preserves::IOValue) -> Result<molten_core::compute_capacity::CapacityReadback> {
    let f = fields(value.as_ref(), READBACK_RECORD, 6)?;
    let readback = molten_core::compute_capacity::CapacityReadback {
        host_id: text(&f[0], "host id")?,
        revision: number(&f[1], "revision")?,
        is_fresh: boolean(&f[2], "freshness")?,
        health: health(&f[3])?,
        qualification_refs: sequence(&f[4], "qualification refs", |v| reference(v, "qualification ref"))?,
        allocations: records(&f[5], "allocations", parse_allocation_readback)?,
    };
    validate_readback(&readback)?;
    Ok(readback)
}
