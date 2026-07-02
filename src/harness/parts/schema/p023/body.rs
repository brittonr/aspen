
fn effect_participants(event: &IoValue) -> Result<Option<Vec<String>>> {
    if let Some(request) = event.collect_simple_record("effect-request", None) {
        let arity = request.fields_iter().count();
        if arity != 3 && arity != 4 {
            return Err(MoltenError::invalid_harness(format!("effect-request arity must be 3 or 4, got {arity}")));
        }
        return Ok(Some(vec![required_string(&request[1], "effect request actor")?]));
    }
    if let Some(response) = event.collect_simple_record("effect-response", None) {
        let arity = response.fields_iter().count();
        if arity != 4 && arity != 5 {
            return Err(MoltenError::invalid_harness(format!("effect-response arity must be 4 or 5, got {arity}")));
        }
        return Ok(Some(vec![required_string(&response[1], "effect response actor")?]));
    }
    if let Some(rollback) = event.collect_simple_record("turn-rolled-back", Some(2)) {
        return Ok(Some(vec![required_string(&rollback[0], "rollback actor")?]));
    }
    Ok(None)
}

fn decision_participants(event: &IoValue) -> Result<Option<Vec<String>>> {
    if event.collect_simple_record("admission-decision-v1", None).is_none() {
        return Ok(None);
    }
    let decision = parse_admission_decision_event(event)?;
    let mut actors = vec![decision.request.actor];
    if matches!(&decision.request.action, crate::runtime::AdmissionAction::Send)
        && let Some(target) = decision.request.target
    {
        actors.push(target);
    }
    Ok(Some(actors))
}

fn boundary_participants(event: &IoValue) -> Result<Option<Vec<String>>> {
    if let Some(input) = event.collect_simple_record("actor-input-v1", Some(9)) {
        let actor_value = value_to_iovalue(&input[1]);
        let actor = simple_record(&actor_value, "actor", 2)?;
        return Ok(Some(vec![required_string(&actor[0], "actor input actor")?]));
    }
    if let Some(request) = event.collect_simple_record("hostcall-request-v1", None) {
        let arity = request.fields_iter().count();
        if arity != 9 && arity != 11 && arity != 15 {
            return Err(MoltenError::invalid_harness(format!(
                "hostcall-request arity must be 9, 11, or 15, got {arity}"
            )));
        }
        let parsed_request = parse_admission_request(&request[4])?;
        let mut actors = vec![parsed_request.actor];
        if matches!(&parsed_request.action, crate::runtime::AdmissionAction::Send)
            && let Some(target) = parsed_request.target
        {
            actors.push(target);
        }
        return Ok(Some(actors));
    }
    if let Some(output) = event.collect_simple_record("actor-output-v1", Some(8)) {
        return Ok(Some(vec![required_record_string(&output[1], "actor", "actor output actor")?]));
    }
    Ok(None)
}

fn receipt_participants(event: &IoValue) -> Result<Option<Vec<String>>> {
    if let Some(receipt) = event.collect_simple_record("steel-execution-receipt-v1", None) {
        return Ok(Some(vec![required_record_string(&receipt[1], "actor", "Steel execution actor")?]));
    }
    if let Some(receipt) = event.collect_simple_record("wasm-execution-receipt-v1", None) {
        return Ok(Some(vec![required_record_string(&receipt[1], "actor", "Wasm execution actor")?]));
    }
    Ok(None)
}

fn require_declared_actor(
    actor_ids: &OrderedSet<&str>,
    actor: &str,
    context: &str,
    observation: Option<usize>,
) -> Result<()> {
    if actor_ids.contains(actor) {
        return Ok(());
    }
    let location = observation.map_or_else(String::new, |position| format!(" at observation {position}"));
    Err(MoltenError::invalid_harness(format!(
        "actor {actor} in {context}{location} is not declared in explicit actor registry"
    )))
}

fn infer_actor_registry(steps: &[super::core::CoreStep]) -> Vec<ActorDecl> {
    let mut ids = OrderedSet::new();
    for step in steps {
        for actor in actor_ids_for_step(step) {
            ids.insert(actor.to_owned());
        }
    }
    ids.into_iter()
        .map(|id| ActorDecl {
            id,
            kind: ActorKind::Native,
            executor: None,
        })
        .collect()
}

fn parse_admission_action(action: &str) -> Result<crate::runtime::AdmissionAction> {
    match action {
        "send" => Ok(crate::runtime::AdmissionAction::Send),
        "observe" => Ok(crate::runtime::AdmissionAction::Observe),
        "assert" => Ok(crate::runtime::AdmissionAction::Assert),
        "retract" => Ok(crate::runtime::AdmissionAction::Retract),
        "clock" => Ok(crate::runtime::AdmissionAction::Clock),
        "random" => Ok(crate::runtime::AdmissionAction::Random),
        other => Err(MoltenError::invalid_harness(format!("unknown admission action {other}"))),
    }
}

fn parse_actor_kind(kind: &str) -> Result<ActorKind> {
    match kind {
        "native" => Ok(ActorKind::Native),
        "steel" => Ok(ActorKind::Steel),
        "wasm" => Ok(ActorKind::Wasm),
        "adapter" => Ok(ActorKind::Adapter),
        "remote-proxy" => Ok(ActorKind::RemoteProxy),
        other => Err(MoltenError::invalid_harness(format!("unknown actor kind {other}"))),
    }
}

fn parse_legacy_report_repro_bundle(bundle_value: &IoValue, bundle: &Record<Value<IoValue>>) -> Result<ReproBundle> {
    let report_ref = required_string(&bundle[1], "repro bundle report ref")?;
    let suite_ref = required_string(&bundle[2], "repro bundle suite ref")?;
    let initial_state_hash = required_hash(&bundle[3], "repro bundle initial state hash")?;
    let final_state_hash = required_hash(&bundle[4], "repro bundle final state hash")?;
    let replay_status = required_string(&bundle[5], "repro bundle replay status")?;
    let profile = required_string(&bundle[6], "repro bundle profile")?;
    let actors = parse_actor_registry(&value_to_iovalue(&bundle[7]))?;
    let effect_log = parse_effect_log(&value_to_iovalue(&bundle[8]))?;
    let suite_value = value_to_iovalue(&bundle[9]);
    let report_value = value_to_iovalue(&bundle[10]);
    let report = parse_report(&report_value)?;
    require_repro_report_matches(&ReproReportMatchInput {
        report: &report,
        report_ref: &report_ref,
        suite_ref: &suite_ref,
        initial_state_hash: &initial_state_hash,
        final_state_hash: &final_state_hash,
        replay_status: &replay_status,
        profile: &profile,
        actors: &actors,
        effect_log: &effect_log,
        suite_value: &suite_value,
    })?;
    Ok(ReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: ReproBundleKind::Report,
        artifact_ref: report_ref,
        report_value: Some(report_value),
        failure_value: None,
        gate_receipt_ref: None,
        receipt_value: None,
        redaction_policy_ref: None,
        redaction_gate_ref: None,
        export_profile: None,
        export_profile_ref: None,
        export_profile_value: None,
        source_report_ref: None,
        source_suite_ref: None,
        redaction_transform_manifest_ref: None,
        redaction_transform_manifest_value: None,
        redaction_transform_receipt_ref: None,
        redaction_transform_receipt_value: None,
        private_bundle_profile_ref: None,
        private_bundle_profile_value: None,
        loss_classification: None,
        encrypted_refs: Vec::new(),
    })
}

struct ReportBundleBody {
    artifact_refs: Vec<(String, String)>,
    report_ref: String,
    suite_ref: String,
    replay_status: String,
    profile: String,
    report_value: IoValue,
    report: Report,
}

struct SealedRedaction {
    redaction_policy_ref: Option<String>,
    redaction_gate_ref: Option<String>,
    seal_index: usize,
    receipt_index: usize,
    checks_index: usize,
    has_redaction: bool,
}

fn parse_report_repro_bundle(bundle_value: &IoValue, bundle: &Record<Value<IoValue>>) -> Result<ReproBundle> {
    let body = parse_report_body(bundle)?;
    report_bundle_from_body(bundle_value, body)
}

fn parse_report_body(bundle: &Record<Value<IoValue>>) -> Result<ReportBundleBody> {
    let kind = required_record_string(&bundle[1], "bundle-kind", "repro bundle kind")?;
    if kind != "report" {
        return Err(MoltenError::invalid_harness(format!("expected report repro bundle kind, got {kind}")));
    }
    validate_tool_record(&bundle[2])?;
    validate_sequence_record(&bundle[3], "command", "repro bundle command")?;
    validate_sequence_record(&bundle[4], "replay-instructions", "repro bundle replay instructions")?;
    let artifact_refs = parse_artifact_refs(&bundle[5])?;
    let report_ref = required_string(&bundle[6], "repro bundle report ref")?;
    let suite_ref = required_string(&bundle[7], "repro bundle suite ref")?;
    require_artifact_ref(&artifact_refs, "report", &report_ref)?;
    require_artifact_ref(&artifact_refs, "suite", &suite_ref)?;
    let initial_state_hash = required_hash(&bundle[8], "repro bundle initial state hash")?;
    let final_state_hash = required_hash(&bundle[9], "repro bundle final state hash")?;
    let replay_status = required_string(&bundle[10], "repro bundle replay status")?;
    let profile = required_string(&bundle[11], "repro bundle profile")?;
    let actors = parse_actor_registry(&value_to_iovalue(&bundle[12]))?;
    let effect_log = parse_effect_log(&value_to_iovalue(&bundle[13]))?;
    let suite_value = value_to_iovalue(&bundle[14]);
    let report_value = value_to_iovalue(&bundle[15]);
    let report = parse_report(&report_value)?;
    require_repro_report_matches(&ReproReportMatchInput {
        report: &report,
        report_ref: &report_ref,
        suite_ref: &suite_ref,
        initial_state_hash: &initial_state_hash,
        final_state_hash: &final_state_hash,
        replay_status: &replay_status,
        profile: &profile,
        actors: &actors,
        effect_log: &effect_log,
        suite_value: &suite_value,
    })?;
    Ok(ReportBundleBody {
        artifact_refs,
        report_ref,
        suite_ref,
        replay_status,
        profile,
        report_value,
        report,
    })
}

fn report_bundle_from_body(bundle_value: &IoValue, body: ReportBundleBody) -> Result<ReproBundle> {
    Ok(ReproBundle {
        bundle_ref: canonical_hash(bundle_value)?,
        kind: ReproBundleKind::Report,
        artifact_ref: body.report_ref,
        report_value: Some(body.report_value),
        failure_value: None,
        gate_receipt_ref: None,
        receipt_value: None,
        redaction_policy_ref: None,
        redaction_gate_ref: None,
        export_profile: None,
        export_profile_ref: None,
        export_profile_value: None,
        source_report_ref: None,
        source_suite_ref: None,
        redaction_transform_manifest_ref: None,
        redaction_transform_manifest_value: None,
        redaction_transform_receipt_ref: None,
        redaction_transform_receipt_value: None,
        private_bundle_profile_ref: None,
        private_bundle_profile_value: None,
        loss_classification: None,
        encrypted_refs: Vec::new(),
    })
}

fn parse_sealed_report_repro_bundle(bundle_value: &IoValue, bundle: &Record<Value<IoValue>>) -> Result<ReproBundle> {
    let body = parse_report_body(bundle)?;
    require_report_artifact_refs(&body.artifact_refs, &body.report)?;
    let arity = bundle.fields_iter().count();
    let redaction = parse_sealed_redaction(bundle, &body, arity)?;
    let seal = parse_repro_seal(
        &bundle[redaction.seal_index],
        &body.report_ref,
        &body.suite_ref,
        &body.profile,
        &body.replay_status,
    )?;
    require_artifact_ref(&body.artifact_refs, "gate-receipt", &seal.gate_receipt_ref)?;
    let receipt_value = embedded_gate_receipt(bundle, redaction.receipt_index, &seal.gate_receipt_ref)?;
    let seal_checks = parse_seal_checks(&bundle[redaction.checks_index])?;
    require_report_seal_checks(&seal_checks, redaction.has_redaction)?;
    sealed_report_bundle(bundle_value, body, seal, receipt_value, redaction)
}
