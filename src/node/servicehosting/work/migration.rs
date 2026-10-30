impl<A: super::super::AuthorityPort, R: super::super::RuntimePort> super::super::Host<A, R> {
    /// Explicitly migrate retained logical work; ordinary readmission never changes generation.
    pub fn readmit_migrated(
        &mut self,
        member: &str,
        input: &super::super::MigratedWorkInput,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::DeliveryClaim> {
        let group_id = self.member_group(member)?;
        self.available_group(&group_id, logical_tick)?;
        crate::system_extension::admit_callback_value(&input.payload, super::super::MAX_HOST_PAYLOAD_BYTES as u64)
            .map_err(|error| super::super::invalid(format!("migrated work payload denied: {error:?}")))?;
        let current = self
            .state
            .members
            .get(member)
            .ok_or_else(|| super::super::invalid("unknown migrated work member"))?;
        let work = self
            .state
            .work
            .get(&input.work_ref)
            .ok_or_else(|| super::super::invalid("unknown migrated logical work"))?;
        if work.member != member
            || work.presentation.class != crate::system_extension::WorkClass::Durable
            || work.presentation.generation.checked_add(1) != Some(current.lifecycle.generation)
            || !current.is_accepting_ingress
            || current.lifecycle.phase != crate::system_extension::LifecyclePhase::Running
            || matches!(work.state, super::super::WorkState::Unknown | super::super::WorkState::Completed)
            || work.migrations.len() >= super::super::MAX_HOST_WORK_MIGRATIONS
        {
            return Err(super::super::invalid(
                "logical work is not eligible for an adjacent admitted generation migration",
            ));
        }
        let receipt = self.runtime.migration(member, &input.migration_ref)?;
        validate_receipt(&receipt, current)?;
        let input_ref = input_ref(input)?;
        let admission_ref = self.admit("readmit-migrated", member, &input_ref, logical_tick)?;
        let claim = super::claim_for(&self.state, member, &input.work_ref, &admission_ref)?;
        let mut next = self.state.clone();
        let work = next
            .work
            .get_mut(&input.work_ref)
            .ok_or_else(|| super::super::invalid("migrated work disappeared"))?;
        work.migrations.push(super::super::WorkMigration {
            source: work.presentation.clone(),
            source_payload_ref: work.payload.value_ref.clone(),
            migration_ref: input.migration_ref.clone(),
            admission_ref,
        });
        work.presentation.generation = claim.generation;
        work.presentation.incarnation = claim.incarnation;
        work.payload = input.payload.clone();
        work.claim = Some(claim.clone());
        work.state = super::super::WorkState::Claimed;
        next.groups
            .get_mut(&group_id)
            .ok_or_else(|| super::super::invalid("unknown migrated work group"))?
            .last_tick = logical_tick;
        self.commit(next)?;
        Ok(claim)
    }
}

fn validate_receipt(
    value: &preserves::IOValue,
    current: &crate::system_extension::NativeInstanceRecord,
) -> crate::error::Result<()> {
    let fields = crate::preserves_rail::simple_record_fields(value, "system-extension-state-migration-v1", 11)?;
    if super::super::codec::text(&fields[0])? != crate::system_extension::SYSTEM_EXTENSION_STATE_MIGRATION_SCHEMA
        || text_field(&fields[1], "operation")? != "upgrade"
        || text_field(&fields[2], "extension-id")? != current.extension_id
        || text_field(&fields[3], "service-id")? != current.service_id
        || text_field(&fields[5], "next-manifest-ref")? != current.manifest_ref
        || number_field(&fields[9], "generation")? != current.lifecycle.generation
    {
        return Err(super::super::invalid("durable migration receipt does not bind the executing replacement"));
    }
    Ok(())
}

fn text_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, label, 1)?;
    super::super::codec::text(&fields[0])
}

fn number_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, label, 1)?;
    super::super::codec::number(&fields[0])
}

pub(in crate::node_service_hosting) fn input_ref(
    input: &super::super::MigratedWorkInput,
) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("host-migrated-work-input-v1", vec![
        crate::preserves_rail::string(&input.work_ref),
        crate::preserves_rail::string(&input.migration_ref),
        crate::preserves_rail::string(&input.payload.value_ref),
    ]))
}
