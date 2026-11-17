pub(super) fn validate(
    expected: &super::ExpectedCohort,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    bundle: &super::Bundle,
    original_facts: &super::checks::Facts,
    original_history: &chaoscontrol_smr::HistoryReport,
) -> Result<Vec<String>, super::ImportIssue> {
    let Some(replay) = &bundle.replay else {
        return Ok(Vec::new());
    };
    let original = &replay.original;
    if original.operation_ids != original_facts.operation_ids
        || original.proposal_outcomes != bundle.proposals
        || original.observations != original_facts.chains
        || original.safety_prefixes != original_history.safety_prefixes
        || original.stabilization_facts != bundle.stabilization
        || original.liveness != bundle.liveness
        || original.terminal_status != bundle.terminal_status
    {
        return Err(super::ImportIssue::Replay);
    }
    let repeat = &replay.repeat;
    let repeat_facts = super::checks::facts(expected, plan, &replay.repeat_observations, &repeat.proposal_outcomes)?;
    let repeat_history =
        chaoscontrol_smr::validate_history(plan, &repeat_facts.chains).map_err(|_| super::ImportIssue::Replay)?;
    super::checks::stabilization(expected, &repeat_facts, repeat.stabilization_facts.as_ref())?;
    let repeat_liveness = chaoscontrol_smr::evaluate_liveness(plan, repeat.stabilization_facts.as_ref())
        .map_err(|_| super::ImportIssue::Replay)?;
    if repeat.operation_ids != repeat_facts.operation_ids
        || repeat.observations != repeat_facts.chains
        || repeat.safety_prefixes != repeat_history.safety_prefixes
        || repeat.liveness != repeat_liveness
    {
        return Err(super::ImportIssue::Replay);
    }
    // Neither identical failing runs nor identical instrumentation fixtures are
    // execution or snapshot-backed replay proof. A genuine mismatch is retained.
    let comparison = chaoscontrol_smr::compare_replay_bounded(original, repeat, &plan.profile.bounds)
        .map_err(|_| super::ImportIssue::Replay)?;
    if comparison != replay.comparison {
        return Err(super::ImportIssue::Replay);
    }
    let refs = vec![
        super::artifact_ref("molten-chaoscontrol-semantic-original-v1", original)?,
        super::artifact_ref("molten-chaoscontrol-semantic-repeat-v1", repeat)?,
        super::artifact_ref("molten-chaoscontrol-repeat-bindings-v1", &replay.repeat_observations)?,
        chaoscontrol_smr::replay_comparison_ref(&comparison).map_err(|_| super::ImportIssue::Replay)?,
    ];
    Ok(refs)
}
