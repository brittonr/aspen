pub(super) fn declaration_ref(
    declaration: &crate::system_extension::RecoveryGroupDeclaration,
) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&groupwire::declaration_value(declaration))
}

mod groupwire;
mod workwire;

pub(super) fn encode(state: &super::State) -> crate::error::Result<preserves::IOValue> {
    validate_bounds(state)?;
    let groups = state.groups.values().map(groupwire::encode).collect::<crate::error::Result<Vec<_>>>()?;
    let members = state
        .members
        .values()
        .map(|member| crate::system_extension::canonical_instance_record(member).map(|record| record.value))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let work = state.work.values().map(workwire::encode).collect();
    Ok(crate::preserves_rail::record("node-service-host-state-v1", vec![
        crate::preserves_rail::u64_value(state.revision),
        crate::preserves_rail::sequence(groups),
        crate::preserves_rail::sequence(members),
        crate::preserves_rail::sequence(work),
    ]))
}

pub(super) fn decode(bytes: &[u8]) -> crate::error::Result<super::State> {
    let value = crate::preserves_rail::strict_canonical_decode(bytes)?.value;
    let fields = crate::preserves_rail::simple_record_fields(&value, "node-service-host-state-v1", 4)?;
    let mut state = super::State {
        revision: number(&fields[0])?,
        ..super::State::default()
    };
    for value in sequence(&fields[1], super::MAX_HOST_GROUPS)? {
        let group = groupwire::decode(&crate::preserves_rail::value_to_iovalue(&value))?;
        if state.groups.insert(group.declaration.group_id.clone(), group).is_some() {
            return Err(super::invalid("duplicate hosted group"));
        }
    }
    for value in sequence(&fields[2], super::MAX_HOST_MEMBERS)? {
        let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::value_to_iovalue(&value))?;
        let member = crate::system_extension::decode_instance_record(&bytes)?;
        if state.members.insert(member.service_id.clone(), member).is_some() {
            return Err(super::invalid("duplicate hosted member"));
        }
    }
    for value in sequence(&fields[3], super::MAX_HOST_WORK)? {
        let work = workwire::decode(&crate::preserves_rail::value_to_iovalue(&value))?;
        if state.work.insert(work.presentation.work_ref.clone(), work).is_some() {
            return Err(super::invalid("duplicate hosted durable work"));
        }
    }
    validate_bounds(&state)?;
    Ok(state)
}

pub(super) fn number(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| super::invalid("hosting field must be u64"))?
        .map_err(|error| super::invalid(format!("hosting u64 out of range: {error}")))
}

pub(super) fn text(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    crate::preserves_rail::required_string_field(value, "hosting text")
}

pub(super) fn reference(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    crate::preserves_rail::required_content_ref_string(value, "hosting content reference")
}

pub(super) fn boolean(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<bool> {
    value.as_boolean().ok_or_else(|| super::invalid("hosting field must be boolean"))
}

pub(super) fn sequence(
    value: &preserves::Value<preserves::IOValue>,
    maximum: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    let values = crate::preserves_rail::required_sequence_field(value, "hosting collection")?;
    if values.len() > maximum {
        return Err(super::invalid("hosting collection exceeds item bound"));
    }
    Ok(values.into_owned())
}

pub(super) fn optional(reference: Option<&str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(reference.iter().map(crate::preserves_rail::string).collect())
}

pub(super) fn parse_optional(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Option<String>> {
    let values = sequence(value, 1)?;
    values.first().map(reference).transpose()
}

fn validate_bounds(state: &super::State) -> crate::error::Result<()> {
    if state.groups.len() > super::MAX_HOST_GROUPS
        || state.members.len() > super::MAX_HOST_MEMBERS
        || state.work.len() > super::MAX_HOST_WORK
    {
        return Err(super::invalid("node hosting state exceeds collection bound"));
    }
    let mut ownership = std::collections::BTreeSet::new();
    for group in state.groups.values() {
        if !crate::system_extension::validate_recovery_group(&group.declaration).is_empty()
            || group.history.len() > crate::system_extension::MAX_RECOVERY_HISTORY
            || group.declaration.max_restarts_per_group > crate::system_extension::MAX_RECOVERY_HISTORY as u64
            || group.pending.len() > super::MAX_HOST_MEMBERS * 2
            || group.declaration.members.iter().any(|member| !state.members.contains_key(&member.member_id))
        {
            return Err(super::invalid("invalid persisted hosted group"));
        }
        for member in &group.declaration.members {
            if !ownership.insert(member.member_id.as_str()) {
                return Err(super::invalid("member belongs to multiple persisted recovery groups"));
            }
        }
        if group.history.iter().any(|attempt| {
            attempt.observed_at_tick > group.last_tick
                || !group.declaration.members.iter().any(|member| member.member_id == attempt.member_id)
        }) || group.pending.iter().any(|action| {
            action.before.service_id != action.member
                || !group.declaration.members.iter().any(|member| member.member_id == action.member)
        }) {
            return Err(super::invalid("persisted recovery history/action escapes admitted group"));
        }
    }
    for work in state.work.values() {
        if !state.members.contains_key(&work.member)
            || work.payload.bytes.len() > super::MAX_HOST_PAYLOAD_BYTES
            || work.migrations.len() > super::MAX_HOST_WORK_MIGRATIONS
        {
            return Err(super::invalid("invalid persisted hosted work"));
        }
        if let Some(claim) = &work.claim {
            let member = state.members.get(&work.member).ok_or_else(|| super::invalid("unknown claim member"))?;
            if work.presentation.class != crate::system_extension::WorkClass::Durable
                || claim.class != crate::system_extension::WorkClass::Durable
                || claim.generation != member.lifecycle.generation
                || claim.incarnation != member.lifecycle.incarnation
            {
                return Err(super::invalid("persisted work claim is not current for its member"));
            }
        }
        for (index, migration) in work.migrations.iter().enumerate() {
            let target = work.migrations.get(index + 1).map_or(&work.presentation, |next| &next.source);
            if migration.source.work_ref != work.presentation.work_ref
                || migration.source.class != crate::system_extension::WorkClass::Durable
                || migration.source.generation.checked_add(1) != Some(target.generation)
            {
                return Err(super::invalid("persisted durable work migration lineage is not adjacent"));
            }
        }
        crate::system_extension::admit_callback_value(&work.payload, super::MAX_HOST_PAYLOAD_BYTES as u64)
            .map_err(|error| super::invalid(format!("invalid durable payload: {error:?}")))?;
    }
    Ok(())
}
