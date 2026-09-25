pub struct ComponentExecutionRequest<'a> {
    pub profile: &'a super::super::model::ComponentRuntimeProfile,
    pub requested_profile: super::super::model::RequestedExecutionProfile,
    pub evidence_scope: super::super::model::EvidenceScope,
    pub source: super::super::evidence::materialization::ComponentArtifactSource<'a>,
    pub facts: &'a super::super::admission::ComponentArtifactFacts,
    pub import_manifest: &'a super::super::imports::surface::DeclaredManifest,
    pub import_grants: &'a [super::super::admission::ComponentImportGrant],
    pub input: &'a preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentExecutionOutcome {
    pub decision: super::super::evidence::receipt::ComponentReceiptDecision,
    pub output: Option<preserves::IOValue>,
    pub receipts: Vec<super::super::evidence::receipt::ComponentReceipt>,
    pub diagnostics: Vec<String>,
}

impl ComponentExecutionOutcome {
    pub fn is_pass(&self) -> bool {
        self.decision == super::super::evidence::receipt::ComponentReceiptDecision::Pass
    }
}

pub fn execute_component(request: &ComponentExecutionRequest<'_>) -> ComponentExecutionOutcome {
    match execute_component_inner(request) {
        Ok(outcome) => outcome,
        Err(denial) => super::denial::denied_outcome(request, denial),
    }
}

fn execute_component_inner(
    request: &ComponentExecutionRequest<'_>,
) -> super::super::model::ComponentResult<ComponentExecutionOutcome> {
    super::super::profile::validate_component_profile(request.profile)?;
    super::super::migration::classify_for_profile(request.requested_profile, request.source.component_bytes())?;
    let materialization = super::super::evidence::materialization::verify_materialization(
        request.profile,
        request.evidence_scope,
        request.source,
    )?;
    super::super::admission::inspection::verify_component_artifact_facts(
        request.source.component_bytes(),
        request.facts,
    )?;
    admit_observed_import_surface(request, &materialization)?;
    let plan = super::super::admission::plan_component_execution(
        request.profile,
        materialization,
        request.import_manifest,
        request.facts,
        request.import_grants,
    )?;
    let input_bytes = crate::preserves_rail::canonical_bytes(request.input).map_err(|error| {
        super::super::model::ComponentDenial::classified(
            super::super::model::ComponentDenialClass::InvalidPreservesPayload,
            format!("component input is not canonical Preserves: {error}"),
        )
    })?;
    let input_length = u64::try_from(input_bytes.len()).map_err(|error| {
        super::super::model::ComponentDenial::classified(
            super::super::model::ComponentDenialClass::ResourceDenial,
            format!("component input length is unsupported: {error}"),
        )
    })?;
    if input_length > request.profile.resources.max_hostcall_bytes {
        return Err(super::super::model::ComponentDenial::classified(
            super::super::model::ComponentDenialClass::ResourceDenial,
            "component input exceeds the admitted canonical payload bound",
        ));
    }
    let input_ref = crate::preserves_rail::canonical_hash(request.input).map_err(|error| {
        super::super::model::ComponentDenial::new(format!("component input identity failed: {error}"))
    })?;
    execute_admitted_component(request, &plan, &input_bytes, input_ref)
}

fn execute_admitted_component(
    request: &ComponentExecutionRequest<'_>,
    plan: &super::super::admission::ComponentExecutionPlan,
    input_bytes: &[u8],
    input_ref: String,
) -> super::super::model::ComponentResult<ComponentExecutionOutcome> {
    let inspection = plan_receipt(request, PlanReceiptInput {
        plan,
        stage: super::super::evidence::receipt::ComponentReceiptStage::Inspection,
        input_ref: None,
        output_ref: None,
        fuel_limit: None,
        fuel_remaining: None,
        parent_refs: Vec::new(),
    })?;
    let mut session =
        match super::instantiate_component(request.profile, request.source.component_bytes(), request.facts) {
            Ok(session) => session,
            Err(denial) => {
                return super::denial::plan_denied_outcome(request, plan, denial, vec![inspection], Some(input_ref));
            }
        };
    let instantiation = plan_receipt(request, PlanReceiptInput {
        plan,
        stage: super::super::evidence::receipt::ComponentReceiptStage::Instantiation,
        input_ref: None,
        output_ref: None,
        fuel_limit: None,
        fuel_remaining: None,
        parent_refs: vec![inspection.receipt_ref.clone()],
    })?;
    let runtime = match super::invoke_component(&mut session, input_bytes) {
        Ok(runtime) => runtime,
        Err(denial) => {
            return super::denial::plan_denied_outcome(
                request,
                plan,
                denial,
                vec![inspection, instantiation],
                Some(input_ref),
            );
        }
    };
    let (output, output_ref) = match decode_component_output(request.profile, &runtime.output_bytes) {
        Ok(output) => output,
        Err(denial) => {
            return super::denial::plan_denied_outcome(
                request,
                plan,
                denial,
                vec![inspection, instantiation],
                Some(input_ref),
            );
        }
    };
    let execution = plan_receipt(request, PlanReceiptInput {
        plan,
        stage: super::super::evidence::receipt::ComponentReceiptStage::Execution,
        input_ref: Some(input_ref),
        output_ref: Some(output_ref),
        fuel_limit: Some(request.profile.resources.fuel),
        fuel_remaining: Some(runtime.fuel_remaining),
        parent_refs: vec![instantiation.receipt_ref.clone()],
    })?;
    Ok(ComponentExecutionOutcome {
        decision: super::super::evidence::receipt::ComponentReceiptDecision::Pass,
        output: Some(output),
        receipts: vec![inspection, instantiation, execution],
        diagnostics: Vec::new(),
    })
}

fn decode_component_output(
    profile: &super::super::model::ComponentRuntimeProfile,
    output_bytes: &[u8],
) -> super::super::model::ComponentResult<(preserves::IOValue, String)> {
    let output_length = u64::try_from(output_bytes.len()).map_err(|error| {
        super::super::model::ComponentDenial::classified(
            super::super::model::ComponentDenialClass::ResourceDenial,
            format!("component output length is unsupported: {error}"),
        )
    })?;
    if output_length > profile.resources.max_result_bytes {
        return Err(super::super::model::ComponentDenial::classified(
            super::super::model::ComponentDenialClass::ResourceDenial,
            "component output exceeds the admitted canonical result bound",
        ));
    }
    let output = crate::preserves_rail::parse_canonical_bytes(output_bytes).map_err(|error| {
        super::super::model::ComponentDenial::classified(
            super::super::model::ComponentDenialClass::InvalidPreservesPayload,
            format!("component output is not canonical Preserves: {error}"),
        )
    })?;
    let output_ref = crate::preserves_rail::canonical_hash(&output).map_err(|error| {
        super::super::model::ComponentDenial::new(format!("component output identity failed: {error}"))
    })?;
    Ok((output, output_ref))
}

struct PlanReceiptInput<'a> {
    plan: &'a super::super::admission::ComponentExecutionPlan,
    stage: super::super::evidence::receipt::ComponentReceiptStage,
    input_ref: Option<String>,
    output_ref: Option<String>,
    fuel_limit: Option<u64>,
    fuel_remaining: Option<u64>,
    parent_refs: Vec<String>,
}

fn plan_receipt(
    request: &ComponentExecutionRequest<'_>,
    input: PlanReceiptInput<'_>,
) -> super::super::model::ComponentResult<super::super::evidence::receipt::ComponentReceipt> {
    let PlanReceiptInput {
        plan,
        stage,
        input_ref,
        output_ref,
        fuel_limit,
        fuel_remaining,
        parent_refs,
    } = input;
    super::super::evidence::receipt::build_component_receipt(super::super::evidence::receipt::ComponentReceiptInput {
        stage,
        decision: super::super::evidence::receipt::ComponentReceiptDecision::Pass,
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

fn admit_observed_import_surface(
    request: &ComponentExecutionRequest<'_>,
    materialization: &super::super::evidence::materialization::MaterializationAdmission,
) -> super::super::model::ComponentResult<()> {
    let world = super::super::imports::surface::declared_world(request.profile);
    let observation = super::super::imports::observation::observe_artifact_surface(
        &request.facts.declared_world,
        &materialization.component_ref,
        request.source.component_bytes(),
    )?;
    let admission = super::super::imports::surface::admit_declared(request.import_manifest, &world, &observation)?;
    if !admission.is_admitted {
        return Err(super::super::model::ComponentDenial::from_blockers(admission.blockers));
    }
    Ok(())
}
