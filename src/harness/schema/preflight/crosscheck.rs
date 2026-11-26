use preserves::ValueImpl;

pub fn validate_executor_preflight_evidence(
    suite: &super::model::Suite,
    observations: &[super::observed::Observation],
    preflights: Option<&super::observed::ExecutorPreflightsEvidence>,
) -> crate::error::Result<()> {
    let preflights =
        preflights.ok_or_else(|| crate::error::Failure::invalid_harness("missing executor preflight evidence"))?;
    let expected = super::readiness::executor_preflights_value(suite)?;
    if preflights.value != expected {
        return Err(crate::error::Failure::invalid_harness(format!(
            "executor preflight evidence mismatch: got {}, expected {}",
            crate::preserves_rail::canonical_hash(&preflights.value)?,
            crate::preserves_rail::canonical_hash(&expected)?
        )));
    }
    let by_actor = preflights_by_actor(&preflights.preflights)?;
    validate_actor_preflights(suite, &by_actor)?;
    validate_hostcall_preflight_bindings(observations, &by_actor)
}

type PreflightMap<'a> = std::collections::BTreeMap<&'a str, &'a super::observed::ExecutorPreflightEvidence>;

fn preflights_by_actor(
    preflights: &[super::observed::ExecutorPreflightEvidence],
) -> crate::error::Result<PreflightMap<'_>> {
    let mut by_actor = std::collections::BTreeMap::new();
    for preflight in preflights {
        if by_actor.insert(preflight.actor_id.as_str(), preflight).is_some() {
            return Err(crate::error::Failure::invalid_harness(format!(
                "duplicate executor preflight for actor {}",
                preflight.actor_id
            )));
        }
    }
    Ok(by_actor)
}

fn validate_actor_preflights(suite: &super::model::Suite, by_actor: &PreflightMap<'_>) -> crate::error::Result<()> {
    for actor in &suite.actors {
        let Some(preflight) = by_actor.get(actor.id.as_str()) else {
            return Err(crate::error::Failure::invalid_harness(format!(
                "missing executor preflight for actor {}",
                actor.id
            )));
        };
        if preflight.kind != actor.kind {
            return Err(crate::error::Failure::invalid_harness(format!(
                "executor kind binding mismatch for actor {}",
                actor.id
            )));
        }
        validate_actor_executor_preflight(actor, preflight)?;
    }
    Ok(())
}

fn validate_hostcall_preflight_bindings(
    observations: &[super::observed::Observation],
    by_actor: &PreflightMap<'_>,
) -> crate::error::Result<()> {
    for (position, observation) in observations.iter().enumerate() {
        for event in &observation.events {
            validate_hostcall_preflight_event(position, event, by_actor)?;
        }
    }
    Ok(())
}

fn validate_hostcall_preflight_event(
    position: usize,
    event: &preserves::IOValue,
    by_actor: &PreflightMap<'_>,
) -> crate::error::Result<()> {
    let Some(request) = event.collect_simple_record("hostcall-request-v1", None) else {
        return Ok(());
    };
    let arity = request.fields_iter().count();
    if arity != 9 && arity != 11 && arity != 15 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "hostcall request at observation {position} must have arity 9, 11, or 15, got {arity}"
        )));
    }
    let admission_request = super::vetting::parse_admission_request(&request[4])?;
    let Some(preflight) = by_actor.get(admission_request.actor.as_str()) else {
        return Err(crate::error::Failure::invalid_harness(format!(
            "hostcall request at observation {position} has no executor preflight for actor {}",
            admission_request.actor
        )));
    };
    let allowed_hostcalls = preflight.allowed_hostcalls.as_slice();
    let operation = admission_request.action.as_str();
    if !allowed_hostcalls.iter().any(|allowed| allowed.as_str() == operation) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "hostcall operation {operation} at observation {position} is not allowed by executor preflight for actor {}",
            admission_request.actor
        )));
    }
    Ok(())
}

fn validate_actor_executor_preflight(
    actor: &super::model::ActorDecl,
    preflight: &super::observed::ExecutorPreflightEvidence,
) -> crate::error::Result<()> {
    let expected_conformance_refs = super::readiness::executor_conformance_refs(&preflight.allowed_hostcalls)?;
    if preflight.conformance_refs != expected_conformance_refs {
        return Err(crate::error::Failure::invalid_harness(format!(
            "executor conformance suite refs mismatch for actor {}",
            actor.id
        )));
    }
    match (&actor.kind, &actor.executor) {
        (super::model::ActorKind::Native, None) => super::variants::validate_native_preflight(actor, preflight),
        (super::model::ActorKind::Steel, Some(super::model::ActorExecutorConfig::Steel(config))) => {
            super::variants::validate_steel_preflight(actor, preflight, config)
        }
        (super::model::ActorKind::Wasm, Some(super::model::ActorExecutorConfig::Wasm(config))) => {
            super::variants::validate_wasm_preflight(actor, preflight, config)
        }
        (super::model::ActorKind::Adapter, Some(super::model::ActorExecutorConfig::Adapter(config))) => {
            super::variants::validate_adapter_preflight(actor, preflight, config)
        }
        (super::model::ActorKind::RemoteProxy, Some(super::model::ActorExecutorConfig::RemoteProxy(config))) => {
            super::variants::validate_remote_preflight(actor, preflight, config)
        }
        (super::model::ActorKind::Steel, None) => Err(crate::error::Failure::invalid_harness(format!(
            "steel actor {} missing reviewed Steel executor preflight fixture",
            actor.id
        ))),
        (super::model::ActorKind::Wasm, None) => Err(crate::error::Failure::invalid_harness(format!(
            "wasm actor {} missing Wasm executor preflight fixture",
            actor.id
        ))),
        (super::model::ActorKind::Adapter | super::model::ActorKind::RemoteProxy, _) => {
            Err(crate::error::Failure::invalid_harness(format!(
                "executor kind {} requires executor adapter preflight and remains disabled in local harness",
                actor.kind.as_str()
            )))
        }
        (super::model::ActorKind::Steel, Some(_)) | (super::model::ActorKind::Wasm, Some(_)) => {
            Err(crate::error::Failure::invalid_harness(format!(
                "actor {} kind {} has mismatched executor preflight fixture",
                actor.id,
                actor.kind.as_str()
            )))
        }
        (super::model::ActorKind::Native, Some(_)) => Err(crate::error::Failure::invalid_harness(format!(
            "native actor {} must not declare non-native executor preflight fixture",
            actor.id
        ))),
    }
}
