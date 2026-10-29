pub(crate) fn plan_denied_outcome(
    request: &super::shell::ExecutionRequest<'_>,
    plan: &super::super::admission::ExecutionPlan,
    denial: super::super::model::Denial,
    mut receipts: Vec<super::super::evidence::receipt::ExecutionRecord>,
    input_ref: Option<String>,
) -> super::super::model::Result<super::shell::ExecutionOutcome> {
    let parent_refs = receipts.last().map(|receipt| vec![receipt.receipt_ref.clone()]).unwrap_or_default();
    let denial_class = denial.canonical_class().to_string();
    let denial_receipt = super::super::evidence::receipt::build(super::super::evidence::receipt::Input {
        stage: super::super::evidence::receipt::Stage::Denial,
        decision: super::super::evidence::receipt::Decision::Deny,
        evidence_scope: request.evidence_scope,
        consumer: plan.materialization.consumer,
        component_ref: plan.component_ref.clone(),
        wit_ref: plan.wit_ref.clone(),
        profile_ref: plan.profile_ref.clone(),
        runtime_configuration_ref: plan.runtime_configuration_ref.clone(),
        bundle_ref: plan.bundle_ref.clone(),
        imports: plan.imports.clone(),
        capabilities: plan.capabilities.clone(),
        mantle_evidence_refs: plan.mantle_evidence_refs.clone(),
        valence_evidence_refs: plan.valence_evidence_refs.clone(),
        cairn_evidence_refs: plan.cairn_evidence_refs.clone(),
        policy_refs: plan.policy_refs.clone(),
        authority_refs: plan.authority_refs.clone(),
        resource_refs: plan.resource_refs.clone(),
        recorded_effect_refs: plan.recorded_effect_refs.clone(),
        input_ref,
        output_ref: None,
        fuel_limit: Some(request.profile.resources.fuel),
        fuel_remaining: None,
        trap_class: Some(denial_class.clone()),
        parent_refs,
        diagnostics: vec![denial_class],
    })?;
    receipts.push(denial_receipt);
    Ok(super::shell::ExecutionOutcome {
        decision: super::super::evidence::receipt::Decision::Deny,
        output: None,
        receipts,
        diagnostics: denial.blockers,
    })
}

pub(crate) fn denied_outcome(
    request: &super::shell::ExecutionRequest<'_>,
    denial: super::super::model::Denial,
) -> super::shell::ExecutionOutcome {
    let component =
        super::super::evidence::materialization::MaterializedObjectIdentity::measure(request.source.component_bytes());
    let wit = super::super::evidence::materialization::MaterializedObjectIdentity::measure(request.source.wit_bytes());
    let component_ref =
        component.map_or_else(|_| super::super::model::content_ref(&[]), |identity| identity.content_ref);
    let wit_ref = wit.map_or_else(|_| super::super::model::content_ref(&[]), |identity| identity.content_ref);
    let context = context(request.source);
    let denial_class = denial.canonical_class().to_string();
    let profile_ref = super::super::profile::r#ref(request.profile);
    let runtime_configuration_ref =
        super::super::model::content_ref(format!("denied:{profile_ref}:{component_ref}:{wit_ref}").as_bytes());
    let receipt =
        super::provenance::bind_configuration_ref(&runtime_configuration_ref).and_then(|runtime_configuration_ref| {
            super::super::evidence::receipt::build(super::super::evidence::receipt::Input {
                stage: super::super::evidence::receipt::Stage::Denial,
                decision: super::super::evidence::receipt::Decision::Deny,
                evidence_scope: request.evidence_scope,
                consumer: context.consumer,
                component_ref,
                wit_ref,
                profile_ref,
                runtime_configuration_ref,
                bundle_ref: context.bundle_ref,
                imports: super::super::model::sorted_unique(&request.facts.imports),
                capabilities: Vec::new(),
                mantle_evidence_refs: context.mantle_evidence_refs,
                valence_evidence_refs: context.valence_evidence_refs,
                cairn_evidence_refs: context.cairn_evidence_refs,
                policy_refs: context.policy_refs,
                authority_refs: context.authority_refs,
                resource_refs: context.resource_refs,
                recorded_effect_refs: Vec::new(),
                input_ref: None,
                output_ref: None,
                fuel_limit: None,
                fuel_remaining: None,
                trap_class: Some(denial_class.clone()),
                parent_refs: Vec::new(),
                diagnostics: vec![denial_class],
            })
        });
    super::shell::ExecutionOutcome {
        decision: super::super::evidence::receipt::Decision::Deny,
        output: None,
        receipts: receipt.into_iter().collect(),
        diagnostics: denial.blockers,
    }
}

/// Deny a post-execution boundary without erasing the execution that occurred.
// r[impl molten.wasm_component_adoption.consumers]
pub(crate) fn deny_completed_execution(
    mut outcome: super::shell::ExecutionOutcome,
    denial: super::super::model::Denial,
) -> super::shell::ExecutionOutcome {
    outcome.decision = super::super::evidence::receipt::Decision::Deny;
    outcome.output = None;
    let denial_class = denial.canonical_class().to_string();
    outcome.diagnostics = denial.blockers;
    let Some(previous) = outcome.receipts.last() else {
        outcome.diagnostics.push("completed execution has no receipt to preserve".to_string());
        return outcome;
    };
    let mut input = previous.input.clone();
    input.stage = super::super::evidence::receipt::Stage::Denial;
    input.decision = super::super::evidence::receipt::Decision::Deny;
    input.output_ref = None;
    input.trap_class = Some(denial_class.clone());
    input.parent_refs = vec![previous.receipt_ref.clone()];
    input.diagnostics = vec![denial_class];
    match super::super::evidence::receipt::build(input) {
        Ok(receipt) => outcome.receipts.push(receipt),
        Err(failure) => outcome.diagnostics.extend(failure.blockers),
    }
    outcome
}

struct Context {
    bundle_ref: Option<String>,
    consumer: super::super::model::Consumer,
    mantle_evidence_refs: Vec<String>,
    valence_evidence_refs: Vec<String>,
    cairn_evidence_refs: Vec<String>,
    policy_refs: Vec<String>,
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
}

fn context(source: super::super::evidence::materialization::ArtifactSource<'_>) -> Context {
    match source {
        super::super::evidence::materialization::ArtifactSource::Mantle { bundle, envelope, .. } => {
            let mut mantle_refs = vec![bundle.build_cohort_ref.clone(), bundle.octet_report_ref.clone()];
            mantle_refs.extend(bundle.stage_receipt_refs.clone());
            Context {
                bundle_ref: Some(super::super::evidence::materialization::mantle_bundle_ref(bundle)),
                consumer: bundle.consumer,
                mantle_evidence_refs: valid_refs(&mantle_refs),
                valence_evidence_refs: valid_refs(&envelope.valence_sidecar_refs),
                cairn_evidence_refs: valid_refs(&envelope.cairn_acceptance_refs),
                policy_refs: valid_refs(&envelope.policy_refs),
                authority_refs: valid_refs(&envelope.authority_refs),
                resource_refs: valid_refs(&envelope.resource_refs),
            }
        }
        super::super::evidence::materialization::ArtifactSource::PublishedMantle {
            materialization,
            envelope,
            consumer,
        } => Context {
            bundle_ref: Some(materialization.bundle_ref().to_string()),
            consumer,
            mantle_evidence_refs: valid_refs(materialization.mantle_evidence_refs()),
            valence_evidence_refs: valid_refs(&envelope.valence_sidecar_refs),
            cairn_evidence_refs: valid_refs(&envelope.cairn_acceptance_refs),
            policy_refs: valid_refs(&envelope.policy_refs),
            authority_refs: valid_refs(&envelope.authority_refs),
            resource_refs: valid_refs(&envelope.resource_refs),
        },
        super::super::evidence::materialization::ArtifactSource::TestOnlyLoose { .. } => Context {
            bundle_ref: None,
            consumer: super::super::model::Consumer::Actor,
            mantle_evidence_refs: Vec::new(),
            valence_evidence_refs: Vec::new(),
            cairn_evidence_refs: Vec::new(),
            policy_refs: Vec::new(),
            authority_refs: Vec::new(),
            resource_refs: Vec::new(),
        },
    }
}

fn valid_refs(values: &[String]) -> Vec<String> {
    super::super::model::sorted_unique(
        &values
            .iter()
            .filter(|value| super::super::model::valid_content_ref(value))
            .cloned()
            .collect::<Vec<_>>(),
    )
}
