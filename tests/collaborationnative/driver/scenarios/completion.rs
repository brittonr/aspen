//! Deterministic failure of only the real post-effect Message state publication.

use preserves::ValueImpl;

pub(super) fn post_effect_failure(cohort: &crate::producer::Cohort) -> crate::producer::Result<super::Observed> {
    let mut session = super::Session::open(cohort, crate::driver::operations::intent)?;
    let actor = session.work.actor;
    let fixture = &cohort.nodes[actor];
    let future = obstruct_state(fixture, &session.native_status, &session.intent)?;
    crate::driver::flow::prepare_delivery(&session.delivery(cohort))?;
    let (is_delivered, detail) = crate::driver::flow::execute_delivery(&session.delivery(cohort))?;
    if is_delivered {
        return Err(format!("obstructed completion publication was reported as success: {detail}").into());
    }
    let tick = session.work.submit_tick + 3;
    let live = crate::driver::scenarios::admission::readback::live_status(&session.nodes[actor], fixture, tick)?;
    crate::driver::scenarios::admission::readback::require_unknown(&live, &session.work.presentation.work_ref)?;
    // The obstruction is a definite pre-publication rejection, so every owner closes cleanly.
    // A killed durable store is quarantined by design and serves only explicit reconciliation,
    // not the journal readback this scenario proves; replacement evidence is separate.
    session.finish()?;
    let before = crate::driver::scenarios::admission::readback::native(fixture)?;
    let state = crate::driver::scope::persisted(fixture)?;
    let proposals = super::uncommitted::inventory(fixture)?;
    let (committed, completion, proposal) =
        require_post_effect_failure(fixture, &session.intent, &before, &state, &future)?;
    let reopened = crate::driver::scenarios::admission::readback::receipt(&crate::driver::process::reopened_status(
        &session.nodes[actor],
        fixture,
        tick + 1,
    )?)?;
    crate::driver::scenarios::admission::readback::require_unknown(&reopened, &session.work.presentation.work_ref)?;
    let after = crate::driver::scenarios::admission::readback::native(fixture)?;
    crate::driver::scenarios::admission::readback::unchanged(&before, &after)?;
    let reopened_proposals = super::uncommitted::inventory(fixture)?;
    if proposals != reopened_proposals {
        return Err("owner reopen changed actual proposal inventory".into());
    }
    if molten::collaboration_scope::state_value(&crate::driver::scope::persisted(fixture)?)
        != molten::collaboration_scope::state_value(&state)
    {
        return Err("owner reopen changed actual committed application inventory".into());
    }
    Ok(super::Observed {
        actor,
        work_ref: session.work.presentation.work_ref,
        next_tick: tick + 2,
        value: molten::preserves_rail::record("collaboration-native-completion-failure-observation-v1", vec![
            session.native_status,
            live,
            reopened,
            molten::preserves_rail::string(future),
            committed,
            molten::system_extension::canonical_instance_record(&before)?.value,
            molten::system_extension::canonical_instance_record(&after)?.value,
            molten::collaboration_scope::state_value(&state),
            proposals,
            reopened_proposals,
            completion,
            proposal,
        ]),
    })
}

fn obstruct_state(
    fixture: &crate::producer::Node,
    status: &preserves::IOValue,
    intent: &crate::driver::operations::Intent,
) -> crate::producer::Result<String> {
    let member = crate::driver::scenarios::admission::readback::member(status)?;
    let references = member[5].collect_sequence().ok_or("actual native state references")?;
    let [reference] = references.as_slice() else {
        return Err("one actual native state required".into());
    };
    let reference = molten::preserves_rail::required_string_field(reference, "native state")?;
    let current = crate::driver::scenarios::admission::readback::materialize(fixture, &reference)?;
    let fields = molten::preserves_rail::simple_record_fields(&current, "collaboration-native-bridge-v1", 5)?;
    let sequence = member[7].as_u64().ok_or("actual callback sequence")??;
    if crate::driver::operations::wire::number(&fields[3])? != sequence {
        return Err("public native sequence differs from materialized bridge state".into());
    }
    let command = molten::collaboration_scope::parse_staged_command(&molten::preserves_rail::parse_canonical_bytes(
        &intent.prepared.staged_bytes,
    )?)?;
    let mut future: Vec<_> = (0..5).map(|index| molten::preserves_rail::value_to_iovalue(&fields[index])).collect();
    future[3] = molten::preserves_rail::u64_value(sequence.checked_add(2).ok_or("callback sequence overflow")?);
    future[4] = molten::preserves_rail::record("optional", vec![molten::preserves_rail::string(command.request_ref)]);
    let future = molten::preserves_rail::canonical_hash(&molten::preserves_rail::record(
        "collaboration-native-bridge-v1",
        future,
    ))?;
    let digest = future.strip_prefix("blake3:").ok_or("future state digest")?;
    let namespace = fixture
        .root
        .ledger()?
        .open_subdir(&molten::node_state::RelativePath::parse(&fixture.runtime.callback_namespace)?)?;
    let leaf = molten::node_state::RelativePath::parse(&format!("values/{digest}.preserves"))?;
    if !matches!(namespace.observe_file(&leaf)?, molten::node_state::FileObservation::Missing) {
        return Err("future completion value leaf already exists".into());
    }
    namespace.create_dir_all(&leaf)?;
    Ok(future)
}

fn require_post_effect_failure(
    fixture: &crate::producer::Node,
    intent: &crate::driver::operations::Intent,
    native: &molten::system_extension::NativeInstanceRecord,
    state: &molten::collaboration_scope::State,
    future: &str,
) -> crate::producer::Result<(preserves::IOValue, preserves::IOValue, preserves::IOValue)> {
    let effects: Vec<_> = native
        .completed_operations
        .iter()
        .filter(|entry| {
            entry.kind == molten::system_extension::NativeOperationKind::Effect
                && entry.parent_ref == intent.prepared.effect_ref
                && entry.state == molten::system_extension::NativeOperationState::Terminal
        })
        .collect();
    let [effect] = effects.as_slice() else {
        return Err("exact terminal provider effect missing or duplicated".into());
    };
    let output_ref = effect.terminal_ref.as_deref().ok_or("provider result missing")?;
    let completion =
        crate::driver::status::commit::find_completion(fixture, native, &intent.prepared.effect_ref, output_ref)?;
    let completion_fields =
        molten::preserves_rail::simple_record_fields(&completion, "system-extension-effect-completion-v2", 11)?;
    let committed = crate::driver::status::commit::inline(
        &molten::preserves_rail::value_to_iovalue(&completion_fields[9]),
        output_ref,
    )?;
    let proposal = crate::driver::status::commit::require(fixture, intent, &committed, state)?;
    let fields = molten::preserves_rail::simple_record_fields(&committed, "collaboration-committed-command-v1", 8)?;
    if molten::collaboration_scope::validated_state_ref(state)? == intent.observation.state {
        return Err("terminal provider result does not advance the observed application state".into());
    }
    let position_value = molten::preserves_rail::value_to_iovalue(&fields[6]);
    let position =
        molten::preserves_rail::simple_record_fields(&position_value, "collaboration-live-commit-position-v1", 7)?;
    let index = crate::driver::operations::wire::number(&position[2])?;
    if index <= intent.observation.currentness.committed_index
        || crate::driver::operations::wire::number(&position[3])? < index
        || crate::driver::operations::wire::number(&position[4])? < index
        || state.currentness.committed_index < index
        || crate::driver::operations::wire::text(&position[0])? != fixture.runtime.replica.group_binding_ref
    {
        return Err("actual provider result lacks the committed and applied Raft position".into());
    }
    let molten::collaboration_scope::Mutation::Create(scope) = &intent.prepared.request.mutation else {
        return Err("expected genuine create mutation".into());
    };
    let has_failed_publication = has_failed_publication(native, future);
    if state.scopes.get(&scope.reference) != Some(scope)
        || !has_failed_publication
        || native.completed_operation_refs.contains(&effect.operation_ref)
        || native.state_ref.as_deref() == Some(future)
    {
        return Err("actual failed completion publication or unconsumed effect not retained".into());
    }
    Ok((committed, completion, proposal))
}

fn has_failed_publication(native: &molten::system_extension::NativeInstanceRecord, future: &str) -> bool {
    native.completed_operations.iter().any(|entry| {
        if entry.kind != molten::system_extension::NativeOperationKind::ValuePublication
            || entry.state != molten::system_extension::NativeOperationState::Terminal
        {
            return false;
        }
        let operation = molten_core::system_extension::native_identity_ref(&[
            "native-value-publication-operation-v2",
            &entry.parent_ref,
            "state",
            future,
            &native.lifecycle.generation.to_string(),
        ]);
        let rejection =
            molten_core::system_extension::native_identity_ref(&["native-value-publication-rejection-v2", &operation]);
        entry.operation_ref == operation
            && entry.terminal_ref.as_deref() == Some(rejection.as_str())
            && native.completed_operations.iter().any(|callback| {
                callback.kind == molten::system_extension::NativeOperationKind::Callback
                    && callback.operation_ref == entry.parent_ref
                    && callback.state == molten::system_extension::NativeOperationState::Terminal
            })
    })
}
