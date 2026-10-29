pub struct ExecutionRequest<'a> {
    pub profile: &'a super::super::model::RuntimeProfile,
    pub requested_profile: super::super::model::RequestedExecutionProfile,
    pub evidence_scope: super::super::model::EvidenceScope,
    pub source: super::super::evidence::materialization::ArtifactSource<'a>,
    pub facts: &'a super::super::admission::ArtifactFacts,
    pub import_manifest: &'a super::super::imports::surface::DeclaredManifest,
    pub import_grants: &'a [super::super::admission::ImportGrant],
    pub input: &'a preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutcome {
    pub decision: super::super::evidence::receipt::Decision,
    pub output: Option<preserves::IOValue>,
    pub receipts: Vec<super::super::evidence::receipt::ExecutionRecord>,
    pub diagnostics: Vec<String>,
}

impl ExecutionOutcome {
    pub fn is_pass(&self) -> bool {
        self.decision == super::super::evidence::receipt::Decision::Pass
    }
}

pub fn execute(request: &ExecutionRequest<'_>) -> ExecutionOutcome {
    match execute_inner(request) {
        Ok(outcome) => outcome,
        Err(denial) => super::denial::denied_outcome(request, denial),
    }
}

fn execute_inner(request: &ExecutionRequest<'_>) -> super::super::model::Result<ExecutionOutcome> {
    let admitted = admit_request(request, super::provenance::Backend::Local)?;
    execute_admitted(request, &admitted.plan, &admitted.input_bytes, admitted.input_ref)
}

/// A request that passed every admission boundary before execution.
pub(super) struct AdmittedRequest {
    pub(super) plan: super::super::admission::ExecutionPlan,
    pub(super) input_bytes: Vec<u8>,
    pub(super) input_ref: String,
}

/// Admit a request: profile, requested profile, migration, materialization,
/// artifact facts, import surface, plan, and canonical bounded input.
pub(super) fn admit_request(
    request: &ExecutionRequest<'_>,
    backend: super::provenance::Backend,
) -> super::super::model::Result<AdmittedRequest> {
    super::super::profile::validate_runtime(request.profile)?;
    super::super::slicing::admit_requested_profile(request.profile, request.requested_profile)?;
    super::super::migration::classify_for_profile(request.requested_profile, request.source.component_bytes())?;
    let materialization =
        super::super::evidence::materialization::verify(request.profile, request.evidence_scope, request.source)?;
    super::super::admission::inspection::verify_artifact_facts(request.source.component_bytes(), request.facts)?;
    admit_observed_import_surface(request, &materialization)?;
    let mut plan = super::super::admission::plan_execution(
        request.profile,
        materialization,
        request.import_manifest,
        request.facts,
        request.import_grants,
    )?;
    plan.runtime_configuration_ref =
        super::provenance::bind_configuration_ref(&plan.runtime_configuration_ref, backend)?;
    let input_bytes = crate::preserves_rail::canonical_bytes(request.input).map_err(|error| {
        super::super::model::Denial::classified(
            super::super::model::DenialClass::InvalidPreservesPayload,
            format!("component input is not canonical Preserves: {error}"),
        )
    })?;
    let input_length = u64::try_from(input_bytes.len()).map_err(|error| {
        super::super::model::Denial::classified(
            super::super::model::DenialClass::ResourceDenial,
            format!("component input length is unsupported: {error}"),
        )
    })?;
    if input_length > request.profile.resources.max_hostcall_bytes {
        return Err(super::super::model::Denial::classified(
            super::super::model::DenialClass::ResourceDenial,
            "component input exceeds the admitted canonical payload bound",
        ));
    }
    let input_ref = crate::preserves_rail::canonical_hash(request.input)
        .map_err(|error| super::super::model::Denial::new(format!("component input identity failed: {error}")))?;
    Ok(AdmittedRequest {
        plan,
        input_bytes,
        input_ref,
    })
}

fn execute_admitted(
    request: &ExecutionRequest<'_>,
    plan: &super::super::admission::ExecutionPlan,
    input_bytes: &[u8],
    input_ref: String,
) -> super::super::model::Result<ExecutionOutcome> {
    let inspection = plan_receipt(request, PlanReceiptInput {
        plan,
        stage: super::super::evidence::receipt::Stage::Inspection,
        input_ref: None,
        output_ref: None,
        fuel_limit: None,
        fuel_remaining: None,
        parent_refs: Vec::new(),
    })?;
    let mut session = match super::instantiate(request.profile, request.source.component_bytes(), request.facts) {
        Ok(session) => session,
        Err(denial) => {
            return super::denial::plan_denied_outcome(request, plan, denial, vec![inspection], Some(input_ref));
        }
    };
    let instantiation = plan_receipt(request, PlanReceiptInput {
        plan,
        stage: super::super::evidence::receipt::Stage::Instantiation,
        input_ref: None,
        output_ref: None,
        fuel_limit: None,
        fuel_remaining: None,
        parent_refs: vec![inspection.receipt_ref.clone()],
    })?;
    let runtime = match super::invoke(&mut session, input_bytes) {
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
    let (output, output_ref) = match decode_output(request.profile, &runtime.output_bytes) {
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
        stage: super::super::evidence::receipt::Stage::Execution,
        input_ref: Some(input_ref),
        output_ref: Some(output_ref),
        fuel_limit: Some(request.profile.resources.fuel),
        fuel_remaining: Some(runtime.fuel_remaining),
        parent_refs: vec![instantiation.receipt_ref.clone()],
    })?;
    Ok(ExecutionOutcome {
        decision: super::super::evidence::receipt::Decision::Pass,
        output: Some(output),
        receipts: vec![inspection, instantiation, execution],
        diagnostics: Vec::new(),
    })
}

pub(super) fn decode_output(
    profile: &super::super::model::RuntimeProfile,
    output_bytes: &[u8],
) -> super::super::model::Result<(preserves::IOValue, String)> {
    let output_length = u64::try_from(output_bytes.len()).map_err(|error| {
        super::super::model::Denial::classified(
            super::super::model::DenialClass::ResourceDenial,
            format!("component output length is unsupported: {error}"),
        )
    })?;
    if output_length > profile.resources.max_result_bytes {
        return Err(super::super::model::Denial::classified(
            super::super::model::DenialClass::ResourceDenial,
            "component output exceeds the admitted canonical result bound",
        ));
    }
    let output = crate::preserves_rail::parse_canonical_bytes(output_bytes).map_err(|error| {
        super::super::model::Denial::classified(
            super::super::model::DenialClass::InvalidPreservesPayload,
            format!("component output is not canonical Preserves: {error}"),
        )
    })?;
    let output_ref = crate::preserves_rail::canonical_hash(&output)
        .map_err(|error| super::super::model::Denial::new(format!("component output identity failed: {error}")))?;
    Ok((output, output_ref))
}

pub(super) struct PlanReceiptInput<'a> {
    pub(super) plan: &'a super::super::admission::ExecutionPlan,
    pub(super) stage: super::super::evidence::receipt::Stage,
    pub(super) input_ref: Option<String>,
    pub(super) output_ref: Option<String>,
    pub(super) fuel_limit: Option<u64>,
    pub(super) fuel_remaining: Option<u64>,
    pub(super) parent_refs: Vec<String>,
}

pub(super) fn plan_receipt(
    request: &ExecutionRequest<'_>,
    input: PlanReceiptInput<'_>,
) -> super::super::model::Result<super::super::evidence::receipt::ExecutionRecord> {
    let PlanReceiptInput {
        plan,
        stage,
        input_ref,
        output_ref,
        fuel_limit,
        fuel_remaining,
        parent_refs,
    } = input;
    super::super::evidence::receipt::build(super::super::evidence::receipt::Input {
        stage,
        decision: super::super::evidence::receipt::Decision::Pass,
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
    request: &ExecutionRequest<'_>,
    materialization: &super::super::evidence::materialization::Admission,
) -> super::super::model::Result<()> {
    let world = super::super::imports::surface::declared_world(request.profile);
    let observation = super::super::imports::observation::observe_artifact_surface(
        &request.facts.declared_world,
        &materialization.component_ref,
        request.source.component_bytes(),
    )?;
    let admission = super::super::imports::surface::admit_declared(request.import_manifest, &world, &observation)?;
    if !admission.is_admitted {
        return Err(super::super::model::Denial::from_blockers(admission.blockers));
    }
    Ok(())
}
