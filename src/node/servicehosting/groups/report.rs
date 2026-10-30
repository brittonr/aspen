impl<A: super::super::AuthorityPort, R: super::super::RuntimePort> super::super::Host<A, R> {
    pub fn status(&mut self, logical_tick: u64) -> crate::error::Result<preserves::IOValue> {
        let input_ref = self.state_ref()?;
        self.admit("status", "node-service-host", &input_ref, logical_tick)?;
        let members = self
            .state
            .members
            .values()
            .map(|member| self.member_status(member))
            .collect::<crate::error::Result<Vec<_>>>()?;
        let work = self.state.work.values().map(work_status).collect();
        let groups = self.state.groups.values().map(group_status).collect();
        Ok(crate::preserves_rail::record("node-service-host-status-v1", vec![
            crate::preserves_rail::string(&self.state_ref()?),
            crate::preserves_rail::sequence(groups),
            crate::preserves_rail::sequence(members),
            crate::preserves_rail::sequence(work),
        ]))
    }

    fn member_status(
        &self,
        persisted: &crate::system_extension::NativeInstanceRecord,
    ) -> crate::error::Result<preserves::IOValue> {
        let observed = self.runtime.instance(&persisted.service_id)?;
        let group_id = self.member_group(&persisted.service_id)?;
        let group = self.state.groups.get(&group_id).ok_or_else(|| super::super::invalid("unknown status group"))?;
        let is_accepting = persisted.is_accepting_ingress
            && observed.is_accepting_ingress
            && !group.is_terminal
            && group.pending.is_empty();
        Ok(crate::preserves_rail::record("host-member-status-v1", vec![
            crate::preserves_rail::string(&observed.service_id),
            crate::preserves_rail::u64_value(observed.lifecycle.generation),
            crate::preserves_rail::u64_value(observed.lifecycle.incarnation),
            crate::preserves_rail::string(observed.lifecycle.phase.as_str()),
            crate::preserves_rail::bool_value(is_accepting),
            super::super::codec::optional(observed.state_ref.as_deref()),
            super::super::codec::optional(observed.checkpoint_ref.as_deref()),
            crate::preserves_rail::u64_value(observed.callback_sequence),
            crate::preserves_rail::string(&observed.manifest_ref),
            native_inventory(&observed),
        ]))
    }
}

fn work_status(work: &super::super::Work) -> preserves::IOValue {
    crate::preserves_rail::record("host-work-status-v1", vec![
        crate::preserves_rail::string(&work.member),
        crate::preserves_rail::string(&work.presentation.work_ref),
        crate::preserves_rail::string(match work.state {
            super::super::WorkState::Pending => "pending",
            super::super::WorkState::Claimed => "claimed",
            super::super::WorkState::Unknown => "unknown",
            super::super::WorkState::Completed => "completed",
        }),
        super::super::codec::optional(work.claim.as_ref().map(|claim| claim.claim_ref.as_str())),
        super::super::codec::optional(work.receipt_ref.as_deref()),
        crate::preserves_rail::string(work.presentation.class.as_str()),
        crate::preserves_rail::u64_value(work.presentation.generation),
        crate::preserves_rail::u64_value(work.presentation.incarnation),
        crate::preserves_rail::string(&work.payload.value_ref),
        crate::preserves_rail::sequence(
            work.migrations
                .iter()
                .map(|migration| {
                    crate::preserves_rail::record("host-work-lineage-v1", vec![
                        crate::preserves_rail::u64_value(migration.source.generation),
                        crate::preserves_rail::u64_value(migration.source.incarnation),
                        crate::preserves_rail::string(&migration.source_payload_ref),
                        crate::preserves_rail::string(&migration.migration_ref),
                        crate::preserves_rail::string(&migration.admission_ref),
                    ])
                })
                .collect(),
        ),
    ])
}

fn group_status(group: &super::super::Group) -> preserves::IOValue {
    crate::preserves_rail::record("host-group-status-v1", vec![
        crate::preserves_rail::string(&group.declaration.group_id),
        crate::preserves_rail::u64_value(group.last_tick),
        crate::preserves_rail::u64_value(group.history.len() as u64),
        crate::preserves_rail::u64_value(group.pending.len() as u64),
        crate::preserves_rail::bool_value(group.is_terminal),
        crate::preserves_rail::string(&group.declaration.elapsed_source_ref),
        crate::preserves_rail::string(&group.admission_ref),
        crate::preserves_rail::sequence(
            group
                .history
                .iter()
                .map(|attempt| {
                    crate::preserves_rail::record("host-attempt-v1", vec![
                        crate::preserves_rail::string(&attempt.member_id),
                        crate::preserves_rail::u64_value(attempt.observed_at_tick),
                    ])
                })
                .collect(),
        ),
    ])
}

fn native_inventory(member: &crate::system_extension::NativeInstanceRecord) -> preserves::IOValue {
    crate::preserves_rail::sequence(
        crate::system_extension::classify_native_recovery(member)
            .iter()
            .map(|entry| {
                crate::preserves_rail::record("host-native-recovery-v1", vec![
                    crate::preserves_rail::string(&entry.operation_ref),
                    crate::preserves_rail::string(entry.kind.as_str()),
                    crate::preserves_rail::string(match entry.class {
                        crate::system_extension::NativeRecoveryClass::NotStarted => "not-started",
                        crate::system_extension::NativeRecoveryClass::RunningObserved => "running-observed",
                        crate::system_extension::NativeRecoveryClass::Terminal => "terminal",
                        crate::system_extension::NativeRecoveryClass::Unknown => "unknown",
                        crate::system_extension::NativeRecoveryClass::Stale => "stale",
                    }),
                    crate::preserves_rail::bool_value(entry.is_retry_permitted),
                ])
            })
            .collect(),
    )
}
