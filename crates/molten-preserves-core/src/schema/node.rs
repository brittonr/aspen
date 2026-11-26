//! Schema ids of node identity and control plane, delivery idempotency, remote dataspaces,
//! federation, peers, authority and secrets.

schema_ids! {
    table NODE_ENTRIES;
    DELIVERY_OPERATION_ID_SCHEMA = "molten.delivery.operation-id.v1";
    DELIVERY_SCOPE_PROFILE_SCHEMA = "molten.delivery.scope-profile.v1";
    DELIVERY_WINDOW_SCHEMA = "molten.delivery.window.v1";
    DELIVERY_DEDUP_ENTRY_SCHEMA = "molten.delivery.dedup-entry.v1";
    DELIVERY_IDEMPOTENCY_RECEIPT_SCHEMA = "molten.delivery.idempotency-receipt.v1";
    DELIVERY_RETRY_RECEIPT_SCHEMA = "molten.delivery.retry-receipt.v1";
    REMOTE_DATASPACE_ENVELOPE_SCHEMA = "molten.remote-dataspace.envelope.v1";
    REMOTE_DATASPACE_TRANSPORT_RECEIPT_SCHEMA = "molten.remote-dataspace.transport-receipt.v1";
    REMOTE_DATASPACE_ADMISSION_RECEIPT_SCHEMA = "molten.remote-dataspace.admission-receipt.v1";
    REMOTE_DATASPACE_DELIVERY_LOG_SCHEMA = "molten.remote-dataspace.delivery-log.v1";
    REMOTE_DATASPACE_GATE_RECEIPT_SCHEMA = "molten.remote-dataspace.gate-receipt.v1";
    REMOTE_DATASPACE_SESSION_SCHEMA = "molten.remote-dataspace.session.v1";
    REMOTE_DATASPACE_APPLIED_ASSERTION_SCHEMA = "molten.remote-dataspace.applied-assertion.v1";
    FEDERATION_ANNOUNCEMENT_SCHEMA = "molten.federation.announcement.v1";
    FEDERATION_INVENTORY_SCHEMA = "molten.federation.inventory.v1";
    FEDERATION_RECEIPT_SCHEMA = "molten.federation.receipt.v1";
    NODE_IDENTITY_SCHEMA = "molten.node.identity.v1";
    NODE_IDENTITY_RECEIPT_SCHEMA = "molten.node.identity-receipt.v1";
    NODE_IDENTITY_BOOTSTRAP_SCHEMA = "molten.node.identity-bootstrap.v1";
    NODE_IDENTITY_STARTUP_SCHEMA = "molten.node.identity-startup.v1";
    NODE_CONFIG_SCHEMA = "molten.node.config.v1";
    NODE_STARTUP_RECEIPT_SCHEMA = "molten.node.startup-receipt.v1";
    NODE_ADAPTER_RECEIPT_SCHEMA = "molten.node.adapter-receipt.v1";
    NODE_CONTROL_REQUEST_SCHEMA = "molten.node.control-request.v1";
    NODE_CONTROL_RECEIPT_SCHEMA = "molten.node.control-receipt.v1";
    NODE_CONTROL_LOCK_SCHEMA = "molten.node.control-lock.v1";
    NODE_CONTROL_LOCK_SCHEMA_V2 = "molten.node.control-lock.v2";
    NODE_CONTROL_RUN_SCHEMA = "molten.node.control-run.v1";
    NODE_CONTROL_RUN_SHUTDOWN_SCHEMA = "molten.node.control-run-shutdown.v1";
    NODE_CONTROL_QUEUE_RECEIPT_SCHEMA = "molten.node.control-queue-receipt.v1";
    NODE_CONTROL_OPERATION_RECEIPT_SCHEMA = "molten.node.control-operation-receipt.v1";
    NODE_CONTROL_HEARTBEAT_RECEIPT_SCHEMA = "molten.node.control-heartbeat-receipt.v1";
    NODE_CONTROL_LOOP_RECEIPT_SCHEMA = "molten.node.control-loop-receipt.v1";
    NODE_CONTROL_SERVICE_LOCK_SCHEMA = "molten.node.control-service-lock.v1";
    NODE_CONTROL_SERVICE_HEARTBEAT_RECEIPT_SCHEMA = "molten.node.control-service-heartbeat-receipt.v1";
    NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA = "molten.node.control-service-run-receipt.v1";
    NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA = "molten.node.control-supervisor-policy.v1";
    NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA = "molten.node.control-supervisor-receipt.v1";
    NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA = "molten.node.control-ingress-envelope.v1";
    NODE_CONTROL_INGRESS_RECEIPT_SCHEMA = "molten.node.control-ingress-receipt.v1";
    NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA = "molten.node.control-live-transport-receipt.v1";
    NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA = "molten.node.control-live-send-receipt.v1";
    NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA = "molten.node.control-live-send-retry-receipt.v1";
    NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA = "molten.node.control-live-send-duplicate-receipt.v1";
    NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA = "molten.node.control-live-workflow-receipt.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA = "molten.node.control-live-workflow-bundle.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA =
        "molten.node.control-live-workflow-bundle-export-receipt.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA =
        "molten.node.control-live-workflow-bundle-import-receipt.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA =
        "molten.node.control-live-workflow-bundle-verify-receipt.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA = "molten.node.control-live-workflow-bundle-gate-receipt.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA =
        "molten.node.control-live-workflow-bundle-apply-receipt.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA =
        "molten.node.control-live-workflow-bundle-reconcile-receipt.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA = "molten.node.control-live-workflow-bundle-ack.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_EXPORT_RECEIPT_SCHEMA =
        "molten.node.control-live-workflow-bundle-ack-export-receipt.v1";
    NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_IMPORT_RECEIPT_SCHEMA =
        "molten.node.control-live-workflow-bundle-ack-import-receipt.v1";
    NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA = "molten.node.control-live-listener-receipt.v1";
    NODE_CONTROL_AUTHORITY_GRANT_SCHEMA = "molten.node.control-authority-grant.v1";
    NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA = "molten.node.control-authority-receipt.v1";
    NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA = "molten.node.control-authority-grant-import-receipt.v1";
    NODE_CONTROL_LIVE_TICKET_SCHEMA = "molten.node.control-live-ticket.v1";
    NODE_CONTROL_LIVE_PEER_ADMISSION_SCHEMA = "molten.node.control-live-peer-admission.v1";
    NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA = "molten.node.control-live-ticket-import-receipt.v1";
    NODE_HEALTH_RECEIPT_SCHEMA = "molten.node.health-receipt.v1";
    NODE_SHUTDOWN_RECEIPT_SCHEMA = "molten.node.shutdown-receipt.v1";
    CONFIDENTIAL_LABEL_SCHEMA = "molten.secrets.confidential-label.v1";
    SECRET_REF_SCHEMA = "molten.secrets.secret-ref.v1";
    ENCRYPTED_REF_SCHEMA = "molten.secrets.encrypted-ref.v1";
    SECRET_REDACTION_MARKER_SCHEMA = "molten.secrets.redaction-marker.v1";
    SECRET_REVEAL_RECEIPT_SCHEMA = "molten.secrets.reveal-receipt.v1";
    SECRET_DECRYPT_RECEIPT_SCHEMA = "molten.secrets.decrypt-receipt.v1";
    SECRET_REDACTION_TRANSFORM_RECEIPT_SCHEMA = "molten.secrets.redaction-transform-receipt.v1";
    SECRET_CLEANUP_RECEIPT_SCHEMA = "molten.secrets.cleanup-receipt.v1";
    SECRET_COMMITMENT_REPLAY_RECEIPT_SCHEMA = "molten.secrets.commitment-replay-receipt.v1";
    PRIVATE_BUNDLE_PROFILE_SCHEMA = "molten.secrets.private-bundle-profile.v1";
    PEER_BOOTSTRAP_INPUT_SCHEMA = "molten.peer.bootstrap-input.v1";
    PEER_HANDSHAKE_SCHEMA = "molten.peer.handshake.v1";
    PEER_AGREEMENT_SCHEMA = "molten.peer.agreement.v1";
    PEER_BOOTSTRAP_RECEIPT_SCHEMA = "molten.peer.bootstrap-receipt.v1";
    AUTHORITY_IDENTITY_SCHEMA = "molten.authority.identity.v1";
    AUTHORITY_CONTEXT_SCHEMA = "molten.authority.context.v1";
    AUTHORITY_REVOCATION_SCHEMA = "molten.authority.revocation.v1";
    AUTHORITY_RECEIPT_SCHEMA = "molten.authority.receipt.v1";
    AUTHORITY_LIVE_REF_SCHEMA = "molten.authority.live-ref.v1";
}
