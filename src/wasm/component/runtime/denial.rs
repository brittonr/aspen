use super::super::admission::ComponentExecutionPlan;
use super::super::evidence::materialization::ComponentArtifactSource;
use super::super::evidence::materialization::MaterializedObjectIdentity;
use super::super::evidence::materialization::mantle_bundle_ref;
use super::super::evidence::receipt::ComponentReceipt;
use super::super::evidence::receipt::ComponentReceiptDecision;
use super::super::evidence::receipt::ComponentReceiptInput;
use super::super::evidence::receipt::ComponentReceiptStage;
use super::super::evidence::receipt::build_component_receipt;
use super::super::model::ComponentConsumer;
use super::super::model::ComponentDenial;
use super::super::model::ComponentResult;
use super::super::model::content_ref;
use super::super::model::sorted_unique;
use super::super::model::valid_content_ref;
use super::super::profile::component_profile_ref;
use super::shell::ComponentExecutionOutcome;
use super::shell::ComponentExecutionRequest;

pub(crate) fn plan_denied_outcome(
    request: &ComponentExecutionRequest<'_>,
    plan: &ComponentExecutionPlan,
    denial: ComponentDenial,
    mut receipts: Vec<ComponentReceipt>,
    input_ref: Option<String>,
) -> ComponentResult<ComponentExecutionOutcome> {
    let parent_refs = receipts.last().map(|receipt| vec![receipt.receipt_ref.clone()]).unwrap_or_default();
    let denial_class = denial.canonical_class().to_string();
    let denial_receipt = build_component_receipt(ComponentReceiptInput {
        stage: ComponentReceiptStage::Denial,
        decision: ComponentReceiptDecision::Deny,
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
    Ok(ComponentExecutionOutcome {
        decision: ComponentReceiptDecision::Deny,
        output: None,
        receipts,
        diagnostics: denial.blockers,
    })
}

pub(crate) fn denied_outcome(
    request: &ComponentExecutionRequest<'_>,
    denial: ComponentDenial,
) -> ComponentExecutionOutcome {
    let component = MaterializedObjectIdentity::measure(request.source.component_bytes());
    let wit = MaterializedObjectIdentity::measure(request.source.wit_bytes());
    let component_ref = component.map_or_else(|_| content_ref(&[]), |identity| identity.content_ref);
    let wit_ref = wit.map_or_else(|_| content_ref(&[]), |identity| identity.content_ref);
    let context = denial_context(request.source);
    let denial_class = denial.canonical_class().to_string();
    let profile_ref = component_profile_ref(request.profile);
    let runtime_configuration_ref = content_ref(format!("denied:{profile_ref}:{component_ref}:{wit_ref}").as_bytes());
    let receipt = build_component_receipt(ComponentReceiptInput {
        stage: ComponentReceiptStage::Denial,
        decision: ComponentReceiptDecision::Deny,
        evidence_scope: request.evidence_scope,
        consumer: context.consumer,
        component_ref,
        wit_ref,
        profile_ref,
        runtime_configuration_ref,
        bundle_ref: context.bundle_ref,
        imports: sorted_unique(&request.facts.imports),
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
    });
    ComponentExecutionOutcome {
        decision: ComponentReceiptDecision::Deny,
        output: None,
        receipts: receipt.into_iter().collect(),
        diagnostics: denial.blockers,
    }
}

struct DenialContext {
    bundle_ref: Option<String>,
    consumer: ComponentConsumer,
    mantle_evidence_refs: Vec<String>,
    valence_evidence_refs: Vec<String>,
    cairn_evidence_refs: Vec<String>,
    policy_refs: Vec<String>,
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
}

fn denial_context(source: ComponentArtifactSource<'_>) -> DenialContext {
    match source {
        ComponentArtifactSource::Mantle { bundle, envelope, .. } => {
            let mut mantle_refs = vec![bundle.build_cohort_ref.clone(), bundle.octet_report_ref.clone()];
            mantle_refs.extend(bundle.stage_receipt_refs.clone());
            DenialContext {
                bundle_ref: Some(mantle_bundle_ref(bundle)),
                consumer: bundle.consumer,
                mantle_evidence_refs: valid_refs(&mantle_refs),
                valence_evidence_refs: valid_refs(&envelope.valence_sidecar_refs),
                cairn_evidence_refs: valid_refs(&envelope.cairn_acceptance_refs),
                policy_refs: valid_refs(&envelope.policy_refs),
                authority_refs: valid_refs(&envelope.authority_refs),
                resource_refs: valid_refs(&envelope.resource_refs),
            }
        }
        ComponentArtifactSource::TestOnlyLoose { .. } => DenialContext {
            bundle_ref: None,
            consumer: ComponentConsumer::Actor,
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
    sorted_unique(&values.iter().filter(|value| valid_content_ref(value)).cloned().collect::<Vec<_>>())
}
