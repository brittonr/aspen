use preserves::ValueImpl;

pub fn validate_admission_evidence(
    suite: &super::model::Suite,
    observations: &[super::observed::Observation],
    capability_gate: &super::observed::CapabilityGateEvidence,
) -> crate::error::Result<()> {
    if observations.len() != suite.steps.len() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "admission evidence observation count {} does not match suite step count {}",
            observations.len(),
            suite.steps.len()
        )));
    }

    for (position, (step, observation)) in suite.steps.iter().zip(observations.iter()).enumerate() {
        if observation.events.is_empty() {
            return Err(crate::error::Failure::invalid_harness(format!(
                "missing admission decision at observation {position}"
            )));
        }
        if super::tape::event_boundary(&observation.events[0]) != super::observed::EventBoundary::PolicyDecision {
            return Err(crate::error::Failure::invalid_harness(format!(
                "missing admission decision at observation {position}; first event is not admission-decision-v1"
            )));
        }
        let mut decision_count = 0usize;
        for event in &observation.events {
            if super::tape::event_boundary(event) == super::observed::EventBoundary::PolicyDecision {
                decision_count += 1;
            }
        }
        if decision_count != 1 {
            return Err(crate::error::Failure::invalid_harness(format!(
                "duplicate admission decision at observation {position}: got {decision_count} decisions"
            )));
        }

        let recorded = parse_admission_decision_event(&observation.events[0])?;
        let expected_request = crate::harness::nucleus::AdmissionRequest::from_step(step);
        if recorded.request != expected_request {
            return Err(crate::error::Failure::invalid_harness(format!(
                "admission request mismatch at observation {position}"
            )));
        }
        let expected_authority = super::proofs::admission_authority_evidence(&suite.capabilities, &expected_request)?;
        let recorded_authority = recorded.authority.as_ref().ok_or_else(|| {
            crate::error::Failure::invalid_harness(format!(
                "missing capability authority evidence at observation {position}"
            ))
        })?;
        if recorded_authority != &expected_authority {
            return Err(crate::error::Failure::invalid_harness(format!(
                "capability authority mismatch at observation {position}"
            )));
        }
        validate_authority_binding(position, recorded_authority, capability_gate)?;
        let expected_decision = suite.policy.decide_with_capabilities(&suite.capabilities, &expected_request);
        if recorded.decision != expected_decision {
            return Err(crate::error::Failure::invalid_harness(format!(
                "admission decision mismatch at observation {position}"
            )));
        }
        if !recorded.decision.is_allowed() {
            validate_denied_observation_events(position, &observation.events[1..])?;
        }
    }
    Ok(())
}

/// The recorded authority must name the preflight capability and proofset, and every grant, derived
/// grant, and UCAN verification ref it carries must be bound by the capability gate.
fn validate_authority_binding(
    position: usize,
    recorded_authority: &super::observed::AdmissionAuthorityEvidence,
    capability_gate: &super::observed::CapabilityGateEvidence,
) -> crate::error::Result<()> {
    if recorded_authority.capability_ref != capability_gate.capability_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "capability authority preflight ref mismatch at observation {position}"
        )));
    }
    if recorded_authority.proofset_ref != capability_gate.proofset_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "capability authority proofset ref mismatch at observation {position}"
        )));
    }
    let preflight_grant_refs: &[String] = capability_gate.grant_refs.as_slice();
    if let Some(grant_ref) = recorded_authority.grant_ref.as_deref()
        && !preflight_grant_refs.iter().any(|preflight_ref| preflight_ref.as_str() == grant_ref)
    {
        return Err(crate::error::Failure::invalid_harness(format!(
            "capability grant ref at observation {position} is not bound by authority preflight"
        )));
    }
    for derived_grant_ref in &recorded_authority.derived_grant_refs {
        if !capability_gate.derived_grant_refs.as_slice().iter().any(|gate_ref| gate_ref == derived_grant_ref) {
            return Err(crate::error::Failure::invalid_harness(format!(
                "derived grant ref at observation {position} is not bound by capability gate"
            )));
        }
    }
    for verification_ref in &recorded_authority.ucan_verification_receipt_refs {
        if !capability_gate
            .ucan_verification_receipt_refs
            .as_slice()
            .iter()
            .any(|gate_ref| gate_ref == verification_ref)
        {
            return Err(crate::error::Failure::invalid_harness(format!(
                "UCAN verification receipt ref at observation {position} is not bound by capability gate"
            )));
        }
    }
    Ok(())
}

pub fn parse_admission_decision_event(
    value: &preserves::IOValue,
) -> crate::error::Result<super::observed::AdmissionDecisionEvent> {
    let admission = value
        .collect_simple_record("admission-decision-v1", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <admission-decision-v1 ...>"))?;
    let arity = admission.fields_iter().count();
    if arity != 3 && arity != 4 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected <admission-decision-v1 ...> with arity 3 or 4, got {arity}"
        )));
    }
    let schema = super::fields::required_string(&admission[0], "admission decision schema")?;
    if schema != crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported admission decision schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA
        )));
    }
    let request = parse_admission_request(&admission[1])?;
    let (authority, decision_index) = if arity == 4 {
        (Some(parse_admission_authority(&admission[2])?), 3)
    } else {
        (None, 2)
    };
    let decision = parse_admission_decision(&admission[decision_index])?;
    Ok(super::observed::AdmissionDecisionEvent {
        value: value.clone(),
        request,
        authority,
        decision,
    })
}

fn validate_denied_observation_events(position: usize, events: &[preserves::IOValue]) -> crate::error::Result<()> {
    let mut has_rollback_event = false;
    for event in events {
        match super::tape::event_boundary(event) {
            super::observed::EventBoundary::EffectRequest | super::observed::EventBoundary::EffectResponse => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "denied effect emitted effect request/response at observation {position}"
                )));
            }
            super::observed::EventBoundary::SteelExecution => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "denied turn emitted Steel execution evidence at observation {position}"
                )));
            }
            super::observed::EventBoundary::WasmExecution => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "denied turn emitted Wasm execution evidence at observation {position}"
                )));
            }
            super::observed::EventBoundary::PolicyDecision => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "duplicate admission decision at observation {position}"
                )));
            }
            super::observed::EventBoundary::ActorInput
            | super::observed::EventBoundary::HostcallRequest
            | super::observed::EventBoundary::HostcallDecision
            | super::observed::EventBoundary::RuntimePredicate
            | super::observed::EventBoundary::ActorOutput => {}
            super::observed::EventBoundary::Trace if is_turn_rolled_back(event) => {
                has_rollback_event = true;
            }
            super::observed::EventBoundary::Trace if is_turn_journal(event) => {}
            super::observed::EventBoundary::Trace => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "denied turn committed action or non-rollback trace at observation {position}"
                )));
            }
        }
    }
    if !has_rollback_event {
        return Err(crate::error::Failure::invalid_harness(format!(
            "denied turn missing rollback evidence at observation {position}"
        )));
    }
    Ok(())
}

fn is_turn_rolled_back(value: &preserves::IOValue) -> bool {
    value.collect_simple_record("turn-rolled-back", Some(2)).is_some()
}

pub(super) fn is_turn_journal(value: &preserves::IOValue) -> bool {
    value.collect_simple_record("turn-journal-v1", None).is_some()
}

pub(super) fn parse_admission_request(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<crate::harness::nucleus::AdmissionRequest> {
    let request_value = crate::preserves_rail::value_to_iovalue(value);
    let request = super::fields::simple_record(&request_value, "request", 5)?;
    Ok(crate::harness::nucleus::AdmissionRequest {
        actor: super::fields::required_string(&request[0], "admission request actor")?,
        action: super::roster::parse_admission_action(&super::fields::required_string(
            &request[1],
            "admission request action",
        )?)?,
        target: super::fields::optional_request_string(&request[2], "admission request target")?,
        value: super::fields::optional_request_runtime_value(&request[3], "admission request value")?,
        upper: super::fields::optional_request_u64(&request[4], "admission request upper")?,
    })
}

fn parse_admission_authority(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<super::observed::AdmissionAuthorityEvidence> {
    let authority_value = crate::preserves_rail::value_to_iovalue(value);
    let authority = super::fields::simple_record(&authority_value, "authority", 10)?;
    let source = super::fields::required_record_string(&authority[0], "source", "admission authority source")?;
    let capability_ref =
        super::fields::required_record_hash(&authority[1], "capability-ref", "admission authority capability ref")?;
    let is_authorized =
        super::fields::required_record_bool(&authority[2], "authorized", "admission authority authorized")?;
    let grant_ref = super::fields::optional_request_string(&authority[3], "admission authority grant ref")?;
    let request_ref =
        super::fields::required_record_hash(&authority[4], "request-ref", "admission authority request ref")?;
    let proofset_ref = super::fields::required_record_hash(
        &authority[5],
        "ucan-proofset-ref",
        "admission authority UCAN proofset ref",
    )?;
    let ucan_verification_receipt_refs = super::fields::required_record_hash_sequence(
        &authority[6],
        "ucan-verification-receipt-refs",
        "admission authority UCAN verification receipt refs",
    )?;
    let derived_grant_refs = super::fields::required_record_hash_sequence(
        &authority[7],
        "derived-grant-refs",
        "admission authority derived grant refs",
    )?;
    let basalt_enforcement_receipt_ref = super::fields::required_record_hash(
        &authority[8],
        "basalt-enforcement-receipt-ref",
        "admission authority Basalt enforcement receipt ref",
    )?;
    let basalt_enforcement_receipt_value = super::fields::required_record_iovalue(
        &authority[9],
        "basalt-enforcement-receipt",
        "admission authority Basalt enforcement receipt",
    )?;
    let actual_receipt_ref = crate::preserves_rail::canonical_hash(&basalt_enforcement_receipt_value)?;
    if actual_receipt_ref != basalt_enforcement_receipt_ref {
        return Err(crate::error::Failure::invalid_harness(
            "admission authority Basalt enforcement receipt ref does not match embedded receipt",
        ));
    }
    Ok(super::observed::AdmissionAuthorityEvidence {
        source,
        capability_ref,
        authorized: is_authorized,
        grant_ref,
        request_ref,
        proofset_ref,
        ucan_verification_receipt_refs,
        derived_grant_refs,
        basalt_enforcement_receipt_ref,
        basalt_enforcement_receipt_value,
    })
}

fn parse_admission_decision(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<crate::runtime::AdmissionDecision> {
    let decision_value = crate::preserves_rail::value_to_iovalue(value);
    let decision = super::fields::simple_record(&decision_value, "decision", 2)?;
    let status = super::fields::required_string(&decision[0], "admission decision status")?;
    let reason = super::fields::required_string(&decision[1], "admission decision reason")?;
    match status.as_str() {
        "allow" => Ok(crate::runtime::AdmissionDecision::Allow { reason }),
        "deny" => Ok(crate::runtime::AdmissionDecision::Deny { reason }),
        other => Err(crate::error::Failure::invalid_harness(format!("unknown admission decision status {other}"))),
    }
}
