//! Independent scope denial after genuine route and host grants.
#[path = "admission/inputs.rs"]
pub(super) mod inputs;
#[path = "admission/readback.rs"]
pub(super) mod readback;

pub(super) fn run(
    cohort: &crate::producer::Cohort,
    mode: crate::driver::scenarios::admission::inputs::Mode,
) -> crate::producer::Result<super::Observed> {
    let mut session = super::Session::open(cohort, crate::driver::operations::intent)?;
    let actor = session.work.actor;
    let fixture = &cohort.nodes[actor];
    let label = mode.label();
    let expected_code = match &mode {
        crate::driver::scenarios::admission::inputs::Mode::Missing => Some("collaboration-scope/missing"),
        crate::driver::scenarios::admission::inputs::Mode::Stale => Some("collaboration-scope/stale"),
        crate::driver::scenarios::admission::inputs::Mode::Malformed => None,
    };
    let request = crate::driver::flow::prepare_delivery(&session.delivery(cohort))?;
    let current_trust = mode.apply(&session.delivery(cohort), &request)?;
    let (is_delivered, detail) = crate::driver::flow::execute_delivery(&session.delivery(cohort))?;
    if is_delivered || expected_code.is_some_and(|code| detail != code) {
        return Err(format!("scope {label} did not return its fail-closed denial: {detail}").into());
    }
    let tick = session.work.submit_tick + 3;
    let live = crate::driver::scenarios::admission::readback::live_status(&session.nodes[actor], fixture, tick)?;
    crate::driver::scenarios::admission::readback::require_unknown(&live, &session.work.presentation.work_ref)?;
    session.finish()?;
    let before = crate::driver::scenarios::admission::readback::native(fixture)?;
    let state = crate::driver::scope::persisted(fixture)?;
    let proposals = super::uncommitted::absent(cohort, &session.intent)?;
    if molten::collaboration_scope::validated_state_ref(&state)? != session.intent.observation.state {
        return Err("scope-denied delivery changed the actual application baseline".into());
    }
    require_unresolved_effect(&before, &session.intent)?;
    let reopened = crate::driver::process::reopened_status(&session.nodes[actor], fixture, tick + 1)?;
    let reopened = crate::driver::scenarios::admission::readback::receipt(&reopened)?;
    crate::driver::scenarios::admission::readback::require_unknown(&reopened, &session.work.presentation.work_ref)?;
    let after = crate::driver::scenarios::admission::readback::native(fixture)?;
    crate::driver::scenarios::admission::readback::unchanged(&before, &after)?;
    let reopened_proposals = super::uncommitted::absent(cohort, &session.intent)?;
    if proposals != reopened_proposals {
        return Err("source-admitted reopen changed denied proposal inventory".into());
    }
    if molten::collaboration_scope::state_value(&crate::driver::scope::persisted(fixture)?)
        != molten::collaboration_scope::state_value(&state)
    {
        return Err("source-admitted reopen changed denied application state".into());
    }
    Ok(super::Observed {
        actor,
        work_ref: session.work.presentation.work_ref,
        next_tick: tick + 2,
        value: molten::preserves_rail::record("collaboration-native-scope-authority-failure-observation-v1", vec![
            molten::preserves_rail::string(label),
            molten::preserves_rail::string(current_trust),
            molten::preserves_rail::string(detail),
            session.intent.observation.value(),
            live,
            reopened,
            molten::system_extension::canonical_instance_record(&before)?.value,
            molten::system_extension::canonical_instance_record(&after)?.value,
            molten::collaboration_scope::state_value(&state),
            proposals,
            reopened_proposals,
        ]),
    })
}

fn require_unresolved_effect(
    native: &molten::system_extension::NativeInstanceRecord,
    intent: &crate::driver::operations::Intent,
) -> crate::producer::Result<()> {
    let effects: Vec<_> = native
        .unresolved
        .iter()
        .filter(|entry| {
            entry.kind == molten::system_extension::NativeOperationKind::Effect
                && entry.parent_ref == intent.prepared.effect_ref
        })
        .collect();
    if effects.len() != 1
        || effects[0].is_retry_permitted
        || effects[0].state != molten::system_extension::NativeOperationState::Unknown
        || native.completed_operations.iter().any(|entry| {
            entry.kind == molten::system_extension::NativeOperationKind::Effect
                && entry.parent_ref == intent.prepared.effect_ref
        })
    {
        return Err("scope denial did not retain the exact unresolved nonretryable effect".into());
    }
    Ok(())
}
