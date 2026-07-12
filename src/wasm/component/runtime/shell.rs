use super::super::admission::ComponentArtifactFacts;
use super::super::admission::ComponentExecutionPlan;
use super::super::admission::ComponentImportGrant;
use super::super::admission::plan_component_execution;
use super::super::evidence::materialization::ComponentArtifactSource;
use super::super::evidence::materialization::verify_materialization;
use super::super::evidence::receipt::ComponentReceipt;
use super::super::evidence::receipt::ComponentReceiptDecision;
use super::super::evidence::receipt::ComponentReceiptInput;
use super::super::evidence::receipt::ComponentReceiptStage;
use super::super::evidence::receipt::build_component_receipt;
use super::super::migration::classify_for_profile;
use super::super::model::ComponentDenial;
use super::super::model::ComponentDenialClass;
use super::super::model::ComponentResult;
use super::super::model::ComponentRuntimeProfile;
use super::super::model::EvidenceScope;
use super::super::model::RequestedExecutionProfile;
use super::super::profile::validate_component_profile;
use super::denial::denied_outcome;
use super::denial::plan_denied_outcome;

pub struct ComponentExecutionRequest<'a> {
    pub profile: &'a ComponentRuntimeProfile,
    pub requested_profile: RequestedExecutionProfile,
    pub evidence_scope: EvidenceScope,
    pub source: ComponentArtifactSource<'a>,
    pub facts: &'a ComponentArtifactFacts,
    pub import_grants: &'a [ComponentImportGrant],
    pub input: &'a preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentExecutionOutcome {
    pub decision: ComponentReceiptDecision,
    pub output: Option<preserves::IOValue>,
    pub receipts: Vec<ComponentReceipt>,
    pub diagnostics: Vec<String>,
}

impl ComponentExecutionOutcome {
    pub fn is_pass(&self) -> bool {
        self.decision == ComponentReceiptDecision::Pass
    }
}

pub fn execute_component(request: &ComponentExecutionRequest<'_>) -> ComponentExecutionOutcome {
    match execute_component_inner(request) {
        Ok(outcome) => outcome,
        Err(denial) => denied_outcome(request, denial),
    }
}

fn execute_component_inner(request: &ComponentExecutionRequest<'_>) -> ComponentResult<ComponentExecutionOutcome> {
    validate_component_profile(request.profile)?;
    classify_for_profile(request.requested_profile, request.source.component_bytes())?;
    let materialization = verify_materialization(request.profile, request.evidence_scope, request.source)?;
    super::super::admission::inspection::verify_component_artifact_facts(
        request.source.component_bytes(),
        request.facts,
    )?;
    let plan = plan_component_execution(request.profile, materialization, request.facts, request.import_grants)?;
    let input_bytes = crate::preserves_rail::canonical_bytes(request.input).map_err(|error| {
        ComponentDenial::classified(
            ComponentDenialClass::InvalidPreservesPayload,
            format!("component input is not canonical Preserves: {error}"),
        )
    })?;
    let input_length = u64::try_from(input_bytes.len()).map_err(|error| {
        ComponentDenial::classified(
            ComponentDenialClass::ResourceDenial,
            format!("component input length is unsupported: {error}"),
        )
    })?;
    if input_length > request.profile.resources.max_hostcall_bytes {
        return Err(ComponentDenial::classified(
            ComponentDenialClass::ResourceDenial,
            "component input exceeds the admitted canonical payload bound",
        ));
    }
    let input_ref = crate::preserves_rail::canonical_hash(request.input)
        .map_err(|error| ComponentDenial::new(format!("component input identity failed: {error}")))?;
    execute_admitted_component(request, &plan, &input_bytes, input_ref)
}

fn execute_admitted_component(
    request: &ComponentExecutionRequest<'_>,
    plan: &ComponentExecutionPlan,
    input_bytes: &[u8],
    input_ref: String,
) -> ComponentResult<ComponentExecutionOutcome> {
    let inspection =
        plan_receipt(request, plan, ComponentReceiptStage::Inspection, None, None, None, None, Vec::new())?;
    let mut session =
        match super::instantiate_component(request.profile, request.source.component_bytes(), request.facts) {
            Ok(session) => session,
            Err(denial) => return plan_denied_outcome(request, plan, denial, vec![inspection], Some(input_ref)),
        };
    let instantiation =
        plan_receipt(request, plan, ComponentReceiptStage::Instantiation, None, None, None, None, vec![
            inspection.receipt_ref.clone(),
        ])?;
    let runtime = match super::invoke_component(&mut session, input_bytes) {
        Ok(runtime) => runtime,
        Err(denial) => {
            return plan_denied_outcome(request, plan, denial, vec![inspection, instantiation], Some(input_ref));
        }
    };
    let (output, output_ref) = match decode_component_output(request.profile, &runtime.output_bytes) {
        Ok(output) => output,
        Err(denial) => {
            return plan_denied_outcome(request, plan, denial, vec![inspection, instantiation], Some(input_ref));
        }
    };
    let execution = plan_receipt(
        request,
        plan,
        ComponentReceiptStage::Execution,
        Some(input_ref),
        Some(output_ref),
        Some(request.profile.resources.fuel),
        Some(runtime.fuel_remaining),
        vec![instantiation.receipt_ref.clone()],
    )?;
    Ok(ComponentExecutionOutcome {
        decision: ComponentReceiptDecision::Pass,
        output: Some(output),
        receipts: vec![inspection, instantiation, execution],
        diagnostics: Vec::new(),
    })
}

fn decode_component_output(
    profile: &ComponentRuntimeProfile,
    output_bytes: &[u8],
) -> ComponentResult<(preserves::IOValue, String)> {
    let output_length = u64::try_from(output_bytes.len()).map_err(|error| {
        ComponentDenial::classified(
            ComponentDenialClass::ResourceDenial,
            format!("component output length is unsupported: {error}"),
        )
    })?;
    if output_length > profile.resources.max_result_bytes {
        return Err(ComponentDenial::classified(
            ComponentDenialClass::ResourceDenial,
            "component output exceeds the admitted canonical result bound",
        ));
    }
    let output = crate::preserves_rail::parse_canonical_bytes(output_bytes).map_err(|error| {
        ComponentDenial::classified(
            ComponentDenialClass::InvalidPreservesPayload,
            format!("component output is not canonical Preserves: {error}"),
        )
    })?;
    let output_ref = crate::preserves_rail::canonical_hash(&output)
        .map_err(|error| ComponentDenial::new(format!("component output identity failed: {error}")))?;
    Ok((output, output_ref))
}

fn plan_receipt(
    request: &ComponentExecutionRequest<'_>,
    plan: &ComponentExecutionPlan,
    stage: ComponentReceiptStage,
    input_ref: Option<String>,
    output_ref: Option<String>,
    fuel_limit: Option<u64>,
    fuel_remaining: Option<u64>,
    parent_refs: Vec<String>,
) -> ComponentResult<ComponentReceipt> {
    build_component_receipt(ComponentReceiptInput {
        stage,
        decision: ComponentReceiptDecision::Pass,
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
        output_ref,
        fuel_limit,
        fuel_remaining,
        trap_class: None,
        parent_refs,
        diagnostics: Vec::new(),
    })
}
