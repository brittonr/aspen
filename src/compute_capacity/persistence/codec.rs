// r[impl molten.compute_capacity.durable_reconciliation]
fn encode_state(state: &molten_core::compute_capacity::CapacityState) -> crate::fabric::FabricPortResult<Vec<u8>> {
    if state.offers.len() > molten_core::compute_capacity::MAX_ITEMS
        || state.allocations.len() > molten_core::compute_capacity::MAX_ITEMS
        || state.retired_allocations.len() > molten_core::compute_capacity::MAX_ITEMS
    {
        return Err(crate::fabric::FabricPortError::malformed("compute state exceeds collection bound"));
    }
    if state.allocations.values().any(|allocation| {
        allocation.status == molten_core::compute_capacity::ReservationStatus::Released
            || allocation.charge_cursor > state.charge_cursor
    }) || state.retired_allocations.iter().any(|allocation| allocation.charge_cursor > state.charge_cursor)
        || state
            .offers
            .values()
            .any(|offer| offer.observation_charge_cursor.is_some_and(|cursor| cursor > state.charge_cursor))
    {
        return Err(crate::fabric::FabricPortError::malformed(
            "inconsistent compute charge cursor or allocation status",
        ));
    }
    let mut retired_ids = std::collections::BTreeSet::new();
    if state.retired_allocations.iter().any(|row| !retired_ids.insert(&row.id)) {
        return Err(crate::fabric::FabricPortError::malformed("duplicate retired compute allocation"));
    }
    let offers = state
        .offers
        .iter()
        .map(|(host, offer)| {
            if host != &offer.host_id {
                return Err(crate::fabric::FabricPortError::malformed("offer map key mismatch"));
            }
            Ok(super::canonical::offer(offer).map_err(crate::fabric::FabricPortError::from)?.value)
        })
        .collect::<crate::fabric::FabricPortResult<Vec<_>>>()?;
    let allocations = state
        .allocations
        .iter()
        .map(|(id, allocation)| {
            if id != &allocation.id {
                return Err(crate::fabric::FabricPortError::malformed("allocation map key mismatch"));
            }
            Ok(super::canonical::allocation(allocation).map_err(crate::fabric::FabricPortError::from)?.value)
        })
        .collect::<crate::fabric::FabricPortResult<Vec<_>>>()?;
    let retired = state
        .retired_allocations
        .iter()
        .map(|allocation| {
            if allocation.status != molten_core::compute_capacity::ReservationStatus::Released
                || state.allocations.contains_key(&allocation.id)
            {
                return Err(crate::fabric::FabricPortError::malformed("invalid retired allocation"));
            }
            Ok(super::canonical::allocation(allocation).map_err(crate::fabric::FabricPortError::from)?.value)
        })
        .collect::<crate::fabric::FabricPortResult<Vec<_>>>()?;
    let value = crate::preserves_rail::record(STATE_LABEL, vec![
        crate::preserves_rail::string("molten.compute.owner-state.v2"),
        crate::preserves_rail::sequence(offers),
        crate::preserves_rail::sequence(allocations),
        crate::preserves_rail::u64_value(state.charge_cursor),
        crate::preserves_rail::sequence(retired),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value).map_err(crate::fabric::FabricPortError::from)?;
    if bytes.len() > MAX_STATE_BYTES {
        return Err(crate::fabric::FabricPortError::malformed("compute state exceeds byte bound"));
    }
    Ok(bytes)
}

fn decode_state(bytes: &[u8]) -> crate::fabric::FabricPortResult<molten_core::compute_capacity::CapacityState> {
    if bytes.len() > MAX_STATE_BYTES {
        return Err(crate::fabric::FabricPortError::malformed("compute state exceeds byte bound"));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes).map_err(crate::fabric::FabricPortError::from)?;
    let state_fields = fields(&value, STATE_LABEL, 5)?;
    if state_fields[0].as_string().as_deref() != Some("molten.compute.owner-state.v2") {
        return Err(crate::fabric::FabricPortError::malformed("wrong compute owner state schema"));
    }
    let mut offers = std::collections::BTreeMap::new();
    for item in items(&state_fields[1], molten_core::compute_capacity::MAX_ITEMS)? {
        let offer = super::canonical::parse_offer(&item).map_err(crate::fabric::FabricPortError::from)?;
        if offers.insert(offer.host_id.clone(), offer).is_some() {
            return Err(crate::fabric::FabricPortError::malformed("duplicate stored compute offer"));
        }
    }
    let mut allocations = std::collections::BTreeMap::new();
    for item in items(&state_fields[2], molten_core::compute_capacity::MAX_ITEMS)? {
        let allocation = super::canonical::parse_allocation(&item).map_err(crate::fabric::FabricPortError::from)?;
        if allocations.insert(allocation.id.clone(), allocation).is_some() {
            return Err(crate::fabric::FabricPortError::malformed("duplicate stored compute allocation"));
        }
    }
    let charge_cursor = state_fields[3]
        .as_u64()
        .ok_or_else(|| crate::fabric::FabricPortError::malformed("invalid charge cursor"))?
        .map_err(|_| crate::fabric::FabricPortError::malformed("charge cursor out of range"))?;
    let mut retired_allocations = std::collections::VecDeque::new();
    let mut retired_ids = std::collections::BTreeSet::new();
    for item in items(&state_fields[4], molten_core::compute_capacity::MAX_ITEMS)? {
        let allocation = super::canonical::parse_allocation(&item).map_err(crate::fabric::FabricPortError::from)?;
        if allocation.status != molten_core::compute_capacity::ReservationStatus::Released
            || allocations.contains_key(&allocation.id)
            || !retired_ids.insert(allocation.id.clone())
        {
            return Err(crate::fabric::FabricPortError::malformed("invalid retired compute allocation"));
        }
        retired_allocations.push_back(allocation);
    }
    if allocations
        .values()
        .any(|allocation| allocation.status == molten_core::compute_capacity::ReservationStatus::Released)
        || allocations
            .values()
            .chain(retired_allocations.iter())
            .any(|allocation| allocation.charge_cursor > charge_cursor)
        || offers
            .values()
            .any(|offer| offer.observation_charge_cursor.is_some_and(|cursor| cursor > charge_cursor))
    {
        return Err(crate::fabric::FabricPortError::malformed(
            "inconsistent compute charge cursor or allocation status",
        ));
    }
    Ok(molten_core::compute_capacity::CapacityState {
        offers,
        allocations,
        charge_cursor,
        retired_allocations,
    })
}

fn fields(
    value: &preserves::IOValue,
    label: &str,
    count: usize,
) -> crate::fabric::FabricPortResult<Vec<preserves::IOValue>> {
    let record = value
        .collect_simple_record(label, Some(count))
        .ok_or_else(|| crate::fabric::FabricPortError::malformed(format!("expected {label}")))?;
    Ok(record.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

fn items(value: &preserves::IOValue, maximum: usize) -> crate::fabric::FabricPortResult<Vec<preserves::IOValue>> {
    let sequence = value
        .collect_sequence()
        .ok_or_else(|| crate::fabric::FabricPortError::malformed("expected compute state sequence"))?;
    if sequence.len() > maximum {
        return Err(crate::fabric::FabricPortError::malformed("compute state sequence exceeds bound"));
    }
    Ok(sequence.iter().map(crate::preserves_rail::value_to_iovalue).collect())
}