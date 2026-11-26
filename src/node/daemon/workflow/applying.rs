//! Live workflow bundle apply orchestration and its input projections.

#[derive(Debug, Default)]
pub(super) struct Check {
    pub(super) receipt_ref: Option<String>,
    pub(super) diagnostics: Vec<String>,
}

#[derive(Debug, Default)]
pub(super) struct ImportStep {
    pub(super) receipt_ref: Option<String>,
    pub(super) imported_refs: Vec<String>,
    pub(super) diagnostics: Vec<String>,
}

#[derive(Debug, Default)]
pub(super) struct TransferStep {
    pub(super) envelope_ref: Option<String>,
    pub(super) operation_ref: Option<String>,
    pub(super) send_receipt_ref: Option<String>,
    pub(super) send_receipt_value: Option<preserves::IOValue>,
    pub(super) diagnostics: Vec<String>,
}

pub(super) struct FinishInput<'a> {
    pub(super) input: &'a super::bundling::ControlLiveWorkflowBundleApplyInput<'a>,
    pub(super) verified: super::artifacts::ControlLiveWorkflowBundleVerify,
    pub(super) expected: super::intake::LiveWorkflowBundleExpectedInput<'a>,
    pub(super) gate_receipt_ref: Option<String>,
    pub(super) import_receipt_ref: Option<String>,
    pub(super) imported_refs: Vec<String>,
    pub(super) envelope_ref: Option<String>,
    pub(super) operation_ref: Option<String>,
    pub(super) send_receipt_ref: Option<String>,
    pub(super) send_receipt_value: Option<preserves::IOValue>,
    pub(super) diagnostics: Vec<String>,
}

pub async fn apply_control_live_workflow_bundle(
    input: &super::bundling::ControlLiveWorkflowBundleApplyInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleApply> {
    validate_live_workflow_bundle_apply_input(input)?;
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    super::storage::ensure_state_layout(&state_root)?;
    let verify_input = live_workflow_bundle_verify_input_from_apply(input);
    let verified = super::verifying::verify_control_live_workflow_bundle(&verify_input)?;
    let expected = super::verifying::live_workflow_bundle_expected_input_from_verify(&verify_input);
    let Check {
        receipt_ref: gate_receipt_ref,
        diagnostics: gate_diagnostics,
    } = super::steps::apply_gate_check(input, &verified)?;
    let mut diagnostics = verified.diagnostics.clone();
    diagnostics.extend(gate_diagnostics);
    if input.should_send && input.request_value.is_none() {
        diagnostics.push("node control live workflow bundle apply send requested without a request".to_string());
    }
    let ImportStep {
        receipt_ref: import_receipt_ref,
        imported_refs,
        diagnostics: import_diagnostics,
    } = if diagnostics.is_empty() {
        super::steps::apply_import_step(&state_root, input)?
    } else {
        ImportStep::default()
    };
    diagnostics.extend(import_diagnostics);
    let TransferStep {
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        send_receipt_value,
        diagnostics: transfer_diagnostics,
    } = if diagnostics.is_empty() {
        super::steps::apply_transfer_step(&state_root, input).await?
    } else {
        TransferStep::default()
    };
    diagnostics.extend(transfer_diagnostics);
    super::steps::finish_apply(&state_root, FinishInput {
        input,
        verified,
        expected,
        gate_receipt_ref,
        import_receipt_ref,
        imported_refs,
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        send_receipt_value,
        diagnostics,
    })
}

fn validate_live_workflow_bundle_apply_input(
    input: &super::bundling::ControlLiveWorkflowBundleApplyInput<'_>,
) -> crate::error::Result<()> {
    super::storage::validate_state_root(input.state_root)?;
    super::verifying::validate_live_workflow_bundle_verify_input(&live_workflow_bundle_verify_input_from_apply(input))?;
    if let Some(from_peer) = input.from_peer {
        super::storage::validate_node_id(from_peer)?;
    }
    if let Some(operation_ref) = input.expected_operation_ref {
        super::guards::validate_ingress_ref(operation_ref, "node control live workflow bundle apply operation id")?;
    }
    super::guards::validate_ingress_refs(
        input.peer_bootstrap_refs,
        "node control live workflow bundle apply peer bootstrap ref",
    )?;
    super::guards::validate_ingress_refs(
        input.authority_refs,
        "node control live workflow bundle apply authority ref",
    )?;
    super::guards::validate_ingress_refs(input.policy_refs, "node control live workflow bundle apply policy ref")?;
    super::guards::validate_ingress_refs(input.resource_refs, "node control live workflow bundle apply resource ref")?;
    super::guards::validate_ingress_refs(input.evidence_refs, "node control live workflow bundle apply evidence ref")?;
    if input.request_value.is_some() || input.should_send {
        super::guards::validate_live_send_timeout(input.join_timeout_ms)?;
        super::guards::validate_live_send_attempts(input.max_attempts)?;
    }
    if let Some(profile) = input.topology_profile {
        super::shaping::validate_live_topology_profile(profile)?;
    }
    if let Some(profile) = input.transport_profile {
        super::shaping::validate_live_transport_profile_shape(profile)?;
    }
    Ok(())
}

fn live_workflow_bundle_verify_input_from_apply<'a>(
    input: &'a super::bundling::ControlLiveWorkflowBundleApplyInput<'a>,
) -> super::bundling::ControlLiveWorkflowBundleVerifyInput<'a> {
    super::bundling::ControlLiveWorkflowBundleVerifyInput {
        bundle_value: input.bundle_value,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

pub(super) fn live_workflow_bundle_import_input_from_apply<'a>(
    input: &'a super::bundling::ControlLiveWorkflowBundleApplyInput<'a>,
) -> super::bundling::ControlLiveWorkflowBundleImportInput<'a> {
    super::bundling::ControlLiveWorkflowBundleImportInput {
        state_root: input.state_root,
        bundle_value: input.bundle_value,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_expected_input_from_import<'a>(
    input: &'a super::bundling::ControlLiveWorkflowBundleImportInput<'a>,
) -> super::intake::LiveWorkflowBundleExpectedInput<'a> {
    super::intake::LiveWorkflowBundleExpectedInput {
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

pub(super) fn live_workflow_bundle_import_diagnostics(
    input: &super::bundling::ControlLiveWorkflowBundleImportInput<'_>,
    ticket: &super::entitlements::ControlLiveTicket,
    admission: &super::entitlements::ControlLivePeerAdmission,
    authority: &super::entitlements::ControlAuthorityGrant,
) -> Vec<String> {
    super::verifying::live_workflow_bundle_expected_diagnostics(
        &live_workflow_bundle_expected_input_from_import(input),
        ticket,
        admission,
        authority,
    )
}
