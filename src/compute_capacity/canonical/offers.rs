// Offer and backing-pool canonical projections.
fn compatibility_value(value: &molten_core::compute_capacity::Compatibility) -> preserves::IOValue {
    crate::preserves_rail::record("compute-compatibility-v1", vec![
        optional_value(value.backend.as_deref(), crate::preserves_rail::string),
        optional_value(value.runtime.as_deref(), crate::preserves_rail::string),
        optional_value(value.architecture.as_deref(), crate::preserves_rail::string),
        optional_value(value.topology.as_deref(), crate::preserves_rail::string),
        strings(&value.qualification_refs),
    ])
}

fn parse_compatibility(value: &Field) -> Result<molten_core::compute_capacity::Compatibility> {
    let f = fields(value, "compute-compatibility-v1", 5)?;
    Ok(molten_core::compute_capacity::Compatibility {
        backend: optional(&f[0], |v| text(v, "backend"))?,
        runtime: optional(&f[1], |v| text(v, "runtime"))?,
        architecture: optional(&f[2], |v| text(v, "architecture"))?,
        topology: optional(&f[3], |v| text(v, "topology"))?,
        qualification_refs: sequence(&f[4], "qualification references", |v| reference(v, "qualification ref"))?,
    })
}

fn pool_value(pool: &molten_core::compute_capacity::Pool) -> preserves::IOValue {
    crate::preserves_rail::record("compute-pool-v1", vec![
        crate::preserves_rail::string(&pool.id),
        crate::preserves_rail::string(pool_kind_name(pool.kind)),
        crate::preserves_rail::string(&pool.authority_id),
        crate::preserves_rail::u64_value(pool.total),
        crate::preserves_rail::u64_value(pool.allocatable),
        optional_value(pool.observed_used, crate::preserves_rail::u64_value),
        optional_value(pool.observed_free, crate::preserves_rail::u64_value),
        crate::preserves_rail::bool_value(pool.is_observation_debited),
    ])
}

fn parse_pool(value: &Field) -> Result<molten_core::compute_capacity::Pool> {
    let f = fields(value, "compute-pool-v1", 8)?;
    Ok(molten_core::compute_capacity::Pool {
        id: text(&f[0], "pool id")?,
        kind: pool_kind(&f[1])?,
        authority_id: text(&f[2], "pool authority id")?,
        total: number(&f[3], "pool total")?,
        allocatable: number(&f[4], "pool allocatable")?,
        observed_used: optional(&f[5], |v| number(v, "observed used"))?,
        observed_free: optional(&f[6], |v| number(v, "observed free"))?,
        is_observation_debited: boolean(&f[7], "observation debited")?,
    })
}

fn view_value(view: &molten_core::compute_capacity::View) -> preserves::IOValue {
    crate::preserves_rail::record("compute-view-v1", vec![
        crate::preserves_rail::string(&view.id),
        crate::preserves_rail::string(&view.device_id),
        optional_value(view.partition_id.as_deref(), crate::preserves_rail::string),
        crate::preserves_rail::string(granularity_name(view.granularity)),
        strings(&view.backing_pools),
        crate::preserves_rail::bool_value(view.is_grouping_enforced),
    ])
}

fn parse_view(value: &Field) -> Result<molten_core::compute_capacity::View> {
    let f = fields(value, "compute-view-v1", 6)?;
    Ok(molten_core::compute_capacity::View {
        id: text(&f[0], "view id")?,
        device_id: text(&f[1], "device id")?,
        partition_id: optional(&f[2], |v| text(v, "partition id"))?,
        granularity: granularity(&f[3])?,
        backing_pools: sequence(&f[4], "backing pools", |v| text(v, "backing pool id"))?,
        is_grouping_enforced: boolean(&f[5], "grouping enforced")?,
    })
}

fn validate_offer(offer: &molten_core::compute_capacity::Offer) -> Result<()> {
    molten_core::compute_capacity::validate_offer(offer).map_err(|issue| invalid(format!("invalid compute offer: {issue:?}")))?;
    ref_token(&offer.provider_profile_ref, "provider profile ref")?;
    ref_token(&offer.source_profile_ref, "source profile ref")?;
    bounded(&offer.compatibility.qualification_refs, "qualification refs", true)
}

// r[impl molten.compute_capacity.durable_reconciliation]
// r[impl molten.compute_capacity.offers]
// r[impl molten.compute_capacity.backing_pools]
pub fn offer(offer: &molten_core::compute_capacity::Offer) -> Result<Record> {
    validate_offer(offer)?;
    materialize(crate::preserves_rail::record(OFFER_RECORD, vec![
        crate::preserves_rail::string(&offer.schema),
        crate::preserves_rail::string(&offer.host_id),
        crate::preserves_rail::string(&offer.host_incarnation),
        crate::preserves_rail::string(&offer.device_incarnation),
        crate::preserves_rail::string(&offer.provider_profile_ref),
        crate::preserves_rail::string(&offer.source_profile_ref),
        crate::preserves_rail::u64_value(offer.revision),
        crate::preserves_rail::u64_value(offer.topology_generation),
        crate::preserves_rail::u64_value(offer.observed_at),
        optional_value(offer.observation_charge_cursor, crate::preserves_rail::u64_value),
        crate::preserves_rail::u64_value(offer.expires_at),
        crate::preserves_rail::string(health_name(offer.health)),
        crate::preserves_rail::string(offer.label_authority.as_str()),
        compatibility_value(&offer.compatibility),
        crate::preserves_rail::sequence(offer.pools.iter().map(pool_value).collect()),
        crate::preserves_rail::sequence(offer.views.iter().map(view_value).collect()),
    ]))
}

// r[impl molten.compute_capacity.durable_reconciliation]
// r[impl molten.compute_capacity.offers]
pub fn parse_offer(value: &preserves::IOValue) -> Result<molten_core::compute_capacity::Offer> {
    let f = fields(value.as_ref(), OFFER_RECORD, 16)?;
    let offer = molten_core::compute_capacity::Offer {
        schema: text(&f[0], "offer schema")?,
        host_id: text(&f[1], "host id")?,
        host_incarnation: text(&f[2], "host incarnation")?,
        device_incarnation: text(&f[3], "device incarnation")?,
        provider_profile_ref: reference(&f[4], "provider profile ref")?,
        source_profile_ref: reference(&f[5], "source profile ref")?,
        revision: number(&f[6], "revision")?,
        topology_generation: number(&f[7], "topology generation")?,
        observed_at: number(&f[8], "observation tick")?,
        observation_charge_cursor: optional(&f[9], |v| number(v, "observation charge cursor"))?,
        expires_at: number(&f[10], "expiration tick")?,
        health: health(&f[11])?,
        label_authority: label_authority(&f[12])?,
        compatibility: parse_compatibility(&f[13])?,
        pools: records(&f[14], "pools", parse_pool)?,
        views: records(&f[15], "views", parse_view)?,
    };
    validate_offer(&offer)?;
    Ok(offer)
}
