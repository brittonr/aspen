use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::node_daemon;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::retention;

use crate::RetentionEvidenceArgs;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub(crate) enum RetentionCommand {
    Class {
        #[arg(long)]
        class_name: String,
        #[arg(long, default_value_t = 0)]
        minimum_age_seconds: u64,
        #[arg(long)]
        maximum_age_seconds: Option<u64>,
        #[arg(long)]
        deletion_authority_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "secret-redaction-hook", default_value = "false")]
        has_secret_redaction_hook: bool,
        #[arg(long = "remote-gc-plan", default_value = "false")]
        has_remote_gc_plan: bool,
        #[arg(long = "compaction", default_value = "false")]
        has_compaction: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Pin {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        owner_ref: String,
        #[arg(long)]
        expiry_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value = "true")]
        has_authority: bool,
        #[arg(long)]
        pin_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Unpin {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        pin_ref: String,
        #[arg(long)]
        requester_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value = "true")]
        has_authority: bool,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Admit {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        kind: String,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long = "bound-ref")]
        bound_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "remote-ref")]
        remote_refs: Vec<String>,
        #[arg(long = "reference-index-complete")]
        is_reference_index_complete: bool,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearance {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        peer_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        remote_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        authority_ref: String,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearanceRequest {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        peer_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        remote_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        authority_ref: String,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearanceRespond {
        #[arg(long)]
        root: PathBuf,
        request: PathBuf,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearanceImport {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        response: PathBuf,
        #[arg(long)]
        expected_peer_ref: Option<String>,
        #[arg(long)]
        expected_remote_ref: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearanceLiveRequestSend {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        requester_node_root: Option<PathBuf>,
        #[arg(long)]
        peer_ticket: PathBuf,
        #[arg(long)]
        requester_node_id: String,
        #[arg(long)]
        peer_node_id: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
        max_attempts: u64,
        #[arg(long, default_value_t = 10_000)]
        join_timeout_ms: u64,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        peer_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        remote_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        authority_ref: String,
        #[arg(long = "retention-evidence-ref")]
        retention_evidence_refs: Vec<String>,
        #[arg(long = "peer-bootstrap-ref")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "transport-evidence-ref")]
        transport_evidence_refs: Vec<String>,
        #[arg(long)]
        request_out: Option<PathBuf>,
        #[arg(long)]
        control_out: Option<PathBuf>,
        #[arg(long)]
        transport_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RemoteClearanceLiveResponseSend {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        peer_node_root: Option<PathBuf>,
        #[arg(long)]
        requester_ticket: PathBuf,
        request: PathBuf,
        #[arg(long)]
        peer_node_id: String,
        #[arg(long)]
        requester_node_id: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
        max_attempts: u64,
        #[arg(long, default_value_t = 10_000)]
        join_timeout_ms: u64,
        #[arg(long = "response-evidence-ref")]
        response_evidence_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "peer-bootstrap-ref")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "transport-evidence-ref")]
        transport_evidence_refs: Vec<String>,
        #[arg(long)]
        response_out: Option<PathBuf>,
        #[arg(long)]
        control_out: Option<PathBuf>,
        #[arg(long)]
        transport_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RemoteClearanceLiveImportWorkflow {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        response: PathBuf,
        #[arg(long)]
        request_control: PathBuf,
        #[arg(long)]
        request_send_receipt: PathBuf,
        #[arg(long)]
        request_receive_receipt: PathBuf,
        #[arg(long)]
        request_ingress_ref: String,
        #[arg(long)]
        response_control: PathBuf,
        #[arg(long)]
        response_send_receipt: PathBuf,
        #[arg(long)]
        response_receive_receipt: PathBuf,
        #[arg(long)]
        response_ingress_ref: String,
        #[arg(long)]
        expected_peer_ref: Option<String>,
        #[arg(long)]
        expected_remote_ref: Option<String>,
        #[arg(long)]
        import_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RemoteClearanceLiveLoopback {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        requester_node_root: PathBuf,
        #[arg(long)]
        peer_node_root: PathBuf,
        #[arg(long)]
        requester_node_id: String,
        #[arg(long)]
        peer_node_id: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        request_sequence: u64,
        #[arg(long, default_value_t = 1)]
        response_sequence: u64,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        peer_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        remote_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        authority_ref: String,
        #[arg(long = "retention-evidence-ref")]
        retention_evidence_refs: Vec<String>,
        #[arg(long = "response-evidence-ref")]
        response_evidence_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "request-peer-bootstrap-ref")]
        request_peer_bootstrap_refs: Vec<String>,
        #[arg(long = "request-authority-ref")]
        request_authority_refs: Vec<String>,
        #[arg(long = "request-policy-ref")]
        request_policy_refs: Vec<String>,
        #[arg(long = "request-resource-ref")]
        request_resource_refs: Vec<String>,
        #[arg(long = "request-transport-evidence-ref")]
        request_transport_evidence_refs: Vec<String>,
        #[arg(long = "response-peer-bootstrap-ref")]
        response_peer_bootstrap_refs: Vec<String>,
        #[arg(long = "response-authority-ref")]
        response_authority_refs: Vec<String>,
        #[arg(long = "response-policy-ref")]
        response_policy_refs: Vec<String>,
        #[arg(long = "response-resource-ref")]
        response_resource_refs: Vec<String>,
        #[arg(long = "response-transport-evidence-ref")]
        response_transport_evidence_refs: Vec<String>,
        #[arg(long)]
        request_out: Option<PathBuf>,
        #[arg(long)]
        response_out: Option<PathBuf>,
        #[arg(long)]
        import_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Explain {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: Option<String>,
        #[arg(long)]
        retention_class: Option<String>,
        #[arg(long)]
        action: Option<String>,
        #[arg(long)]
        subsystem: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    BundleExport {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        explain: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "internal")]
        profile: String,
    },
    BundleVerify {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    GcPlan {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, default_value = "generic")]
        subsystem: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long, default_value = "delete")]
        action: String,
        #[command(flatten)]
        retention: RetentionEvidenceArgs,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    GcApplyPlan {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        plan_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    GcAudit {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        execution_ref: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Check {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long, default_value = "eligibility")]
        action: String,
        #[arg(long)]
        requester_ref: String,
        #[arg(long = "reference-index-complete", default_value = "true")]
        is_reference_index_complete: bool,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "remote-ref")]
        remote_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value = "false")]
        has_delete_authority: bool,
        #[arg(long = "remote-gc-clearance")]
        has_remote_gc_clearance: bool,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

pub(crate) fn run_retention_command(command: RetentionCommand) -> Result<()> {
    match command {
        RetentionCommand::Class {
            class_name,
            minimum_age_seconds,
            maximum_age_seconds,
            deletion_authority_ref,
            policy_refs,
            has_secret_redaction_hook,
            has_remote_gc_plan,
            has_compaction,
            out,
        } => {
            let value = retention::retention_class_profile_value(&retention::RetentionClassProfileInput {
                class_name: class_name.clone(),
                minimum_age_seconds,
                maximum_age_seconds,
                deletion_authority_ref,
                policy_refs,
                has_secret_redaction_hook,
                has_remote_gc_plan,
                can_compact: has_compaction,
            })?;
            let profile = retention::parse_retention_class_profile(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("retention class ref={} class={}", profile.profile_ref, profile.class_name),
            );
            Ok(())
        }
        RetentionCommand::Pin {
            root,
            object_ref,
            object_kind,
            retention_class,
            source,
            reason,
            owner_ref,
            expiry_ref,
            policy_refs,
            evidence_refs,
            has_authority,
            pin_out,
            receipt_out,
        } => {
            let operation = retention::pin_object(&root, retention::RetentionPinInput {
                object_ref,
                object_kind,
                retention_class,
                source,
                reason,
                owner_ref,
                expiry_ref,
                policy_refs,
                evidence_refs,
                has_authority,
            })?;
            write_optional_preserves(pin_out.as_ref(), &operation.pin.value)?;
            let is_receipt_written = write_optional_preserves(receipt_out.as_ref(), &operation.receipt.value)?;
            print_or_log_summary(
                is_receipt_written,
                &format!(
                    "retention pin decision={} pin={} receipt={}",
                    operation.receipt.decision, operation.pin.pin_ref, operation.receipt.receipt_ref
                ),
            );
            Ok(())
        }
        RetentionCommand::Unpin {
            root,
            pin_ref,
            requester_ref,
            policy_refs,
            evidence_refs,
            has_authority,
            receipt_out,
        } => {
            let receipt = retention::unpin_object(retention::UnpinObjectInput {
                root: &root,
                pin_ref: &pin_ref,
                requester_ref: &requester_ref,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                has_authority,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &receipt.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention unpin decision={} pin={} receipt={}",
                    receipt.decision, pin_ref, receipt.receipt_ref
                ),
            );
            Ok(())
        }
        RetentionCommand::Admit {
            root,
            kind,
            decision,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            bound_refs,
            retained_refs,
            remote_refs,
            is_reference_index_complete,
            is_stale,
            revoked_refs,
            diagnostics,
            out,
        } => {
            let admission =
                retention::store_retention_evidence_admission(&root, &retention::RetentionEvidenceAdmissionInput {
                    kind: &kind,
                    decision: &decision,
                    requester_ref: &requester_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    bound_refs: &bound_refs,
                    retained_refs: &retained_refs,
                    remote_refs: &remote_refs,
                    is_reference_index_complete,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    diagnostics: &diagnostics,
                })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &admission.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention admission ref={} kind={} decision={}",
                    admission.admission_ref, admission.kind, admission.decision
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearance {
            root,
            decision,
            requester_ref,
            peer_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            remote_ref,
            policy_ref,
            authority_ref,
            evidence_refs,
            retained_refs,
            is_stale,
            revoked_refs,
            diagnostics,
            out,
        } => {
            let clearance =
                retention::store_retention_remote_gc_clearance(&root, &retention::RetentionRemoteGcClearanceInput {
                    decision: &decision,
                    requester_ref: &requester_ref,
                    peer_ref: &peer_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    remote_ref: &remote_ref,
                    policy_ref: &policy_ref,
                    authority_ref: &authority_ref,
                    evidence_refs: &evidence_refs,
                    retained_refs: &retained_refs,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    diagnostics: &diagnostics,
                })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &clearance.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance ref={} peer={} remote={} decision={}",
                    clearance.clearance_ref, clearance.peer_ref, clearance.remote_ref, clearance.decision
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceRequest {
            root,
            requester_ref,
            peer_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            remote_ref,
            policy_ref,
            authority_ref,
            evidence_refs,
            out,
        } => {
            let request = retention::store_retention_remote_gc_clearance_request(
                &root,
                &retention::RetentionRemoteGcClearanceRequestInput {
                    requester_ref: &requester_ref,
                    peer_ref: &peer_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    remote_ref: &remote_ref,
                    policy_ref: &policy_ref,
                    authority_ref: &authority_ref,
                    evidence_refs: &evidence_refs,
                },
            )?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &request.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance request ref={} peer={} remote={} object={}",
                    request.request_ref, request.peer_ref, request.remote_ref, request.object_ref
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceRespond {
            root,
            request,
            evidence_refs,
            retained_refs,
            is_stale,
            revoked_refs,
            diagnostics,
            out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let response = retention::store_retention_remote_gc_clearance_response(
                retention::RetentionRemoteGcClearanceResponseInput {
                    root: &root,
                    request_value: &request_value,
                    evidence_refs: &evidence_refs,
                    retained_refs: &retained_refs,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    diagnostics: &diagnostics,
                },
            )?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &response.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance response ref={} decision={} request={} clearance={}",
                    response.response_ref, response.decision, response.request_ref, response.clearance_ref
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceImport {
            root,
            request,
            response,
            expected_peer_ref,
            expected_remote_ref,
            out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let response_value = read_preserves_file(&response)?;
            let import = retention::import_retention_remote_gc_clearance_response(
                retention::RetentionRemoteGcClearanceImportInput {
                    root: &root,
                    request_value: &request_value,
                    response_value: &response_value,
                    expected_peer_ref: expected_peer_ref.as_deref(),
                    expected_remote_ref: expected_remote_ref.as_deref(),
                },
            )?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &import.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance import ref={} decision={} clearance={}",
                    import.import_ref,
                    import.decision,
                    import.clearance_ref.as_deref().unwrap_or("none")
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceLiveRequestSend {
            root,
            requester_node_root,
            peer_ticket,
            requester_node_id,
            peer_node_id,
            topic,
            sequence,
            max_attempts,
            join_timeout_ms,
            requester_ref,
            peer_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            remote_ref,
            policy_ref,
            authority_ref,
            retention_evidence_refs,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            transport_evidence_refs,
            request_out,
            control_out,
            transport_receipt_out,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&peer_ticket)?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let sent = runtime.block_on(retention::send_retention_remote_gc_clearance_live_request(
                retention::RetentionRemoteGcClearanceLiveRequestSendInput {
                    root: &root,
                    requester_node_root: requester_node_root.as_deref(),
                    peer_ticket_value: &ticket_value,
                    requester_node_id: &requester_node_id,
                    peer_node_id: &peer_node_id,
                    topic: &topic,
                    sequence,
                    max_attempts,
                    join_timeout_ms,
                    requester_ref: &requester_ref,
                    peer_ref: &peer_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    remote_ref: &remote_ref,
                    policy_ref: &policy_ref,
                    authority_ref: &authority_ref,
                    retention_evidence_refs: &retention_evidence_refs,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    transport_evidence_refs: &transport_evidence_refs,
                },
            ))?;
            write_optional_preserves(request_out.as_ref(), &sent.request.value)?;
            write_optional_preserves(control_out.as_ref(), &sent.control_value)?;
            if let Some(path) = transport_receipt_out.as_ref()
                && let Some(value) = sent.send.transport_receipt_value.as_ref()
            {
                write_file(path, &to_text(value)?)?;
            }
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &sent.send.send_receipt_value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance live request-send request={} control={} send={} transport={} diagnostics={}",
                    sent.request.request_ref,
                    sent.control_ref,
                    sent.send.send_receipt_ref,
                    sent.send.transport_receipt_ref.as_deref().unwrap_or("none"),
                    node_daemon::parse_node_control_live_send_receipt(&sent.send.send_receipt_value)?.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceLiveResponseSend {
            root,
            peer_node_root,
            requester_ticket,
            request,
            peer_node_id,
            requester_node_id,
            topic,
            sequence,
            max_attempts,
            join_timeout_ms,
            response_evidence_refs,
            retained_refs,
            is_stale,
            revoked_refs,
            diagnostics,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            transport_evidence_refs,
            response_out,
            control_out,
            transport_receipt_out,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&requester_ticket)?;
            let request_value = read_preserves_file(&request)?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let sent = runtime.block_on(retention::send_retention_remote_gc_clearance_live_response(
                retention::RetentionRemoteGcClearanceLiveResponseSendInput {
                    root: &root,
                    peer_node_root: peer_node_root.as_deref(),
                    requester_ticket_value: &ticket_value,
                    request_value: &request_value,
                    peer_node_id: &peer_node_id,
                    requester_node_id: &requester_node_id,
                    topic: &topic,
                    sequence,
                    max_attempts,
                    join_timeout_ms,
                    response_evidence_refs: &response_evidence_refs,
                    retained_refs: &retained_refs,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    response_diagnostics: &diagnostics,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    transport_evidence_refs: &transport_evidence_refs,
                },
            ))?;
            write_optional_preserves(response_out.as_ref(), &sent.response.value)?;
            write_optional_preserves(control_out.as_ref(), &sent.control_value)?;
            if let Some(path) = transport_receipt_out.as_ref()
                && let Some(value) = sent.send.transport_receipt_value.as_ref()
            {
                write_file(path, &to_text(value)?)?;
            }
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &sent.send.send_receipt_value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance live response-send response={} control={} send={} transport={} diagnostics={}",
                    sent.response.response_ref,
                    sent.control_ref,
                    sent.send.send_receipt_ref,
                    sent.send.transport_receipt_ref.as_deref().unwrap_or("none"),
                    node_daemon::parse_node_control_live_send_receipt(&sent.send.send_receipt_value)?.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceLiveImportWorkflow {
            root,
            request,
            response,
            request_control,
            request_send_receipt,
            request_receive_receipt,
            request_ingress_ref,
            response_control,
            response_send_receipt,
            response_receive_receipt,
            response_ingress_ref,
            expected_peer_ref,
            expected_remote_ref,
            import_out,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let response_value = read_preserves_file(&response)?;
            let request_control_value = read_preserves_file(&request_control)?;
            let request_send_receipt_value = read_preserves_file(&request_send_receipt)?;
            let request_receive_receipt_value = read_preserves_file(&request_receive_receipt)?;
            let response_control_value = read_preserves_file(&response_control)?;
            let response_send_receipt_value = read_preserves_file(&response_send_receipt)?;
            let response_receive_receipt_value = read_preserves_file(&response_receive_receipt)?;
            let imported = retention::import_retention_remote_gc_clearance_live_workflow(
                retention::RetentionRemoteGcClearanceLiveImportWorkflowInput {
                    root: &root,
                    request_value: &request_value,
                    response_value: &response_value,
                    request_control_value: &request_control_value,
                    request_send_receipt_value: &request_send_receipt_value,
                    request_receive_receipt_value: &request_receive_receipt_value,
                    request_ingress_ref: &request_ingress_ref,
                    response_control_value: &response_control_value,
                    response_send_receipt_value: &response_send_receipt_value,
                    response_receive_receipt_value: &response_receive_receipt_value,
                    response_ingress_ref: &response_ingress_ref,
                    expected_peer_ref: expected_peer_ref.as_deref(),
                    expected_remote_ref: expected_remote_ref.as_deref(),
                },
            )?;
            write_optional_preserves(import_out.as_ref(), &imported.import.value)?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &imported.workflow.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance live import-workflow ref={} decision={} import={} clearance={} request-send={} response-send={} diagnostics={}",
                    imported.workflow.workflow_ref,
                    imported.workflow.decision,
                    imported.import.import_ref,
                    imported.import.clearance_ref.as_deref().unwrap_or("none"),
                    imported.request_send_receipt_ref,
                    imported.response_send_receipt_ref,
                    imported.workflow.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceLiveLoopback {
            root,
            requester_node_root,
            peer_node_root,
            requester_node_id,
            peer_node_id,
            topic,
            request_sequence,
            response_sequence,
            requester_ref,
            peer_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            remote_ref,
            policy_ref,
            authority_ref,
            retention_evidence_refs,
            response_evidence_refs,
            retained_refs,
            is_stale,
            revoked_refs,
            diagnostics,
            request_peer_bootstrap_refs,
            request_authority_refs,
            request_policy_refs,
            request_resource_refs,
            request_transport_evidence_refs,
            response_peer_bootstrap_refs,
            response_authority_refs,
            response_policy_refs,
            response_resource_refs,
            response_transport_evidence_refs,
            request_out,
            response_out,
            import_out,
            receipt_out,
        } => {
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let live = runtime.block_on(retention::run_retention_remote_gc_clearance_live_loopback(
                retention::RetentionRemoteGcClearanceLiveLoopbackInput {
                    root: &root,
                    requester_node_root: &requester_node_root,
                    peer_node_root: &peer_node_root,
                    requester_node_id: &requester_node_id,
                    peer_node_id: &peer_node_id,
                    topic: &topic,
                    request_sequence,
                    response_sequence,
                    requester_ref: &requester_ref,
                    peer_ref: &peer_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    remote_ref: &remote_ref,
                    policy_ref: &policy_ref,
                    authority_ref: &authority_ref,
                    retention_evidence_refs: &retention_evidence_refs,
                    response_evidence_refs: &response_evidence_refs,
                    retained_refs: &retained_refs,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    response_diagnostics: &diagnostics,
                    request_peer_bootstrap_refs: &request_peer_bootstrap_refs,
                    request_authority_refs: &request_authority_refs,
                    request_policy_refs: &request_policy_refs,
                    request_resource_refs: &request_resource_refs,
                    request_transport_evidence_refs: &request_transport_evidence_refs,
                    response_peer_bootstrap_refs: &response_peer_bootstrap_refs,
                    response_authority_refs: &response_authority_refs,
                    response_policy_refs: &response_policy_refs,
                    response_resource_refs: &response_resource_refs,
                    response_transport_evidence_refs: &response_transport_evidence_refs,
                },
            ))?;
            if let Some(path) = request_out.as_ref() {
                write_file(path, &to_text(&live.request.value)?)?;
            }
            if let Some(path) = response_out.as_ref() {
                write_file(path, &to_text(&live.response.value)?)?;
            }
            if let Some(path) = import_out.as_ref() {
                write_file(path, &to_text(&live.import.value)?)?;
            }
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &live.workflow.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance live workflow ref={} decision={} request={} response={} import={} clearance={} diagnostics={}",
                    live.workflow.workflow_ref,
                    live.workflow.decision,
                    live.request.request_ref,
                    live.response.response_ref,
                    live.import.import_ref,
                    live.import.clearance_ref.as_deref().unwrap_or("none"),
                    live.workflow.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::Explain {
            root,
            object_ref,
            object_kind,
            retention_class,
            action,
            subsystem,
            out,
        } => {
            let explain = retention::explain_retention_candidate(retention::RetentionCandidateExplainInput {
                root: &root,
                object_ref: &object_ref,
                object_kind: object_kind.as_deref(),
                retention_class: retention_class.as_deref(),
                action: action.as_deref(),
                subsystem: subsystem.as_deref(),
            })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &explain.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention explain ref={} object={} pins={} admissions={} clearances={} plans={} applies={} executes={} audits={} receipts={} tombstones={} diagnostics={}",
                    explain.explain_ref,
                    explain.object_ref,
                    explain.pin_refs.len(),
                    explain.admission_refs.len(),
                    explain.remote_clearance_refs.len(),
                    explain.gc_plan_refs.len(),
                    explain.gc_apply_refs.len(),
                    explain.gc_execution_refs.len(),
                    explain.gc_audit_refs.len(),
                    explain.retention_receipt_refs.len(),
                    explain.tombstone_refs.len(),
                    explain.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::BundleExport {
            root,
            explain,
            out,
            profile,
        } => {
            let explain_value = read_preserves_file(&explain)?;
            let profile = retention::RetentionCandidateBundleExportProfile::parse(&profile)?;
            let bundle =
                retention::export_retention_candidate_bundle(retention::RetentionCandidateBundleExportInput {
                    root: &root,
                    explain_value: &explain_value,
                    out: &out,
                    profile,
                })?;
            eprintln!(
                "retention bundle ref={} explain={} profile={} artifacts={} diagnostics={} out={}",
                bundle.bundle_ref,
                bundle.explain_ref,
                profile.as_str(),
                bundle.artifact_refs.len(),
                bundle.diagnostics.len(),
                out.display()
            );
            Ok(())
        }
        RetentionCommand::BundleVerify { bundle, receipt_out } => {
            let verify =
                retention::verify_retention_candidate_bundle(retention::RetentionCandidateBundleVerifyInput {
                    bundle_dir: &bundle,
                })?;
            let text = to_text(&verify.value)?;
            if let Some(path) = receipt_out {
                write_file(&path, &text)?;
                eprintln!("retention bundle verify receipt {} written to {}", verify.verify_ref, path.display());
            } else {
                println!("{text}");
            }
            eprintln!(
                "retention bundle verify ref={} decision={} bundle={} files={} diagnostics={}",
                verify.verify_ref,
                verify.decision,
                verify.bundle_ref,
                verify.file_refs.len(),
                verify.diagnostics.len()
            );
            Ok(())
        }
        RetentionCommand::GcPlan {
            root,
            subsystem,
            object_ref,
            object_kind,
            retention_class,
            action,
            retention,
            out,
        } => {
            let evidence = retention.into_retention_evidence();
            let plan = retention::store_retention_gc_plan(retention::RetentionGcPlanInput {
                root: &root,
                subsystem: &subsystem,
                object_ref: &object_ref,
                object_kind: &object_kind,
                retention_class: &retention_class,
                action: &action,
                evidence: &evidence,
            })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &plan.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention gc plan ref={} decision={} subsystem={} action={} object={} gates={} diagnostics={}",
                    plan.plan_ref,
                    plan.decision,
                    plan.subsystem,
                    plan.action,
                    plan.object_ref,
                    plan.gates.len(),
                    plan.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::GcApplyPlan {
            root,
            plan_ref,
            receipt_out,
        } => {
            let apply = retention::apply_retention_gc_plan(retention::RetentionGcApplyFromPlanInput {
                root: &root,
                plan_ref: &plan_ref,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &apply.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention gc apply ref={} decision={} plan={} recomputed={} receipt={} tombstone={} diagnostics={}",
                    apply.apply_ref,
                    apply.decision,
                    apply.plan_ref,
                    apply.recomputed_plan_ref,
                    apply.retention_receipt_ref.as_deref().unwrap_or("none"),
                    apply.tombstone_ref.as_deref().unwrap_or("none"),
                    apply.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::GcAudit {
            root,
            execution_ref,
            out,
        } => {
            let audit = retention::audit_retention_gc_execution(retention::RetentionGcAuditInput {
                root: &root,
                execution_ref: &execution_ref,
            })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &audit.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention gc audit ref={} decision={} plan={} apply={} execution={} receipt={} tombstone={} diagnostics={}",
                    audit.audit_ref,
                    audit.decision,
                    audit.plan_ref.as_deref().unwrap_or("none"),
                    audit.apply_ref.as_deref().unwrap_or("none"),
                    audit.execution_ref,
                    audit.retention_receipt_ref.as_deref().unwrap_or("none"),
                    audit.tombstone_ref.as_deref().unwrap_or("none"),
                    audit.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::Check {
            root,
            object_ref,
            object_kind,
            retention_class,
            action,
            requester_ref,
            is_reference_index_complete,
            retained_refs,
            remote_refs,
            policy_refs,
            evidence_refs,
            has_delete_authority,
            has_remote_gc_clearance,
            receipt_out,
        } => {
            let evaluation = retention::evaluate_retention(retention::RetentionEvaluationInput {
                root: &root,
                object_ref: &object_ref,
                object_kind: &object_kind,
                retention_class: &retention_class,
                action: &action,
                requester_ref: &requester_ref,
                is_reference_index_complete,
                retained_refs: &retained_refs,
                remote_refs: &remote_refs,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                has_delete_authority,
                has_remote_gc_clearance,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &evaluation.receipt.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention decision={} action={} object={} receipt={} tombstone={}",
                    evaluation.receipt.decision,
                    evaluation.receipt.action,
                    evaluation.receipt.object_ref,
                    evaluation.receipt.receipt_ref,
                    evaluation.receipt.tombstone_ref.as_deref().unwrap_or("none")
                ),
            );
            Ok(())
        }
        RetentionCommand::RunFixture { out } => {
            let artifacts = retention::run_fixture(&out)?;
            println!("retention fixture artifacts={} out={}", artifacts.len(), out.display());
            Ok(())
        }
        RetentionCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", retention::retention_summary(&value)?);
            Ok(())
        }
    }
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn write_optional_preserves(out: Option<&PathBuf>, value: &preserves::IOValue) -> Result<bool> {
    if let Some(path) = out {
        write_file(path, &to_text(value)?)?;
        Ok(true)
    } else {
        println!("{}", to_text(value)?);
        Ok(false)
    }
}

fn print_or_log_summary(is_written_to_file: bool, summary: &str) {
    if is_written_to_file {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
