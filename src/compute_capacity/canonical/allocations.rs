// Allocation canonical projection and validation.
fn claim_value(claim: &molten_core::compute_capacity::Claim) -> preserves::IOValue {
    crate::preserves_rail::record("compute-claim-v1", vec![crate::preserves_rail::string(&claim.pool_id), crate::preserves_rail::u64_value(claim.quantity)])
}

fn parse_claim(value: &Field) -> Result<molten_core::compute_capacity::Claim> {
    let f = fields(value, "compute-claim-v1", 2)?;
    Ok(molten_core::compute_capacity::Claim {
        pool_id: text(&f[0], "claim pool id")?,
        quantity: number(&f[1], "claim quantity")?,
    })
}

fn validate_allocation(allocation: &molten_core::compute_capacity::Allocation) -> Result<()> {
    if allocation.schema != molten_core::compute_capacity::RESERVATION_SCHEMA
        || allocation.offer_revision == 0
        || allocation.topology_generation == 0
        || allocation.charge_cursor == 0
        || (allocation.status == molten_core::compute_capacity::ReservationStatus::Active && allocation.activation_backend_ref.is_none())
        || (allocation.status == molten_core::compute_capacity::ReservationStatus::Intent && allocation.activation_backend_ref.is_some())
        || allocation.owner_generation == 0
        || allocation.valid_until == 0
        || allocation.id != allocation.operation_id
        || allocation.claims.is_empty()
        || allocation.claims.len() > molten_core::compute_capacity::MAX_ITEMS
        || (allocation.kind == molten_core::compute_capacity::AllocationKind::ServingDeployment && allocation.serving_allocation_id.is_some())
    {
        return Err(invalid("invalid compute allocation schema, generation or claim bound"));
    }
    for (value, name) in [
        (&allocation.id, "allocation id"),
        (&allocation.host_id, "host id"),
        (&allocation.host_incarnation, "host incarnation"),
        (&allocation.device_incarnation, "device incarnation"),
        (&allocation.view_id, "view id"),
        (&allocation.consumer_id, "consumer id"),
        (&allocation.operation_id, "operation id"),
        (&allocation.attempt_id, "attempt id"),
    ] {
        token(value, name)?;
    }
    if let Some(id) = &allocation.serving_allocation_id {
        token(id, "serving allocation id")?;
    }
    if let Some(backend_ref) = &allocation.activation_backend_ref {
        token(backend_ref, "activation backend ref")?;
    }
    ref_token(&allocation.grant_ref, "grant ref")?;
    bounded(&allocation.authority_ids, "authority ids", false)?;
    if allocation.authority_ids.len() != allocation.claims.len()
        || allocation.authority_ids.iter().any(|id| id.is_empty())
    {
        return Err(invalid("allocation authorities must correspond to claims"));
    }
    let mut seen = std::collections::BTreeSet::new();
    if allocation.claims.iter().any(|claim| !seen.insert(&claim.pool_id)) {
        return Err(invalid("allocation claims repeat a physical pool"));
    }
    for claim in &allocation.claims {
        token(&claim.pool_id, "claim pool id")?;
        if claim.quantity == 0 {
            return Err(invalid("claim quantity must be positive"));
        }
    }
    Ok(())
}

// r[impl molten.compute_capacity.durable_reconciliation]
// r[impl molten.compute_capacity.reservation_authority]
// r[impl molten.compute_capacity.uncertainty_release]
pub fn allocation(allocation: &molten_core::compute_capacity::Allocation) -> Result<Record> {
    validate_allocation(allocation)?;
    materialize(crate::preserves_rail::record(ALLOCATION_RECORD, vec![
        crate::preserves_rail::string(&allocation.schema),
        crate::preserves_rail::string(&allocation.id),
        crate::preserves_rail::string(&allocation.host_id),
        crate::preserves_rail::string(&allocation.host_incarnation),
        crate::preserves_rail::string(&allocation.device_incarnation),
        crate::preserves_rail::u64_value(allocation.offer_revision),
        crate::preserves_rail::u64_value(allocation.topology_generation),
        crate::preserves_rail::u64_value(allocation.charge_cursor),
        crate::preserves_rail::u64_value(allocation.charge_at),
        crate::preserves_rail::u64_value(allocation.valid_until),
        strings(&allocation.authority_ids),
        crate::preserves_rail::string(&allocation.view_id),
        crate::preserves_rail::string(granularity_name(allocation.granularity)),
        crate::preserves_rail::string(&allocation.consumer_id),
        crate::preserves_rail::string(&allocation.operation_id),
        crate::preserves_rail::string(&allocation.attempt_id),
        crate::preserves_rail::u64_value(allocation.owner_generation),
        crate::preserves_rail::string(&allocation.grant_ref),
        crate::preserves_rail::sequence(allocation.claims.iter().map(claim_value).collect()),
        optional_value(allocation.serving_allocation_id.as_deref(), crate::preserves_rail::string),
        crate::preserves_rail::string(allocation_kind_name(allocation.kind)),
        crate::preserves_rail::string(status_name(allocation.status)),
        crate::preserves_rail::string(enforcement_name(allocation.enforcement)),
        optional_value(allocation.activation_backend_ref.as_deref(), crate::preserves_rail::string),
    ]))
}

// r[impl molten.compute_capacity.durable_reconciliation]
// r[impl molten.compute_capacity.reservation_authority]
pub fn parse_allocation(value: &preserves::IOValue) -> Result<molten_core::compute_capacity::Allocation> {
    let f = fields(value.as_ref(), ALLOCATION_RECORD, 24)?;
    let allocation = molten_core::compute_capacity::Allocation {
        schema: text(&f[0], "allocation schema")?,
        id: text(&f[1], "allocation id")?,
        host_id: text(&f[2], "host id")?,
        host_incarnation: text(&f[3], "host incarnation")?,
        device_incarnation: text(&f[4], "device incarnation")?,
        offer_revision: number(&f[5], "offer revision")?,
        topology_generation: number(&f[6], "topology generation")?,
        charge_cursor: number(&f[7], "charge cursor")?,
        charge_at: number(&f[8], "charge time")?,
        valid_until: number(&f[9], "valid until")?,
        authority_ids: sequence(&f[10], "authority ids", |v| text(v, "authority id"))?,
        view_id: text(&f[11], "view id")?,
        granularity: granularity(&f[12])?,
        consumer_id: text(&f[13], "consumer id")?,
        operation_id: text(&f[14], "operation id")?,
        attempt_id: text(&f[15], "attempt id")?,
        owner_generation: number(&f[16], "owner generation")?,
        grant_ref: reference(&f[17], "grant ref")?,
        claims: records(&f[18], "claims", parse_claim)?,
        serving_allocation_id: optional(&f[19], |v| text(v, "serving allocation id"))?,
        kind: allocation_kind(&f[20])?,
        status: status(&f[21])?,
        enforcement: enforcement(&f[22])?,
        activation_backend_ref: optional(&f[23], |v| text(v, "activation backend ref"))?,
    };
    validate_allocation(&allocation)?;
    Ok(allocation)
}
