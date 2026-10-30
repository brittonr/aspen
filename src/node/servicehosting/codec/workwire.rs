pub(super) fn encode(work: &super::super::Work) -> preserves::IOValue {
    let presentation = &work.presentation;
    let claim = work
        .claim
        .as_ref()
        .map(|claim| {
            crate::preserves_rail::record("host-claim-v1", vec![
                crate::preserves_rail::string(&claim.claim_ref),
                crate::preserves_rail::u64_value(claim.generation),
                crate::preserves_rail::u64_value(claim.incarnation),
            ])
        })
        .into_iter()
        .collect();
    crate::preserves_rail::record("host-work-v1", vec![
        crate::preserves_rail::string(&work.member),
        crate::preserves_rail::string(&presentation.work_ref),
        crate::preserves_rail::string(presentation.class.as_str()),
        crate::preserves_rail::u64_value(presentation.generation),
        crate::preserves_rail::u64_value(presentation.incarnation),
        crate::preserves_rail::string(&work.payload.value_ref),
        crate::preserves_rail::sequence(
            work.payload.bytes.iter().map(|byte| crate::preserves_rail::u64_value(u64::from(*byte))).collect(),
        ),
        crate::preserves_rail::sequence(claim),
        crate::preserves_rail::string(match work.state {
            super::super::WorkState::Pending => "pending",
            super::super::WorkState::Claimed => "claimed",
            super::super::WorkState::Unknown => "unknown",
            super::super::WorkState::Completed => "completed",
        }),
        super::optional(work.receipt_ref.as_deref()),
        crate::preserves_rail::sequence(work.migrations.iter().map(encode_migration).collect()),
    ])
}

pub(super) fn decode(value: &preserves::IOValue) -> crate::error::Result<super::super::Work> {
    let fields = crate::preserves_rail::simple_record_fields(value, "host-work-v1", 11)?;
    let class = match super::text(&fields[2])?.as_str() {
        "runtime-local" => crate::system_extension::WorkClass::RuntimeLocal,
        "durable" => crate::system_extension::WorkClass::Durable,
        _ => return Err(super::super::invalid("invalid stored work class")),
    };
    let bytes = super::sequence(&fields[6], super::super::MAX_HOST_PAYLOAD_BYTES)?
        .iter()
        .map(|value| {
            u8::try_from(super::number(value)?).map_err(|_| super::super::invalid("invalid hosted payload byte"))
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    let claims = super::sequence(&fields[7], 1)?;
    let claim = claims
        .first()
        .map(|value| -> crate::error::Result<crate::system_extension::DeliveryClaim> {
            let value = crate::preserves_rail::value_to_iovalue(value);
            let fields = crate::preserves_rail::simple_record_fields(&value, "host-claim-v1", 3)?;
            Ok(crate::system_extension::DeliveryClaim {
                claim_ref: super::reference(&fields[0])?,
                class: crate::system_extension::WorkClass::Durable,
                generation: super::number(&fields[1])?,
                incarnation: super::number(&fields[2])?,
            })
        })
        .transpose()?;
    let state = match super::text(&fields[8])?.as_str() {
        "pending" => super::super::WorkState::Pending,
        "claimed" => super::super::WorkState::Claimed,
        "unknown" => super::super::WorkState::Unknown,
        "completed" => super::super::WorkState::Completed,
        _ => return Err(super::super::invalid("invalid stored durable work disposition")),
    };
    Ok(super::super::Work {
        member: super::text(&fields[0])?,
        presentation: crate::system_extension::WorkPresentation {
            work_ref: super::reference(&fields[1])?,
            class,
            generation: super::number(&fields[3])?,
            incarnation: super::number(&fields[4])?,
        },
        payload: crate::system_extension::NativeCallbackValue {
            value_ref: super::reference(&fields[5])?,
            bytes,
        },
        claim,
        state,
        receipt_ref: super::parse_optional(&fields[9])?,
        migrations: super::sequence(&fields[10], super::super::MAX_HOST_WORK_MIGRATIONS)?
            .iter()
            .map(decode_migration)
            .collect::<crate::error::Result<Vec<_>>>()?,
    })
}

fn encode_migration(migration: &super::super::WorkMigration) -> preserves::IOValue {
    crate::preserves_rail::record("host-work-migration-v1", vec![
        crate::preserves_rail::string(&migration.source.work_ref),
        crate::preserves_rail::u64_value(migration.source.generation),
        crate::preserves_rail::u64_value(migration.source.incarnation),
        crate::preserves_rail::string(&migration.source_payload_ref),
        crate::preserves_rail::string(&migration.migration_ref),
        crate::preserves_rail::string(&migration.admission_ref),
    ])
}

fn decode_migration(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<super::super::WorkMigration> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, "host-work-migration-v1", 6)?;
    Ok(super::super::WorkMigration {
        source: crate::system_extension::WorkPresentation {
            work_ref: super::reference(&fields[0])?,
            class: crate::system_extension::WorkClass::Durable,
            generation: super::number(&fields[1])?,
            incarnation: super::number(&fields[2])?,
        },
        source_payload_ref: super::reference(&fields[3])?,
        migration_ref: super::reference(&fields[4])?,
        admission_ref: super::reference(&fields[5])?,
    })
}
