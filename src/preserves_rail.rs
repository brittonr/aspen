use preserves::IOValue;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;

pub const HARNESS_SUITE_SCHEMA: &str = "molten.harness.suite.v1";
pub const HARNESS_REPORT_SCHEMA: &str = "molten.harness.report.v1";
pub const HARNESS_FAILURE_SCHEMA: &str = "molten.harness.failure.v1";
pub const HARNESS_OBSERVATION_SCHEMA: &str = "molten.harness.observation.v1";
pub const HARNESS_EFFECT_LOG_SCHEMA: &str = "molten.harness.effect-log.v1";
pub const HARNESS_BUDGET_SCHEMA: &str = "molten.harness.budget.v1";
pub const HARNESS_BUDGET_NICKEL_STATIC_SCHEMA: &str = "molten.harness.budget.nickel-static.v1";
pub const HARNESS_BUDGET_CONTRACT_SCHEMA: &str = "molten.harness.budget.contract-envelope.v1";
pub const HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA: &str = "molten.harness.basalt-resource-preflight.v1";
pub const HARNESS_BUDGET_GATE_SCHEMA: &str = "molten.harness.budget-gate.v1";
pub const HARNESS_BUDGET_USAGE_SCHEMA: &str = "molten.harness.budget-usage.v1";
pub const HARNESS_ACTOR_REGISTRY_SCHEMA: &str = "molten.harness.actor-registry.v1";
pub const HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA: &str = "molten.harness.executor-preflights.v1";
pub const HARNESS_EXECUTOR_CONFORMANCE_SCHEMA: &str = "molten.harness.executor-conformance.v1";
pub const HARNESS_CAPABILITIES_SCHEMA: &str = "molten.harness.capabilities.v1";
pub const HARNESS_CAPABILITY_CONTRACT_SCHEMA: &str = "molten.harness.capability.contract-envelope.v1";
pub const HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA: &str = "molten.harness.basalt-authority-preflight.v1";
pub const HARNESS_UCAN_PROOFSET_SCHEMA: &str = "molten.harness.ucan-proofset.v1";
pub const HARNESS_CAPABILITY_GATE_SCHEMA: &str = "molten.harness.capability-gate.v1";
pub const HARNESS_POLICY_SCHEMA: &str = "molten.harness.policy.v1";
pub const HARNESS_POLICY_NICKEL_STATIC_SCHEMA: &str = "molten.harness.policy.nickel-static.v1";
pub const HARNESS_POLICY_CONTRACT_SCHEMA: &str = "molten.harness.policy.contract-envelope.v1";
pub const HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA: &str = "molten.harness.basalt-policy-preflight.v1";
pub const HARNESS_POLICY_GATE_SCHEMA: &str = "molten.harness.policy-gate.v1";
pub const HARNESS_REPRO_BUNDLE_SCHEMA: &str = "molten.harness.repro-bundle.v1";
pub const HARNESS_REPRO_SEAL_SCHEMA: &str = "molten.harness.repro-seal.v1";
pub const HARNESS_REPRO_VERIFY_RECEIPT_SCHEMA: &str = "molten.harness.repro-verify-receipt.v1";
pub const HARNESS_REDACTION_POLICY_SCHEMA: &str = "molten.harness.redaction-policy.v1";
pub const HARNESS_REDACTION_GATE_SCHEMA: &str = "molten.harness.redaction-gate.v1";
pub const HARNESS_REDACTION_PROFILE_SCHEMA: &str = "molten.harness.redaction-profile.v1";
pub const HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA: &str = "molten.harness.redaction-transform-manifest.v1";
pub const HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA: &str = "molten.harness.redaction-transform.v1";
pub const RUNTIME_ADMISSION_DECISION_SCHEMA: &str = "molten.runtime.admission-decision.v1";
pub const RUNTIME_ACTOR_INPUT_SCHEMA: &str = "molten.runtime.actor-input.v1";
pub const RUNTIME_HOSTCALL_REQUEST_SCHEMA: &str = "molten.runtime.hostcall-request.v1";
pub const RUNTIME_HOSTCALL_DECISION_SCHEMA: &str = "molten.runtime.hostcall-decision.v1";
pub const RUNTIME_ACTOR_OUTPUT_SCHEMA: &str = "molten.runtime.actor-output.v1";
pub const RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA: &str = "molten.runtime.executor-preflight.v1";
pub const RUNTIME_STEEL_EXECUTOR_SCHEMA: &str = "molten.runtime.steel-executor.v1";
pub const RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA: &str = "molten.runtime.steel-review-receipt.v1";
pub const RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA: &str = "molten.runtime.steel-execution-receipt.v1";
pub const RUNTIME_WASM_EXECUTOR_SCHEMA: &str = "molten.runtime.wasm-executor.v1";
pub const RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA: &str = "molten.runtime.wasm-inspection-receipt.v1";
pub const RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA: &str = "molten.runtime.wasm-execution-receipt.v1";
pub const RUNTIME_WASM_ABI_SCHEMA: &str = "molten.wasm.abi.v1";
pub const RUNTIME_ADAPTER_EXECUTOR_SCHEMA: &str = "molten.runtime.adapter-executor.v1";
pub const RUNTIME_ADAPTER_PREFLIGHT_RECEIPT_SCHEMA: &str = "molten.runtime.adapter-preflight-receipt.v1";
pub const RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA: &str = "molten.runtime.remote-proxy-executor.v1";
pub const RUNTIME_REMOTE_PROXY_PREFLIGHT_RECEIPT_SCHEMA: &str = "molten.runtime.remote-proxy-preflight-receipt.v1";
pub const RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA: &str = "molten.runtime.capability-authorization.v1";
pub const RUNTIME_PREDICATE_RECEIPT_SCHEMA: &str = "molten.runtime.predicate-receipt.v1";
pub const RUNTIME_VAT_OBJECT_REF_SCHEMA: &str = "molten.runtime.vat-object-ref.v1";
pub const RUNTIME_VAT_FIXTURE_RUN_SCHEMA: &str = "molten.runtime.vat-fixture-run.v1";
pub const RUNTIME_VAT_SNAPSHOT_SCHEMA: &str = "molten.runtime.vat-snapshot.v1";
pub const RUNTIME_VAT_OBJECT_UPGRADE_RECIPE_SCHEMA: &str = "molten.runtime.vat-object-upgrade-recipe.v1";
pub const RUNTIME_VAT_RESTORE_RECEIPT_SCHEMA: &str = "molten.runtime.vat-restore-receipt.v1";
pub const RUNTIME_VAT_PROMISE_FIXTURE_SCHEMA: &str = "molten.runtime.vat-promise-fixture.v1";
pub const RUNTIME_VAT_AMBIENT_AUTHORITY_FIXTURE_SCHEMA: &str = "molten.runtime.vat-ambient-authority-fixture.v1";
pub const RUNTIME_VAT_RIGHTS_FIXTURE_SCHEMA: &str = "molten.runtime.vat-rights-fixture.v1";
pub const RUNTIME_VAT_DISTRIBUTED_REF_FIXTURE_SCHEMA: &str = "molten.runtime.vat-distributed-ref-fixture.v1";
pub const RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA: &str = "molten.runtime.vat-time-travel-fixture.v1";
pub const RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA: &str = "molten.runtime.vat-replay-fixture.v1";
pub const RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA: &str = "molten.runtime.vat-authority-graph-fixture.v1";
pub const RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA: &str = "molten.runtime.vat-portable-storage-fixture.v1";
pub const DETERMINISTIC_RUN_IDENTITY_SCHEMA: &str = "molten.determinism.run-identity.v1";
pub const DETERMINISTIC_TURN_JOURNAL_SCHEMA: &str = "molten.determinism.turn-journal.v1";
pub const DETERMINISTIC_EFFECT_LOG_SCHEMA: &str = "molten.determinism.effect-log.v1";
pub const DETERMINISTIC_FIXTURE_RECORD_SCHEMA: &str = "molten.determinism.fixture-record.v1";
pub const DETERMINISTIC_REPLAY_VERIFY_SCHEMA: &str = "molten.determinism.replay-verify.v1";
pub const DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA: &str = "molten.determinism.first-divergence.v1";
pub const DETERMINISTIC_REPLAY_ROLLUP_SCHEMA: &str = "molten.determinism.replay-rollup.v1";
pub const DETERMINISTIC_REPLAY_INDEX_SCHEMA: &str = "molten.determinism.replay-index.v1";
pub const DETERMINISTIC_CHAOS_SCHEDULE_SCHEMA: &str = "molten.determinism.chaos-schedule.v1";
pub const DETERMINISTIC_TRACE_PRIVACY_SCHEMA: &str = "molten.determinism.trace-privacy.v1";
pub const EFFECT_MANIFEST_SCHEMA: &str = "molten.effects.manifest.v1";
pub const EFFECT_HANDLER_PROFILE_SCHEMA: &str = "molten.effects.handler-profile.v1";
pub const EFFECT_HANDLER_BINDING_SCHEMA: &str = "molten.effects.handler-binding.v1";
pub const EFFECT_BINDING_RECEIPT_SCHEMA: &str = "molten.effects.binding-receipt.v1";
pub const EFFECT_REQUEST_SCHEMA: &str = "molten.effects.request.v1";
pub const EFFECT_RESPONSE_SCHEMA: &str = "molten.effects.response.v1";
pub const EFFECT_HANDLE_SCHEMA: &str = "molten.effects.handle.v1";
pub const EFFECT_COMPOUND_HANDLER_SCHEMA: &str = "molten.effects.compound-handler-profile.v1";
pub const EFFECT_DYNAMIC_OPERATION_SCHEMA: &str = "molten.effects.dynamic-operation.v1";
pub const EFFECT_HANDLE_CLEANUP_SCHEMA: &str = "molten.effects.handle-cleanup.v1";
pub const HARNESS_GATE_RECEIPT_SCHEMA: &str = "molten.harness.gate-receipt.v1";
pub const EVIDENCE_LEDGER_IMPORT_RECEIPT_SCHEMA: &str = "molten.evidence.ledger-import-receipt.v1";
pub const EVIDENCE_LEDGER_EXPORT_RECEIPT_SCHEMA: &str = "molten.evidence.ledger-export-receipt.v1";
pub const EVIDENCE_LEDGER_GC_RECEIPT_SCHEMA: &str = "molten.evidence.ledger-gc-receipt.v1";
pub const EVIDENCE_SIGNED_RECEIPT_SCHEMA: &str = "molten.evidence.signed-receipt.v1";
pub const EVIDENCE_SIGNED_RECEIPT_KEY_SCHEMA: &str = "molten.evidence.signed-receipt-key.v1";
pub const EVIDENCE_SIGNED_RECEIPT_KEY_REVOCATION_SCHEMA: &str = "molten.evidence.signed-receipt-key-revocation.v1";
pub const RETENTION_CLASS_SCHEMA: &str = "molten.retention.class.v1";
pub const RETENTION_PIN_SCHEMA: &str = "molten.retention.pin.v1";
pub const RETENTION_REFERENCE_INDEX_SCHEMA: &str = "molten.retention.reference-index.v1";
pub const RETENTION_EVIDENCE_ADMISSION_SCHEMA: &str = "molten.retention.evidence-admission.v1";
pub const RETENTION_REMOTE_GC_CLEARANCE_SCHEMA: &str = "molten.retention.remote-gc-clearance.v1";
pub const RETENTION_REMOTE_GC_CLEARANCE_REQUEST_SCHEMA: &str = "molten.retention.remote-gc-clearance-request.v1";
pub const RETENTION_REMOTE_GC_CLEARANCE_RESPONSE_SCHEMA: &str = "molten.retention.remote-gc-clearance-response.v1";
pub const RETENTION_REMOTE_GC_CLEARANCE_IMPORT_SCHEMA: &str = "molten.retention.remote-gc-clearance-import.v1";
pub const RETENTION_REMOTE_GC_CLEARANCE_LIVE_WORKFLOW_SCHEMA: &str =
    "molten.retention.remote-gc-clearance-live-workflow.v1";
pub const RETENTION_GC_PLAN_SCHEMA: &str = "molten.retention.gc-plan.v1";
pub const RETENTION_GC_APPLY_SCHEMA: &str = "molten.retention.gc-apply.v1";
pub const RETENTION_GC_EXECUTE_SCHEMA: &str = "molten.retention.gc-execute.v1";
pub const RETENTION_GC_AUDIT_SCHEMA: &str = "molten.retention.gc-audit.v1";
pub const RETENTION_CANDIDATE_EXPLAIN_SCHEMA: &str = "molten.retention.candidate-explain.v1";
pub const RETENTION_CANDIDATE_BUNDLE_SCHEMA: &str = "molten.retention.candidate-bundle.v1";
pub const RETENTION_CANDIDATE_BUNDLE_PROFILE_SCHEMA: &str = "molten.retention.candidate-bundle-profile.v1";
pub const RETENTION_CANDIDATE_BUNDLE_VERIFY_SCHEMA: &str = "molten.retention.candidate-bundle-verify.v1";
pub const RETENTION_RECEIPT_SCHEMA: &str = "molten.retention.receipt.v1";
pub const RETENTION_TOMBSTONE_SCHEMA: &str = "molten.retention.tombstone.v1";
pub const EVIDENCE_CHAIN_LINK_SCHEMA: &str = "molten.evidence.chain-link.v1";
pub const EVIDENCE_CHAIN_APPEND_RECEIPT_SCHEMA: &str = "molten.evidence.chain-append-receipt.v1";
pub const EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA: &str = "molten.evidence.chain-verify-receipt.v1";
pub const EVIDENCE_CHAIN_PREDICATE_RECEIPT_SCHEMA: &str = "molten.evidence.chain-predicate-receipt.v1";
pub const EVIDENCE_CHAIN_FORK_EVIDENCE_SCHEMA: &str = "molten.evidence.chain-fork-evidence.v1";
pub const EVIDENCE_CHAIN_ANCHOR_SCHEMA: &str = "molten.evidence.chain-anchor.v1";
pub const EVIDENCE_CHAIN_CHECKPOINT_SCHEMA: &str = "molten.evidence.chain-checkpoint.v1";
pub const EVIDENCE_CHAIN_SEGMENT_BUNDLE_SCHEMA: &str = "molten.evidence.chain-segment-bundle.v1";
pub const IROH_REPRO_EXCHANGE_RECEIPT_SCHEMA: &str = "molten.transport.iroh-repro-exchange-receipt.v1";
pub const IROH_CHAIN_EXCHANGE_RECEIPT_SCHEMA: &str = "molten.transport.iroh-chain-exchange-receipt.v1";
pub const DELIVERY_OPERATION_ID_SCHEMA: &str = "molten.delivery.operation-id.v1";
pub const DELIVERY_SCOPE_PROFILE_SCHEMA: &str = "molten.delivery.scope-profile.v1";
pub const DELIVERY_WINDOW_SCHEMA: &str = "molten.delivery.window.v1";
pub const DELIVERY_DEDUP_ENTRY_SCHEMA: &str = "molten.delivery.dedup-entry.v1";
pub const DELIVERY_IDEMPOTENCY_RECEIPT_SCHEMA: &str = "molten.delivery.idempotency-receipt.v1";
pub const DELIVERY_RETRY_RECEIPT_SCHEMA: &str = "molten.delivery.retry-receipt.v1";
pub const REMOTE_DATASPACE_ENVELOPE_SCHEMA: &str = "molten.remote-dataspace.envelope.v1";
pub const REMOTE_DATASPACE_TRANSPORT_RECEIPT_SCHEMA: &str = "molten.remote-dataspace.transport-receipt.v1";
pub const REMOTE_DATASPACE_ADMISSION_RECEIPT_SCHEMA: &str = "molten.remote-dataspace.admission-receipt.v1";
pub const REMOTE_DATASPACE_DELIVERY_LOG_SCHEMA: &str = "molten.remote-dataspace.delivery-log.v1";
pub const REMOTE_DATASPACE_GATE_RECEIPT_SCHEMA: &str = "molten.remote-dataspace.gate-receipt.v1";
pub const FEDERATION_ANNOUNCEMENT_SCHEMA: &str = "molten.federation.announcement.v1";
pub const FEDERATION_INVENTORY_SCHEMA: &str = "molten.federation.inventory.v1";
pub const FEDERATION_RECEIPT_SCHEMA: &str = "molten.federation.receipt.v1";
pub const NODE_IDENTITY_SCHEMA: &str = "molten.node.identity.v1";
pub const NODE_IDENTITY_RECEIPT_SCHEMA: &str = "molten.node.identity-receipt.v1";
pub const NODE_IDENTITY_BOOTSTRAP_SCHEMA: &str = "molten.node.identity-bootstrap.v1";
pub const NODE_IDENTITY_STARTUP_SCHEMA: &str = "molten.node.identity-startup.v1";
pub const NODE_CONFIG_SCHEMA: &str = "molten.node.config.v1";
pub const NODE_STARTUP_RECEIPT_SCHEMA: &str = "molten.node.startup-receipt.v1";
pub const NODE_ADAPTER_RECEIPT_SCHEMA: &str = "molten.node.adapter-receipt.v1";
pub const NODE_CONTROL_REQUEST_SCHEMA: &str = "molten.node.control-request.v1";
pub const NODE_CONTROL_RECEIPT_SCHEMA: &str = "molten.node.control-receipt.v1";
pub const NODE_CONTROL_LOCK_SCHEMA: &str = "molten.node.control-lock.v1";
pub const NODE_CONTROL_QUEUE_RECEIPT_SCHEMA: &str = "molten.node.control-queue-receipt.v1";
pub const NODE_CONTROL_OPERATION_RECEIPT_SCHEMA: &str = "molten.node.control-operation-receipt.v1";
pub const NODE_CONTROL_HEARTBEAT_RECEIPT_SCHEMA: &str = "molten.node.control-heartbeat-receipt.v1";
pub const NODE_CONTROL_LOOP_RECEIPT_SCHEMA: &str = "molten.node.control-loop-receipt.v1";
pub const NODE_CONTROL_SERVICE_LOCK_SCHEMA: &str = "molten.node.control-service-lock.v1";
pub const NODE_CONTROL_SERVICE_HEARTBEAT_RECEIPT_SCHEMA: &str = "molten.node.control-service-heartbeat-receipt.v1";
pub const NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA: &str = "molten.node.control-service-run-receipt.v1";
pub const NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA: &str = "molten.node.control-supervisor-policy.v1";
pub const NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA: &str = "molten.node.control-supervisor-receipt.v1";
pub const NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA: &str = "molten.node.control-ingress-envelope.v1";
pub const NODE_CONTROL_INGRESS_RECEIPT_SCHEMA: &str = "molten.node.control-ingress-receipt.v1";
pub const NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA: &str = "molten.node.control-live-transport-receipt.v1";
pub const NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA: &str = "molten.node.control-live-send-receipt.v1";
pub const NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA: &str = "molten.node.control-live-send-retry-receipt.v1";
pub const NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA: &str = "molten.node.control-live-send-duplicate-receipt.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA: &str = "molten.node.control-live-workflow-receipt.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA: &str = "molten.node.control-live-workflow-bundle.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA: &str =
    "molten.node.control-live-workflow-bundle-export-receipt.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA: &str =
    "molten.node.control-live-workflow-bundle-import-receipt.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA: &str =
    "molten.node.control-live-workflow-bundle-verify-receipt.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA: &str =
    "molten.node.control-live-workflow-bundle-gate-receipt.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA: &str =
    "molten.node.control-live-workflow-bundle-apply-receipt.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA: &str =
    "molten.node.control-live-workflow-bundle-reconcile-receipt.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA: &str = "molten.node.control-live-workflow-bundle-ack.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_EXPORT_RECEIPT_SCHEMA: &str =
    "molten.node.control-live-workflow-bundle-ack-export-receipt.v1";
pub const NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_IMPORT_RECEIPT_SCHEMA: &str =
    "molten.node.control-live-workflow-bundle-ack-import-receipt.v1";
pub const NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA: &str = "molten.node.control-live-listener-receipt.v1";
pub const NODE_CONTROL_AUTHORITY_GRANT_SCHEMA: &str = "molten.node.control-authority-grant.v1";
pub const NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA: &str = "molten.node.control-authority-receipt.v1";
pub const NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA: &str =
    "molten.node.control-authority-grant-import-receipt.v1";
pub const NODE_CONTROL_LIVE_TICKET_SCHEMA: &str = "molten.node.control-live-ticket.v1";
pub const NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA: &str = "molten.node.control-live-peer-admission.v1";
pub const NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA: &str = "molten.node.control-live-ticket-import-receipt.v1";
pub const NODE_HEALTH_RECEIPT_SCHEMA: &str = "molten.node.health-receipt.v1";
pub const NODE_SHUTDOWN_RECEIPT_SCHEMA: &str = "molten.node.shutdown-receipt.v1";
pub const OPERATOR_WORKFLOW_SCHEMA: &str = "molten.operator.workflow.v1";
pub const OPERATOR_STEP_SCHEMA: &str = "molten.operator.step.v1";
pub const OPERATOR_CHECKPOINT_SCHEMA: &str = "molten.operator.checkpoint.v1";
pub const OPERATOR_DOGFOOD_REPORT_SCHEMA: &str = "molten.operator.dogfood-report.v1";
pub const OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA: &str = "molten.operator.release-gate-receipt.v1";
pub const OPERATOR_NIX_DOGFOOD_EVIDENCE_SCHEMA: &str = "molten.operator.nix-dogfood-evidence.v1";
pub const OPERATOR_NIX_DOGFOOD_VERIFY_RECEIPT_SCHEMA: &str = "molten.operator.nix-dogfood-verify-receipt.v1";
pub const OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA: &str = "molten.operator.release-evidence-bundle.v1";
pub const OPERATOR_RELEASE_EVIDENCE_BUNDLE_VERIFY_RECEIPT_SCHEMA: &str =
    "molten.operator.release-evidence-bundle-verify-receipt.v1";
pub const OPERATOR_RELEASE_PROMOTION_GATE_RECEIPT_SCHEMA: &str = "molten.operator.release-promotion-gate-receipt.v1";
pub const OPERATOR_RELEASE_PROMOTION_SUMMARY_SCHEMA: &str = "molten.operator.release-promotion-summary.v1";
pub const OPERATOR_RELEASE_EXPORT_MANIFEST_SCHEMA: &str = "molten.operator.release-export-manifest.v1";
pub const OPERATOR_RELEASE_EXPORT_VERIFY_RECEIPT_SCHEMA: &str = "molten.operator.release-export-verify-receipt.v1";
pub const PLUGIN_MANIFEST_SCHEMA: &str = "molten.plugin.manifest.v1";
pub const PLUGIN_HOST_ABI_SCHEMA: &str = "molten.plugin.host-abi.v1";
pub const PLUGIN_HOST_ABI_RESULT_SCHEMA: &str = "molten.plugin.host-abi-result.v1";
pub const PLUGIN_INSTALL_RECEIPT_SCHEMA: &str = "molten.plugin.install-receipt.v1";
pub const PLUGIN_PERMISSION_RECEIPT_SCHEMA: &str = "molten.plugin.permission-receipt.v1";
pub const PLUGIN_LIFECYCLE_RECEIPT_SCHEMA: &str = "molten.plugin.lifecycle-receipt.v1";
pub const PLUGIN_HOSTCALL_RECEIPT_SCHEMA: &str = "molten.plugin.hostcall-receipt.v1";
pub const PLUGIN_HEALTH_RECEIPT_SCHEMA: &str = "molten.plugin.health-receipt.v1";
pub const PLUGIN_UPGRADE_RECEIPT_SCHEMA: &str = "molten.plugin.upgrade-receipt.v1";
pub const PLUGIN_REMOVAL_RECEIPT_SCHEMA: &str = "molten.plugin.removal-receipt.v1";
pub const COORDINATION_SERVICE_MANIFEST_SCHEMA: &str = "molten.coordination.service-manifest.v1";
pub const COORDINATION_REQUEST_SCHEMA: &str = "molten.coordination.request.v1";
pub const COORDINATION_RECEIPT_SCHEMA: &str = "molten.coordination.receipt.v1";
pub const COORDINATION_FENCING_TOKEN_SCHEMA: &str = "molten.coordination.fencing-token.v1";
pub const COORDINATION_STATE_SNAPSHOT_SCHEMA: &str = "molten.coordination.state-snapshot.v1";
pub const COORDINATION_STATUS_ASSERTION_SCHEMA: &str = "molten.coordination.status-assertion.v1";
pub const COORDINATION_APPLY_REPORT_SCHEMA: &str = "molten.coordination.apply-report.v1";
pub const CONFIDENTIAL_LABEL_SCHEMA: &str = "molten.secrets.confidential-label.v1";
pub const SECRET_REF_SCHEMA: &str = "molten.secrets.secret-ref.v1";
pub const ENCRYPTED_REF_SCHEMA: &str = "molten.secrets.encrypted-ref.v1";
pub const SECRET_REDACTION_MARKER_SCHEMA: &str = "molten.secrets.redaction-marker.v1";
pub const SECRET_REVEAL_RECEIPT_SCHEMA: &str = "molten.secrets.reveal-receipt.v1";
pub const SECRET_DECRYPT_RECEIPT_SCHEMA: &str = "molten.secrets.decrypt-receipt.v1";
pub const SECRET_REDACTION_TRANSFORM_RECEIPT_SCHEMA: &str = "molten.secrets.redaction-transform-receipt.v1";
pub const SECRET_CLEANUP_RECEIPT_SCHEMA: &str = "molten.secrets.cleanup-receipt.v1";
pub const SECRET_COMMITMENT_REPLAY_RECEIPT_SCHEMA: &str = "molten.secrets.commitment-replay-receipt.v1";
pub const PRIVATE_BUNDLE_PROFILE_SCHEMA: &str = "molten.secrets.private-bundle-profile.v1";
pub const PEER_BOOTSTRAP_INPUT_SCHEMA: &str = "molten.peer.bootstrap-input.v1";
pub const PEER_HANDSHAKE_SCHEMA: &str = "molten.peer.handshake.v1";
pub const PEER_AGREEMENT_SCHEMA: &str = "molten.peer.agreement.v1";
pub const PEER_BOOTSTRAP_RECEIPT_SCHEMA: &str = "molten.peer.bootstrap-receipt.v1";
pub const PROVENANCE_RECORD_SCHEMA: &str = "molten.provenance.record.v1";
pub const PROVENANCE_RECEIPT_SCHEMA: &str = "molten.provenance.receipt.v1";
pub const PROVENANCE_BUILD_RECORD_SCHEMA: &str = "molten.provenance.build-record.v1";
pub const PROVENANCE_BUILD_VERIFY_RECEIPT_SCHEMA: &str = "molten.provenance.build-verify-receipt.v1";
pub const AUTHORITY_IDENTITY_SCHEMA: &str = "molten.authority.identity.v1";
pub const AUTHORITY_CONTEXT_SCHEMA: &str = "molten.authority.context.v1";
pub const AUTHORITY_REVOCATION_SCHEMA: &str = "molten.authority.revocation.v1";
pub const AUTHORITY_RECEIPT_SCHEMA: &str = "molten.authority.receipt.v1";
pub const AUTHORITY_LIVE_REF_SCHEMA: &str = "molten.authority.live-ref.v1";
pub const RESOURCE_GRANT_SCHEMA: &str = "molten.resources.grant.v1";
pub const RESOURCE_CONSUMPTION_SCHEMA: &str = "molten.resources.consumption.v1";
pub const RESOURCE_RECEIPT_SCHEMA: &str = "molten.resources.receipt.v1";
pub const RESOURCE_SCHEDULER_SCHEMA: &str = "molten.resources.scheduler.v1";
pub const SERVICE_MANIFEST_SCHEMA: &str = "molten.service.manifest.v1";
pub const SERVICE_DEMAND_SCHEMA: &str = "molten.service.demand.v1";
pub const SERVICE_STATUS_SCHEMA: &str = "molten.service.status.v1";
pub const SERVICE_SUPERVISOR_SCHEMA: &str = "molten.service.supervisor.v1";
pub const SERVICE_LINK_SCHEMA: &str = "molten.service.link.v1";
pub const SERVICE_MONITOR_SCHEMA: &str = "molten.service.monitor.v1";
pub const SERVICE_RESTART_POLICY_SCHEMA: &str = "molten.service.restart-policy.v1";
pub const SERVICE_RESTART_DECISION_SCHEMA: &str = "molten.service.restart-decision.v1";
pub const SERVICE_LIFECYCLE_RECEIPT_SCHEMA: &str = "molten.service.lifecycle-receipt.v1";
pub const SERVICE_CLEANUP_RECEIPT_SCHEMA: &str = "molten.service.cleanup-receipt.v1";
pub const SERVICE_SUPERVISION_SUITE_SCHEMA: &str = "molten.service.supervision-suite.v1";
pub const SERVICE_SUPERVISION_REPORT_SCHEMA: &str = "molten.service.supervision-report.v1";
pub const SERVICE_SUPERVISION_GATE_RECEIPT_SCHEMA: &str = "molten.service.supervision-gate-receipt.v1";
pub const SERVICE_MONITOR_NOTIFICATION_SCHEMA: &str = "molten.service.monitor-notification.v1";
pub const SERVICE_FAILURE_MARKER_SCHEMA: &str = "molten.service.failure.v1";
pub const SERVICE_RETRACTION_SCHEMA: &str = "molten.service.retraction.v1";
pub const SERVICE_RETENTION_INPUT_SCHEMA: &str = "molten.service.retention-input.v1";
pub const SERVICE_OWNED_STATE_SCHEMA: &str = "molten.service.owned-state.v1";
pub const SERVICE_RUNTIME_SUITE_SCHEMA: &str = "molten.service.runtime-suite.v1";
pub const SERVICE_RUNTIME_REPORT_SCHEMA: &str = "molten.service.runtime-report.v1";
pub const SERVICE_READINESS_ASSERTION_SCHEMA: &str = "molten.service.readiness.v1";
pub const SERVICE_REPLAY_IDENTITY_SCHEMA: &str = "molten.service.replay-identity.v1";
pub const SERVICE_TURN_CONTEXT_SCHEMA: &str = "molten.service.turn-context.v1";
pub const PROTOCOL_MANIFEST_SCHEMA: &str = "molten.protocol.manifest.v1";
pub const PROTOCOL_INSTALL_RECEIPT_SCHEMA: &str = "molten.protocol.install-receipt.v1";
pub const PROTOCOL_ENDPOINT_SCHEMA: &str = "molten.protocol.endpoint.v1";
pub const PROTOCOL_LOCAL_STATE_SCHEMA: &str = "molten.protocol.local-state.v1";
pub const PROTOCOL_SESSION_STATE_SCHEMA: &str = "molten.protocol.session-state.v1";
pub const PROTOCOL_MESSAGE_SCHEMA: &str = "molten.protocol.message.v1";
pub const PROTOCOL_OPERATION_RECEIPT_SCHEMA: &str = "molten.protocol.operation-receipt.v1";
pub const PROTOCOL_SESSION_GATE_RECEIPT_SCHEMA: &str = "molten.protocol.session-gate-receipt.v1";
pub const RAFT_GROUP_MANIFEST_SCHEMA: &str = "molten.raft.group-manifest.v1";
pub const RAFT_COMMAND_ENVELOPE_SCHEMA: &str = "molten.raft.command-envelope.v1";
pub const RAFT_LOG_ENTRY_SCHEMA: &str = "molten.raft.log-entry.v1";
pub const RAFT_COMMIT_RECEIPT_SCHEMA: &str = "molten.raft.commit-receipt.v1";
pub const RAFT_READ_RECEIPT_SCHEMA: &str = "molten.raft.read-receipt.v1";
pub const RAFT_SNAPSHOT_SCHEMA: &str = "molten.raft.snapshot.v1";
pub const RAFT_RECOVERY_RECEIPT_SCHEMA: &str = "molten.raft.recovery-receipt.v1";
pub const RAFT_PREDICATE_RECEIPT_SCHEMA: &str = "molten.raft.predicate-receipt.v1";
pub const CONTROL_REGISTRY_COMMAND_SCHEMA: &str = "molten.control-registry.command.v1";
pub const CONTROL_REGISTRY_STATE_SCHEMA: &str = "molten.control-registry.state.v1";
pub const CONTROL_REGISTRY_RECEIPT_SCHEMA: &str = "molten.control-registry.receipt.v1";
pub const TYPED_STORAGE_REF_SCHEMA: &str = "molten.storage.typed-ref.v1";
pub const TYPED_STORAGE_RECEIPT_SCHEMA: &str = "molten.storage.receipt.v1";
pub const TYPED_STORAGE_EFFECT_MANIFEST_SCHEMA: &str = "molten.storage.effect-manifest.v1";
pub const TYPED_STORAGE_SCHEMA_ARTIFACT_SCHEMA: &str = "molten.storage.schema-artifact.v1";
pub const TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA: &str = "molten.storage.migration-recipe.v1";
pub const ARTIFACT_SCHEMA: &str = "molten.artifacts.artifact.v1";
pub const ARTIFACT_NAME_POINTER_SCHEMA: &str = "molten.artifacts.name-pointer.v1";
pub const ARTIFACT_RECEIPT_SCHEMA: &str = "molten.artifacts.receipt.v1";
pub const ARTIFACT_CLOSURE_SCHEMA: &str = "molten.artifacts.closure.v1";
pub const SCHEMA_IDENTITY_SCHEMA: &str = "molten.schema.identity.v1";
pub const SCHEMA_ALIAS_SCHEMA: &str = "molten.schema.alias.v1";
pub const SCHEMA_COMPATIBILITY_SCHEMA: &str = "molten.schema.compatibility.v1";
pub const SCHEMA_COMPATIBILITY_RECEIPT_SCHEMA: &str = "molten.schema.compatibility-receipt.v1";
pub const SCHEMA_STRUCTURAL_FINGERPRINT_SCHEMA: &str = "molten.schema.structural-fingerprint.v1";
pub const EVAL_CACHE_KEY_SCHEMA: &str = "molten.eval-cache.key.v1";
pub const EVAL_CACHE_VALUE_SCHEMA: &str = "molten.eval-cache.value.v1";
pub const EVAL_CACHE_RECEIPT_SCHEMA: &str = "molten.eval-cache.receipt.v1";
pub const TRANSCRIPT_ARTIFACT_SCHEMA: &str = "molten.transcript.artifact.v1";
pub const TRANSCRIPT_STANZA_SCHEMA: &str = "molten.transcript.stanza.v1";
pub const TRANSCRIPT_STANZA_OUTCOME_SCHEMA: &str = "molten.transcript.stanza-outcome.v1";
pub const TRANSCRIPT_RUN_RECEIPT_SCHEMA: &str = "molten.transcript.run-receipt.v1";
pub const REWRITE_QUERY_SCHEMA: &str = "molten.rewrite.query.v1";
pub const REWRITE_MATCH_SCHEMA: &str = "molten.rewrite.match.v1";
pub const REWRITE_DIFF_SCHEMA: &str = "molten.rewrite.diff.v1";
pub const REWRITE_PLAN_SCHEMA: &str = "molten.rewrite.plan.v1";
pub const REWRITE_RECEIPT_SCHEMA: &str = "molten.rewrite.receipt.v1";
pub const CATALOG_SUMMARY_SCHEMA: &str = "molten.catalog.summary.v1";
pub const CATALOG_VIEW_SCHEMA: &str = "molten.catalog.view.v1";
pub const CATALOG_QUERY_SCHEMA: &str = "molten.catalog.query.v1";
pub const CATALOG_RESULT_SCHEMA: &str = "molten.catalog.result.v1";
pub const CATALOG_RECEIPT_SCHEMA: &str = "molten.catalog.receipt.v1";
pub const CATALOG_SHORT_ID_SCHEMA: &str = "molten.catalog.short-id-resolution.v1";
pub const CATALOG_MCP_REQUEST_SCHEMA: &str = "molten.catalog.mcp-request.v1";
pub const CATALOG_MCP_RESPONSE_SCHEMA: &str = "molten.catalog.mcp-response.v1";
pub const CATALOG_MCP_RECEIPT_SCHEMA: &str = "molten.catalog.mcp-receipt.v1";
pub const JOB_DAG_SCHEMA: &str = "molten.job-dag.dag.v1";
pub const JOB_DAG_NODE_SCHEMA: &str = "molten.job-dag.node.v1";
pub const JOB_DAG_EDGE_SCHEMA: &str = "molten.job-dag.edge.v1";
pub const JOB_DAG_OUTPUT_REQUEST_SCHEMA: &str = "molten.job-dag.output-request.v1";
pub const JOB_DAG_RECEIPT_SCHEMA: &str = "molten.job-dag.receipt.v1";
pub const JOB_STAGE_OPERATION_SCHEMA: &str = "molten.job-dag.stage-operation.v1";
pub const JOB_PLAN_SCHEMA: &str = "molten.job-dag.plan.v1";
pub const JOB_PROFILE_SCHEMA: &str = "molten.job-dag.profile.v1";
pub const JOB_FUSION_PLAN_SCHEMA: &str = "molten.job-dag.fusion-plan.v1";
pub const JOB_PLAN_RECEIPT_SCHEMA: &str = "molten.job-dag.plan-receipt.v1";
pub const JOB_PROFILE_RECEIPT_SCHEMA: &str = "molten.job-dag.profile-receipt.v1";
pub const JOB_FUSION_RECEIPT_SCHEMA: &str = "molten.job-dag.fusion-receipt.v1";
pub const JOB_SYNC_REQUEST_SCHEMA: &str = "molten.job-dag.sync-request.v1";
pub const JOB_SYNC_PLAN_SCHEMA: &str = "molten.job-dag.sync-plan.v1";
pub const JOB_SYNC_RECEIPT_SCHEMA: &str = "molten.job-dag.sync-receipt.v1";
pub const JOB_ADMISSION_REQUEST_SCHEMA: &str = "molten.job-dag.admission-request.v1";
pub const JOB_ADMISSION_PLAN_SCHEMA: &str = "molten.job-dag.admission-plan.v1";
pub const JOB_ADMISSION_RECEIPT_SCHEMA: &str = "molten.job-dag.admission-receipt.v1";
pub const JOB_EXECUTION_REQUEST_SCHEMA: &str = "molten.job-dag.execution-request.v1";
pub const JOB_EXECUTION_RECEIPT_SCHEMA: &str = "molten.job-dag.execution-receipt.v1";
pub const JOB_REF_SUBMISSION_SCHEMA: &str = "molten.job-dag.blob-ref-submission.v1";
pub const JOB_REF_STATUS_SCHEMA: &str = "molten.job-dag.blob-ref-status.v1";
pub const JOB_REF_RECEIPT_SCHEMA: &str = "molten.job-dag.blob-ref-receipt.v1";
pub const JOB_WORKER_REQUEST_SCHEMA: &str = "molten.job-dag.worker-request.v1";
pub const JOB_WORKER_ASSIGNMENT_SCHEMA: &str = "molten.job-dag.worker-assignment.v1";
pub const JOB_WORKER_STATUS_SCHEMA: &str = "molten.job-dag.worker-status.v1";
pub const JOB_WORKER_RESULT_SCHEMA: &str = "molten.job-dag.worker-result.v1";
pub const JOB_WORKER_RECEIPT_SCHEMA: &str = "molten.job-dag.worker-receipt.v1";
pub const JOB_WORKER_SCHEDULE_RECEIPT_SCHEMA: &str = "molten.job-dag.worker-schedule-receipt.v1";
pub const UPGRADE_PLAN_SCHEMA: &str = "molten.upgrade.plan.v1";
pub const UPGRADE_RECEIPT_SCHEMA: &str = "molten.upgrade.receipt.v1";
pub const UPGRADE_NAME_POINTER_SCHEMA: &str = "molten.upgrade.name-pointer.v1";
pub const CHUNK_MANIFEST_SCHEMA: &str = "molten.chunk-store.manifest.v1";
pub const CHUNK_REF_SCHEMA: &str = "molten.chunk-store.chunk-ref.v1";
pub const CHUNK_ROOT_SCHEMA: &str = "molten.chunk-store.chunk-root.v1";
pub const CHUNK_STORE_RECEIPT_SCHEMA: &str = "molten.chunk-store.receipt.v1";
pub const CHUNK_LINEAGE_SCHEMA: &str = "molten.chunk-store.lineage.v1";
pub const OCTET_GATE_POLICY_SCHEMA: &str = "molten.octet.gate-policy.v1";
pub const OCTET_GATE_RECEIPT_SCHEMA: &str = "molten.octet.gate-receipt.v1";
pub const OCTET_STRUCTURED_FINDINGS_SCHEMA: &str = "molten.octet.structured-findings.v1";
pub const OCTET_FINGERPRINT_EVIDENCE_SCHEMA: &str = "molten.octet.fingerprint-evidence.v1";
pub const OCTET_COMMAND_ARTIFACT_SCHEMA: &str = "molten.octet.command-artifact.v1";
pub const OCTET_STATUS_ARTIFACT_SCHEMA: &str = "molten.octet.status-artifact.v1";
pub const OCTET_SUMMARY_ARTIFACT_SCHEMA: &str = "molten.octet.summary-artifact.v1";
pub const OCTET_OBJECT_CORPUS_ARTIFACT_SCHEMA: &str = "molten.octet.object-corpus-artifact.v1";
pub const OCTET_ARTIFACT_LEDGER_RECEIPT_SCHEMA: &str = "molten.octet.artifact-ledger-receipt.v1";
pub const OCTET_WARNING_BASELINE_SCHEMA: &str = "molten.octet.warning-baseline.v1";
pub const OCTET_BASELINE_RECEIPT_SCHEMA: &str = "molten.octet.baseline-receipt.v1";
pub const OCTET_REVIEW_MANIFEST_SCHEMA: &str = "molten.octet.review-manifest.v1";
pub const OCTET_SOURCE_GATE_REQUIREMENT_SCHEMA: &str = "molten.octet.source-gate-requirement.v1";
pub const OCTET_SOURCE_GATE_VALIDATION_SCHEMA: &str = "molten.octet.source-gate-validation.v1";
pub const OCTET_REMEDIATION_PLAN_SCHEMA: &str = "molten.octet.remediation-plan.v1";
pub const HASH_ALGORITHM: &str = "blake3-preserves-packed-v1";
const BLAKE3_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentRef(String);

impl ContentRef {
    pub fn parse(value: &str) -> Result<Self> {
        validate_content_ref(value)?;
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

pub fn validate_content_ref(value: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return Err(MoltenError::invalid_harness(format!(
            "content ref must start with {BLAKE3_REF_PREFIX}, got {value}"
        )));
    };
    validate_content_ref_hex(value, hex)
}

pub fn content_ref_has_prefix(value: &str) -> bool {
    value.starts_with(BLAKE3_REF_PREFIX)
}

pub fn content_ref_hex(value: &str) -> Result<&str> {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return Err(MoltenError::invalid_harness(format!(
            "content ref must start with {BLAKE3_REF_PREFIX}, got {value}"
        )));
    };
    validate_content_ref_hex(value, hex)?;
    Ok(hex)
}

pub fn content_ref_from_hex(hex: &str) -> Result<String> {
    let reference = format!("{BLAKE3_REF_PREFIX}{hex}");
    validate_content_ref_hex(&reference, hex)?;
    Ok(reference)
}

fn validate_content_ref_hex(value: &str, hex: &str) -> Result<()> {
    if hex.len() != BLAKE3_HEX_LEN {
        return Err(MoltenError::invalid_harness(format!(
            "content ref must be {BLAKE3_REF_PREFIX}<64 lowercase hex chars>, got {value}"
        )));
    }
    if !hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(MoltenError::invalid_harness(format!("content ref must use lowercase hex chars, got {value}")));
    }
    Ok(())
}

pub fn parse_text(source: &str) -> Result<IOValue> {
    preserves::read_iovalue_text(source, false).map_err(|error| MoltenError::Preserves(error.to_string()))
}

pub fn to_text(value: &IOValue) -> Result<String> {
    preserves::write_iovalue_text(value, false).map_err(|error| MoltenError::Preserves(error.to_string()))
}

pub fn canonical_bytes(value: &IOValue) -> Result<Vec<u8>> {
    preserves::write_iovalue_packed(value, false).map_err(|error| MoltenError::Preserves(error.to_string()))
}

pub fn parse_canonical_bytes(bytes: &[u8]) -> Result<IOValue> {
    preserves::read_iovalue_packed(bytes, false).map_err(|error| MoltenError::Preserves(error.to_string()))
}

pub fn canonical_hash(value: &IOValue) -> Result<String> {
    let bytes = canonical_bytes(value)?;
    Ok(content_ref_from_bytes(&bytes))
}

pub fn content_ref_from_bytes(bytes: &[u8]) -> String {
    content_ref_from_blake3_hash(blake3::hash(bytes))
}

pub fn content_ref_from_blake3_hash(hash: blake3::Hash) -> String {
    format!("{BLAKE3_REF_PREFIX}{}", hash.to_hex())
}

pub fn canonical_content_ref(value: &IOValue) -> Result<ContentRef> {
    ContentRef::parse(&canonical_hash(value)?)
}

pub fn symbol(name: &'static str) -> IOValue {
    IOValue::symbol(name)
}

pub fn string(value: impl AsRef<str>) -> IOValue {
    IOValue::new(value.as_ref().to_owned())
}

pub fn u64_value(value: u64) -> IOValue {
    IOValue::new(value)
}

pub fn bool_value(value: bool) -> IOValue {
    IOValue::new(value)
}

pub fn sequence(values: Vec<IOValue>) -> IOValue {
    IOValue::new(values)
}

pub fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    IOValue::record(symbol(label), fields)
}

pub fn value_to_iovalue(value: &Value<IOValue>) -> IOValue {
    IOValue::from(value.clone())
}

#[cfg(test)]
mod tests {
    use super::ContentRef;
    use super::canonical_content_ref;
    use super::canonical_hash;
    use super::content_ref_from_hex;
    use super::parse_text;
    use super::to_text;
    use super::validate_content_ref;

    #[test]
    fn preserves_text_roundtrip_keeps_hash() {
        let value = parse_text("<example \"a\" [1 2 3]>").expect("parse initial text");
        let hash = canonical_hash(&value).expect("hash initial value");
        let rendered = to_text(&value).expect("render preserves text");
        let reparsed = parse_text(&rendered).expect("parse rendered text");
        assert_eq!(hash, canonical_hash(&reparsed).expect("hash reparsed value"));
    }

    #[test]
    fn content_ref_parser_rejects_non_canonical_shapes() {
        let valid = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        validate_content_ref(valid).expect("valid ref");
        let parsed = ContentRef::parse(valid).expect("parsed ref");
        assert_eq!(parsed.as_str(), valid);
        assert_eq!(parsed.into_string(), valid);
        assert_eq!(
            content_ref_from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .expect("ref from hex"),
            valid
        );

        for invalid in [
            "",
            "blake3:",
            "blake3:fixture",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde/",
        ] {
            assert!(validate_content_ref(invalid).is_err(), "invalid ref accepted: {invalid}");
        }
    }

    #[test]
    fn canonical_content_ref_matches_canonical_hash() {
        let value = parse_text("<content-ref-fixture [#t 42]>").expect("parse fixture");
        let reference = canonical_content_ref(&value).expect("canonical content ref");
        assert_eq!(reference.as_str(), canonical_hash(&value).expect("canonical hash"));
    }
}
