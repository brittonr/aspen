use preserves::ValueImpl;

pub fn validate_actor_registry_evidence(
    suite: &super::model::Suite,
    observations: &[super::observed::Observation],
) -> crate::error::Result<()> {
    if !suite.actors_explicit {
        return Err(crate::error::Failure::invalid_harness(
            "missing explicit actor registry fixture; inferred actors cannot satisfy evidence gates",
        ));
    }
    let actor_ids = suite.actors.iter().map(|actor| actor.id.as_str()).collect::<std::collections::BTreeSet<_>>();
    for step in &suite.steps {
        for actor in actor_ids_for_step(step) {
            require_declared_actor(&actor_ids, actor, "suite step", None)?;
        }
    }
    for (position, observation) in observations.iter().enumerate() {
        for event in &observation.events {
            for actor in actor_ids_for_event(event)? {
                require_declared_actor(&actor_ids, &actor, "observation event", Some(position))?;
            }
        }
    }
    Ok(())
}

pub fn actor_ids_for_step(step: &crate::harness::nucleus::CoreStep) -> Vec<&str> {
    step.actor_ids()
}

fn actor_ids_for_event(event: &preserves::IOValue) -> crate::error::Result<Vec<String>> {
    if let Some(actors) = message_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = assertion_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = effect_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = decision_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = boundary_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = receipt_participants(event)? {
        return Ok(actors);
    }
    Ok(Vec::new())
}

fn message_participants(event: &preserves::IOValue) -> crate::error::Result<Option<Vec<String>>> {
    if let Some(message) = event.collect_simple_record("message-delivered", Some(3)) {
        return Ok(Some(vec![
            super::fields::required_string(&message[0], "message sender")?,
            super::fields::required_string(&message[1], "message recipient")?,
        ]));
    }
    if let Some(observe) = event.collect_simple_record("observe-registered", Some(2)) {
        return Ok(Some(vec![super::fields::required_string(&observe[0], "observer actor")?]));
    }
    Ok(None)
}

fn assertion_participants(event: &preserves::IOValue) -> crate::error::Result<Option<Vec<String>>> {
    if let Some(observed) = event.collect_simple_record("assertion-observed", Some(3)) {
        return Ok(Some(vec![
            super::fields::required_string(&observed[0], "assertion observer")?,
            super::fields::required_string(&observed[1], "assertion owner")?,
        ]));
    }
    if let Some(assertion) = event.collect_simple_record("assertion-committed", Some(2)) {
        return Ok(Some(vec![super::fields::required_string(&assertion[0], "assertion actor")?]));
    }
    if let Some(retraction) = event.collect_simple_record("assertion-retracted", Some(2)) {
        return Ok(Some(vec![super::fields::required_string(&retraction[0], "retraction actor")?]));
    }
    if let Some(observed) = event.collect_simple_record("assertion-retraction-observed", Some(3)) {
        return Ok(Some(vec![
            super::fields::required_string(&observed[0], "assertion retraction observer")?,
            super::fields::required_string(&observed[1], "assertion retraction owner")?,
        ]));
    }
    Ok(None)
}

fn effect_participants(event: &preserves::IOValue) -> crate::error::Result<Option<Vec<String>>> {
    if let Some(request) = event.collect_simple_record("effect-request", None) {
        let arity = request.fields_iter().count();
        if arity != 3 && arity != 4 {
            return Err(crate::error::Failure::invalid_harness(format!(
                "effect-request arity must be 3 or 4, got {arity}"
            )));
        }
        return Ok(Some(vec![super::fields::required_string(&request[1], "effect request actor")?]));
    }
    if let Some(response) = event.collect_simple_record("effect-response", None) {
        let arity = response.fields_iter().count();
        if arity != 4 && arity != 5 {
            return Err(crate::error::Failure::invalid_harness(format!(
                "effect-response arity must be 4 or 5, got {arity}"
            )));
        }
        return Ok(Some(vec![super::fields::required_string(&response[1], "effect response actor")?]));
    }
    if let Some(rollback) = event.collect_simple_record("turn-rolled-back", Some(2)) {
        return Ok(Some(vec![super::fields::required_string(&rollback[0], "rollback actor")?]));
    }
    Ok(None)
}

fn decision_participants(event: &preserves::IOValue) -> crate::error::Result<Option<Vec<String>>> {
    if event.collect_simple_record("admission-decision-v1", None).is_none() {
        return Ok(None);
    }
    let decision = super::vetting::parse_admission_decision_event(event)?;
    let mut actors = vec![decision.request.actor];
    if matches!(&decision.request.action, crate::runtime::AdmissionAction::Send)
        && let Some(target) = decision.request.target
    {
        actors.push(target);
    }
    Ok(Some(actors))
}

fn boundary_participants(event: &preserves::IOValue) -> crate::error::Result<Option<Vec<String>>> {
    if let Some(input) = event.collect_simple_record("actor-input-v1", Some(9)) {
        let actor_value = crate::preserves_rail::value_to_iovalue(&input[1]);
        let actor = super::fields::simple_record(&actor_value, "actor", 2)?;
        return Ok(Some(vec![super::fields::required_string(&actor[0], "actor input actor")?]));
    }
    if let Some(request) = event.collect_simple_record("hostcall-request-v1", None) {
        let arity = request.fields_iter().count();
        if arity != 9 && arity != 11 && arity != 15 {
            return Err(crate::error::Failure::invalid_harness(format!(
                "hostcall-request arity must be 9, 11, or 15, got {arity}"
            )));
        }
        let parsed_request = super::vetting::parse_admission_request(&request[4])?;
        let mut actors = vec![parsed_request.actor];
        if matches!(&parsed_request.action, crate::runtime::AdmissionAction::Send)
            && let Some(target) = parsed_request.target
        {
            actors.push(target);
        }
        return Ok(Some(actors));
    }
    if let Some(output) = event.collect_simple_record("actor-output-v1", Some(8)) {
        return Ok(Some(vec![super::fields::required_record_string(
            &output[1],
            "actor",
            "actor output actor",
        )?]));
    }
    Ok(None)
}

fn receipt_participants(event: &preserves::IOValue) -> crate::error::Result<Option<Vec<String>>> {
    if let Some(receipt) = event.collect_simple_record("steel-execution-receipt-v1", None) {
        return Ok(Some(vec![super::fields::required_record_string(
            &receipt[1],
            "actor",
            "Steel execution actor",
        )?]));
    }
    if let Some(receipt) = event.collect_simple_record("wasm-execution-receipt-v1", None) {
        return Ok(Some(vec![super::fields::required_record_string(
            &receipt[1],
            "actor",
            "Wasm execution actor",
        )?]));
    }
    Ok(None)
}

fn require_declared_actor(
    actor_ids: &std::collections::BTreeSet<&str>,
    actor: &str,
    context: &str,
    observation: Option<usize>,
) -> crate::error::Result<()> {
    if actor_ids.contains(actor) {
        return Ok(());
    }
    let location = observation.map_or_else(String::new, |position| format!(" at observation {position}"));
    Err(crate::error::Failure::invalid_harness(format!(
        "actor {actor} in {context}{location} is not declared in explicit actor registry"
    )))
}

pub(super) fn infer_actor_registry(steps: &[crate::harness::nucleus::CoreStep]) -> Vec<super::model::ActorDecl> {
    let mut ids = std::collections::BTreeSet::new();
    for step in steps {
        for actor in actor_ids_for_step(step) {
            ids.insert(actor.to_owned());
        }
    }
    ids.into_iter()
        .map(|id| super::model::ActorDecl {
            id,
            kind: super::model::ActorKind::Native,
            executor: None,
        })
        .collect()
}
