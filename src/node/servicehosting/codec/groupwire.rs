pub(super) fn encode(group: &super::super::Group) -> crate::error::Result<preserves::IOValue> {
    let history = group
        .history
        .iter()
        .map(|attempt| {
            crate::preserves_rail::record("host-attempt-v1", vec![
                crate::preserves_rail::string(&attempt.member_id),
                crate::preserves_rail::u64_value(attempt.observed_at_tick),
            ])
        })
        .collect();
    let actions = group.pending.iter().map(encode_action).collect::<crate::error::Result<Vec<_>>>()?;
    let mut fields = declaration_fields(&group.declaration);
    fields.extend([
        crate::preserves_rail::sequence(history),
        crate::preserves_rail::u64_value(group.last_tick),
        crate::preserves_rail::string(&group.admission_ref),
        crate::preserves_rail::sequence(actions),
        crate::preserves_rail::bool_value(group.is_terminal),
    ]);
    Ok(crate::preserves_rail::record("host-group-v1", fields))
}

pub(super) fn declaration_value(declaration: &crate::system_extension::RecoveryGroupDeclaration) -> preserves::IOValue {
    crate::preserves_rail::record("host-recovery-declaration-v1", declaration_fields(declaration))
}

fn declaration_fields(declaration: &crate::system_extension::RecoveryGroupDeclaration) -> Vec<preserves::IOValue> {
    let members = declaration
        .members
        .iter()
        .map(|member| {
            crate::preserves_rail::record("host-member-v1", vec![
                crate::preserves_rail::string(&member.member_id),
                crate::preserves_rail::string(member.restart_class.as_str()),
                crate::preserves_rail::refs_sequence(&member.after),
            ])
        })
        .collect();
    vec![
        crate::preserves_rail::string(&declaration.group_id),
        crate::preserves_rail::sequence(members),
        crate::preserves_rail::u64_value(declaration.window_ticks),
        crate::preserves_rail::u64_value(declaration.backoff_ticks),
        crate::preserves_rail::u64_value(declaration.max_restarts_per_member),
        crate::preserves_rail::u64_value(declaration.max_restarts_per_group),
        crate::preserves_rail::string(&declaration.elapsed_source_ref),
    ]
}

pub(super) fn decode(value: &preserves::IOValue) -> crate::error::Result<super::super::Group> {
    let fields = crate::preserves_rail::simple_record_fields(value, "host-group-v1", 12)?;
    let members = super::sequence(&fields[1], super::super::MAX_HOST_MEMBERS)?
        .iter()
        .map(decode_member)
        .collect::<crate::error::Result<Vec<_>>>()?;
    let history = super::sequence(&fields[7], crate::system_extension::MAX_RECOVERY_HISTORY)?
        .iter()
        .map(|value| {
            let value = crate::preserves_rail::value_to_iovalue(value);
            let fields = crate::preserves_rail::simple_record_fields(&value, "host-attempt-v1", 2)?;
            Ok(crate::system_extension::RecoveryAttempt {
                member_id: super::text(&fields[0])?,
                observed_at_tick: super::number(&fields[1])?,
            })
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    let pending = super::sequence(&fields[10], super::super::MAX_HOST_MEMBERS * 2)?
        .iter()
        .map(decode_action)
        .collect::<crate::error::Result<Vec<_>>>()?;
    Ok(super::super::Group {
        declaration: crate::system_extension::RecoveryGroupDeclaration {
            group_id: super::text(&fields[0])?,
            members,
            window_ticks: super::number(&fields[2])?,
            backoff_ticks: super::number(&fields[3])?,
            max_restarts_per_member: super::number(&fields[4])?,
            max_restarts_per_group: super::number(&fields[5])?,
            elapsed_source_ref: super::reference(&fields[6])?,
        },
        history,
        last_tick: super::number(&fields[8])?,
        admission_ref: super::reference(&fields[9])?,
        pending,
        is_terminal: super::boolean(&fields[11])?,
    })
}

fn decode_member(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<crate::system_extension::RecoveryGroupMember> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, "host-member-v1", 3)?;
    let restart_class = match super::text(&fields[1])?.as_str() {
        "permanent" => crate::system_extension::RestartClass::Permanent,
        "transient" => crate::system_extension::RestartClass::Transient,
        "temporary" => crate::system_extension::RestartClass::Temporary,
        _ => return Err(super::super::invalid("invalid stored restart class")),
    };
    Ok(crate::system_extension::RecoveryGroupMember {
        member_id: super::text(&fields[0])?,
        restart_class,
        after: super::sequence(&fields[2], super::super::MAX_HOST_MEMBERS)?
            .iter()
            .map(super::text)
            .collect::<crate::error::Result<Vec<_>>>()?,
    })
}

fn encode_action(action: &super::super::Action) -> crate::error::Result<preserves::IOValue> {
    let (kind, reference) = match &action.kind {
        super::super::RuntimeAction::Start => ("start", None),
        super::super::RuntimeAction::Stop => ("stop", None),
        super::super::RuntimeAction::Fence => ("fence", None),
        super::super::RuntimeAction::Restart => ("restart", None),
        super::super::RuntimeAction::Drain => ("drain", None),
        super::super::RuntimeAction::Checkpoint => ("checkpoint", None),
        super::super::RuntimeAction::Upgrade { admission_ref } => ("upgrade", Some(admission_ref.as_str())),
    };
    Ok(crate::preserves_rail::record("host-action-v1", vec![
        crate::preserves_rail::string(&action.member),
        crate::preserves_rail::string(kind),
        super::optional(reference),
        crate::system_extension::canonical_instance_record(&action.before)?.value,
        crate::preserves_rail::bool_value(action.is_attempted),
    ]))
}

fn decode_action(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<super::super::Action> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, "host-action-v1", 5)?;
    let reference = super::parse_optional(&fields[2])?;
    let kind = match (super::text(&fields[1])?.as_str(), reference) {
        ("start", None) => super::super::RuntimeAction::Start,
        ("stop", None) => super::super::RuntimeAction::Stop,
        ("fence", None) => super::super::RuntimeAction::Fence,
        ("restart", None) => super::super::RuntimeAction::Restart,
        ("drain", None) => super::super::RuntimeAction::Drain,
        ("checkpoint", None) => super::super::RuntimeAction::Checkpoint,
        ("upgrade", Some(admission_ref)) => super::super::RuntimeAction::Upgrade { admission_ref },
        _ => return Err(super::super::invalid("invalid stored lifecycle action")),
    };
    let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::value_to_iovalue(&fields[3]))?;
    Ok(super::super::Action {
        member: super::text(&fields[0])?,
        kind,
        before: crate::system_extension::decode_instance_record(&bytes)?,
        is_attempted: super::boolean(&fields[4])?,
    })
}
