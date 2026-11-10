use preserves::ValueImpl;

/// A real proposal enters the owning replica before its canonical append exceeds the admitted
/// transport bound. No owner is killed and no protected operation is retried.
pub(super) fn queued(cohort: &crate::producer::Cohort) -> crate::producer::Result<super::Observed> {
    let budget = crate::producer::uncommitted_frame_budget()?;
    if cohort.nodes.iter().any(|node| node.runtime.transport.profile.limits.max_frame_bytes != budget) {
        return Err("queued case requires its canonical separating frame budget".into());
    }
    let mut session = super::Session::open(cohort, crate::driver::operations::intent)?;
    let actor = session.work.actor;
    let delivered = crate::driver::flow::deliver(&session.delivery(cohort));
    // Even a transport error must close every database owner before durable inspection.
    session.finish()?;
    let (success, _) = delivered?;
    if success {
        return Err("bounded append unexpectedly reported a successful delivery".into());
    }
    let before = persisted(cohort, &session.intent, actor)?;
    let fixture = &cohort.nodes[actor];
    let native = crate::driver::status::latest_instance(fixture)?;
    require_unknown(&native, &session.intent.prepared.effect_ref)?;
    let reopened =
        crate::driver::process::reopened_status(&session.nodes[actor], fixture, session.work.submit_tick + 15)?;
    let reopened =
        molten::preserves_rail::parse_text(crate::driver::flow::value(&reopened, "node-service-host-receipt-v1")?)?;
    super::admission::readback::require_unknown(&reopened, &session.work.presentation.work_ref)?;
    let after = persisted(cohort, &session.intent, actor)?;
    if before != after {
        return Err("source-admitted status reopen changed actual queued replica/application facts".into());
    }
    let reopened_native = crate::driver::status::latest_instance(fixture)?;
    require_unknown(&reopened_native, &session.intent.prepared.effect_ref)?;
    Ok(super::Observed {
        actor,
        work_ref: session.work.presentation.work_ref,
        next_tick: session.work.submit_tick + 16,
        value: molten::preserves_rail::record("collaboration-native-queued-uncommitted-observation-v1", vec![
            molten::preserves_rail::u64_value(budget),
            molten::preserves_rail::string(session.intent.observation.state.as_str()),
            before,
            molten::system_extension::canonical_instance_record(&native)?.value,
            reopened,
            after,
            molten::system_extension::canonical_instance_record(&reopened_native)?.value,
        ]),
    })
}

fn require_unknown(
    native: &molten::system_extension::NativeInstanceRecord,
    effect: &str,
) -> crate::producer::Result<()> {
    if native.completed_operations.iter().any(|operation| {
        operation.kind == molten::system_extension::NativeOperationKind::Effect && operation.parent_ref == effect
    }) || !native.unresolved.iter().any(|operation| {
        operation.kind == molten::system_extension::NativeOperationKind::Effect
            && operation.parent_ref == effect
            && operation.state == molten::system_extension::NativeOperationState::Unknown
            && !operation.is_retry_permitted
    }) {
        return Err("actual queued native effect is not unresolved Unknown with retry forbidden".into());
    }
    Ok(())
}

fn persisted(
    cohort: &crate::producer::Cohort,
    intent: &crate::driver::operations::Intent,
    actor: usize,
) -> crate::producer::Result<preserves::IOValue> {
    let command = molten::collaboration_scope::parse_staged_command(&molten::preserves_rail::parse_canonical_bytes(
        &intent.prepared.staged_bytes,
    )?)?;
    let molten::collaboration_scope::Mutation::Create(expected) = &intent.prepared.request.mutation else {
        return Err("queued case requires the actual scope creation".into());
    };
    let mut observations = Vec::with_capacity(cohort.nodes.len());
    for (index, fixture) in cohort.nodes.iter().enumerate() {
        let state = crate::driver::scope::persisted(fixture)?;
        if state.scopes.contains_key(&expected.reference) {
            return Err("queued scope unexpectedly appeared in an actual durable application".into());
        }
        let (log, proposal) = replica_log(fixture, intent, &command)?;
        if index == actor && proposal.is_none() {
            return Err("actual owning replica did not durably queue the exact proposed command".into());
        }
        observations.push(molten::preserves_rail::record("collaboration-native-uncommitted-replica-v1", vec![
            molten::preserves_rail::string(&fixture.runtime.node_id),
            log,
            molten::collaboration_scope::state_value(&state),
            molten::preserves_rail::sequence(proposal.into_iter().collect()),
        ]));
    }
    Ok(molten::preserves_rail::sequence(observations))
}

/// Decode only real durable records through the canonical storage codec. A local log mutation
/// is evidence of queuing, not evidence of commit; a later truncation invalidates that evidence.
/// Match the intended request/schema, then validate the entry's fresh authorized proposal identity.
fn replica_log(
    fixture: &crate::producer::Node,
    intent: &crate::driver::operations::Intent,
    command: &molten::fabric_consistency::raft::Command,
) -> crate::producer::Result<(preserves::IOValue, Option<preserves::IOValue>)> {
    let retained = inventory(fixture)?;
    let mut exact = None;
    let mut commit_index = 0;
    for value in molten::preserves_rail::required_sequence_field(&retained, "actual durable Raft inventory")?
        .as_slice()
        .iter()
    {
        let value = molten::preserves_rail::value_to_iovalue(value);
        if let Some(fields) = value.collect_simple_record("raft-log-mutation-v1", Some(2)) {
            if let Some(truncate) = fields[0].collect_simple_record("some", Some(1)) {
                let from = crate::driver::operations::wire::number(&truncate[0])?;
                if exact.as_ref().is_some_and(|(index, _)| *index >= from) {
                    exact = None;
                }
            }
            for entry in molten::preserves_rail::required_sequence_field(&fields[1], "actual Raft entries")?
                .as_slice()
                .iter()
            {
                let entry_value = molten::preserves_rail::value_to_iovalue(entry);
                let entry = molten::preserves_rail::simple_record_fields(&entry_value, "raft-replicated-entry-v1", 6)?;
                if crate::driver::operations::wire::text(&entry[3])? == command.request_ref
                    && crate::driver::operations::wire::text(&entry[5])? == command.command_schema_ref
                {
                    let proposal = crate::driver::status::commit::require_proposal(
                        fixture,
                        intent,
                        &crate::driver::operations::wire::text(&entry[4])?,
                    )?;
                    exact = Some((crate::driver::operations::wire::number(&entry[1])?, proposal));
                }
            }
        }
        if let Some(fields) = value.collect_simple_record("raft-commit-boundary-v1", Some(1)) {
            commit_index = commit_index.max(crate::driver::operations::wire::number(&fields[0])?);
        }
    }
    if exact.as_ref().is_some_and(|(index, _)| commit_index >= *index) {
        return Err("actual queued log entry has a durable commit boundary".into());
    }
    Ok((retained, exact.map(|(_, proposal)| proposal)))
}

/// Actual canonical Raft storage records, read only after the owning process has closed.
/// Election hard state, flush markers and snapshots are process lifecycle, not retained
/// append/truncate/commit facts, so the observation and its reopen comparison exclude them.
pub(super) fn inventory(fixture: &crate::producer::Node) -> crate::producer::Result<preserves::IOValue> {
    let input = &fixture.runtime.raft_store;
    let profile = input.profile.clone().into_profile()?;
    let namespace = input.namespace.clone().into_descriptor(&profile)?;
    let root = molten::local_store::DurableStoreRoot::from_ledger(
        &fixture.root.ledger_store()?,
        &molten::local_store::Locator::parse(&input.storage_namespace)?,
    )?;
    let adapter = molten::fabric_durability::RedbDurableStateAdapter::open_root(root, profile, namespace)?;
    let values: crate::producer::Result<Vec<_>> = adapter
        .state()
        .durable_log
        .iter()
        .map(|record| Ok(molten::preserves_rail::parse_canonical_bytes(&record.value)?))
        .collect();
    Ok(molten::preserves_rail::sequence(values?.into_iter().filter(is_protected_record).collect()))
}

fn is_protected_record(value: &preserves::IOValue) -> bool {
    value.collect_simple_record("raft-log-mutation-v1", Some(2)).is_some()
        || value.collect_simple_record("raft-commit-boundary-v1", Some(1)).is_some()
}

/// Every retained append for the exact request is checked regardless of its proposal identity,
/// including entries later truncated: denial must precede queueing.
pub(super) fn absent(
    cohort: &crate::producer::Cohort,
    intent: &crate::driver::operations::Intent,
) -> crate::producer::Result<preserves::IOValue> {
    let staged = molten::collaboration_scope::parse_staged_command(&molten::preserves_rail::parse_canonical_bytes(
        &intent.prepared.staged_bytes,
    )?)?;
    let mut observed = Vec::with_capacity(cohort.nodes.len());
    for fixture in &cohort.nodes {
        let retained = inventory(fixture)?;
        for value in
            molten::preserves_rail::required_sequence_field(&retained, "actual denied-proposal inventory")?.as_slice()
        {
            let Some(fields) = value.collect_simple_record("raft-log-mutation-v1", Some(2)) else {
                continue;
            };
            for entry in
                molten::preserves_rail::required_sequence_field(&fields[1], "actual appended entries")?.as_slice()
            {
                let fields = entry
                    .collect_simple_record("raft-replicated-entry-v1", Some(6))
                    .ok_or("actual appended entry schema differs")?;
                let request = fields[3].as_string().ok_or("appended request identity")?;
                if request.as_ref() == staged.request_ref.as_str() {
                    return Err("current scope denial admitted its protected proposal to the actual replica log".into());
                }
            }
        }
        observed.push(molten::preserves_rail::record("collaboration-native-prequeue-inventory-v1", vec![
            molten::preserves_rail::string(&fixture.runtime.node_id),
            retained,
        ]));
    }
    Ok(molten::preserves_rail::sequence(observed))
}
