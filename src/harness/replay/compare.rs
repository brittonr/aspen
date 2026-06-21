type ParsedReport = super::super::schema::HarnessReport;
type Observation = super::super::schema::HarnessObservation;
type Result<T> = crate::error::Result<T>;
type IoValue = preserves::IOValue;

pub(super) fn report_start(expected: &ParsedReport, actual: &ParsedReport) -> Result<()> {
    if expected.initial_state_hash != actual.initial_state_hash {
        return Err(super::divergence(
            "initial-state",
            None,
            expected.initial_state_hash.clone(),
            actual.initial_state_hash.clone(),
            "initial state hash differs",
        ));
    }
    if expected.observations.len() != actual.observations.len() {
        return Err(super::divergence(
            "trace-length",
            None,
            expected.observations.len().to_string(),
            actual.observations.len().to_string(),
            "observation count differs",
        ));
    }
    Ok(())
}

pub(super) fn observations(expected: &[Observation], actual: &[Observation]) -> Result<()> {
    for (expected_observation, actual_observation) in expected.iter().zip(actual.iter()) {
        observation_metadata(expected_observation, actual_observation)?;
        observation_events(expected_observation.index, &expected_observation.events, &actual_observation.events)?;
        observation_tail(expected_observation, actual_observation)?;
    }
    Ok(())
}

fn observation_metadata(expected: &Observation, actual: &Observation) -> Result<()> {
    if expected.step_ref != actual.step_ref {
        return Err(super::divergence(
            "input",
            Some(expected.index),
            expected.step_ref.clone(),
            actual.step_ref.clone(),
            "step input hash differs",
        ));
    }
    if expected.before_state_hash != actual.before_state_hash {
        return Err(super::divergence(
            "state-before",
            Some(expected.index),
            expected.before_state_hash.clone(),
            actual.before_state_hash.clone(),
            "before state hash differs",
        ));
    }
    Ok(())
}

fn observation_events(index: u64, expected: &[IoValue], actual: &[IoValue]) -> Result<()> {
    if expected.len() != actual.len() {
        return Err(super::divergence(
            "trace-length",
            Some(index),
            expected.len().to_string(),
            actual.len().to_string(),
            "event count differs",
        ));
    }
    for (expected_event, actual_event) in expected.iter().zip(actual.iter()) {
        event(index, expected_event, actual_event)?;
    }
    Ok(())
}

fn event(index: u64, expected_event: &IoValue, actual_event: &IoValue) -> Result<()> {
    let expected_hash = crate::preserves_rail::canonical_hash(expected_event)?;
    let actual_hash = crate::preserves_rail::canonical_hash(actual_event)?;
    if expected_hash != actual_hash {
        return Err(super::divergence(
            super::event_divergence_kind(expected_event, actual_event),
            Some(index),
            expected_hash,
            actual_hash,
            "event differs",
        ));
    }
    Ok(())
}

fn observation_tail(expected: &Observation, actual: &Observation) -> Result<()> {
    if expected.after_state_hash != actual.after_state_hash {
        return Err(super::divergence(
            "state-after",
            Some(expected.index),
            expected.after_state_hash.clone(),
            actual.after_state_hash.clone(),
            "after state hash differs",
        ));
    }
    let expected_hash = crate::preserves_rail::canonical_hash(&expected.value)?;
    let actual_hash = crate::preserves_rail::canonical_hash(&actual.value)?;
    if expected_hash != actual_hash {
        return Err(super::divergence(
            "trace",
            Some(expected.index),
            expected_hash,
            actual_hash,
            "turn observation metadata differs",
        ));
    }
    Ok(())
}

pub(super) fn report_end(expected: &ParsedReport, actual: &ParsedReport) -> Result<()> {
    if expected.final_state_hash != actual.final_state_hash {
        return Err(super::divergence(
            "final-state",
            None,
            expected.final_state_hash.clone(),
            actual.final_state_hash.clone(),
            "final state hash differs",
        ));
    }
    if expected.report_ref != actual.report_ref {
        return Err(super::divergence(
            "report",
            None,
            expected.report_ref.clone(),
            actual.report_ref.clone(),
            "report metadata differs after deterministic replay",
        ));
    }
    Ok(())
}
