mod compatibility;
mod interruptions;
mod negative;
mod positive;

use super::super::*;

struct Case {
    workspace: crate::test_support::ProcessWorkspace,
    envelope: ControlIngressEnvelope,
}

impl Case {
    fn new(label: &str) -> Self {
        let workspace = crate::test_support::ProcessWorkspace::new(label).expect("owned workspace");
        init_local(&InitInput {
            state_root: &workspace,
            node_id: "node:recovery",
        })
        .expect("initialize receiver");
        let authority = [local_ref("authority", "recovery").expect("authority")];
        let policy = [local_ref("policy", "recovery").expect("policy")];
        let resource = [local_ref("resource", "recovery").expect("resource")];
        let bootstrap = [local_ref("bootstrap", "recovery").expect("bootstrap")];
        let request = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &authority,
            policy_refs: &policy,
            resource_refs: &resource,
            evidence_refs: &[],
        })
        .expect("control request");
        let envelope = control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request,
            from_peer: "peer:operator",
            to_node: "node:recovery",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &bootstrap,
            authority_refs: &authority,
            policy_refs: &policy,
            resource_refs: &resource,
            evidence_refs: &[],
        })
        .expect("envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &workspace,
            envelope_value: &envelope.value,
        })
        .expect("persist envelope");
        Self { workspace, envelope }
    }

    fn reopen(&self) -> crate::node_state::DirectoryRoot {
        crate::node_state::DirectoryRoot::open_existing(&self.workspace).expect("reopen receiver")
    }

    fn start_fixture(&self) {
        // The repository's explicit local fixture uses synthetic source-gate evidence.
        // It is not release/startup-policy evidence for F03.
        run_local_with_root(&self.reopen()).expect("synthetic local startup fixture");
    }

    fn dedup(&self) -> crate::delivery_idempotency::Decision {
        let root = self.reopen();
        let delivery = root.delivery_store().expect("delivery store");
        let scope = crate::delivery_idempotency::remote_topic_scope_ref(&self.envelope.topic, &self.envelope.to_node)
            .expect("scope");
        let evidence = ingress_idempotency_evidence_refs(&self.envelope);
        crate::delivery_idempotency::check_with_root(crate::delivery_idempotency::CapabilityCheckInput {
            root: &delivery,
            request: crate::delivery_idempotency::CheckRequest {
                scope_profile: crate::delivery_idempotency::SCOPE_REMOTE_TOPIC,
                scope_ref: &scope,
                producer: &self.envelope.from_peer,
                consumer: &self.envelope.to_node,
                sequence: self.envelope.sequence,
                intent: "node-control-ingress",
                payload_ref: &self.envelope.request.request_ref,
                policy_refs: &self.envelope.policy_refs,
                evidence_refs: &evidence,
                semantic_result_ref: Some(&self.envelope.request.request_ref),
                gap_policy: crate::delivery_idempotency::GapPolicy::Deny,
            },
        })
        .expect("observe persisted dedup identity")
    }

    fn reserve(&self) -> crate::delivery_idempotency::Decision {
        let root = self.reopen();
        let delivery = root.delivery_store().expect("delivery store");
        let mut store = crate::delivery_idempotency::PublicationStore::open(&delivery).expect("journal");
        let decision = super::reserve(&mut store, &self.envelope).expect("atomic dedup and intent reservation");
        assert!(decision.should_commit_side_effect);
        assert_eq!(decision.receipt.decision, "first");
        decision
    }

    fn set_metadata(&self, metadata: &IoValue) {
        let root = self.reopen();
        let delivery = root.delivery_store().expect("delivery store");
        let mut store = crate::delivery_idempotency::PublicationStore::open(&delivery).expect("journal");
        let intent = store
            .read(&self.envelope.operation_ref, &self.envelope.request.request_ref)
            .expect("read intent")
            .expect("reserved intent");
        store.update(&intent, metadata).expect("persist controlled crash boundary");
    }

    fn set_phase(&self, phase: crate::core_api::node_ingress::PublicationPhase) {
        self.set_metadata(&super::state::metadata(phase));
    }

    fn write_inbox(&self) {
        let root = self.reopen();
        write_preserves(
            &root,
            &control_inbox_path(&self.envelope.request.request_ref).expect("inbox path"),
            &self.envelope.request.value,
        )
        .expect("persist exact inbox before receipt");
    }

    fn deliver(&self) -> ControlIngressDeliver {
        deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: &self.workspace,
            topic: &self.envelope.topic,
            envelope_ref: &self.envelope.envelope_ref,
        })
        .expect("reopen and reconcile delivery")
    }

    fn dispatch(&self) -> Result<ControlDispatch> {
        dispatch_control_request_with_root(
            &self.reopen(),
            Some(&control_inbox_entry_name(&self.envelope.request.request_ref)),
        )
    }

    fn read(&self, path: &crate::node_state::RelativePath) -> Option<Vec<u8>> {
        let root = self.reopen();
        if root.try_exists(path).expect("observe fixture path") {
            Some(root.read(path, crate::node_state::MAX_NODE_STATE_FILE_BYTES).expect("read persisted fact"))
        } else {
            None
        }
    }

    fn facts(&self) -> Vec<Option<Vec<u8>>> {
        let request = &self.envelope.request.request_ref;
        [
            control_inbox_path(request).expect("inbox"),
            queue_receipt_path(request).expect("queue"),
            control_outbox_request_path(request).expect("archive"),
            control_outbox_receipt_path(request).expect("result"),
            dispatch_receipt_path(request).expect("dispatch"),
            control_operation_receipt_path(request).expect("operation result"),
            fixed_node_path(HEALTH_FILE).expect("health"),
            fixed_node_path(CONTROL_STATUS_FILE).expect("status effect"),
            fixed_node_path(CONTROL_LOCK_FILE).expect("active lock"),
        ]
        .iter()
        .map(|path| self.read(path))
        .collect()
    }

    fn assert_dedup_preserved(&self, prior: &crate::delivery_idempotency::Decision) {
        let observed = self.dedup();
        assert_eq!(observed.receipt.decision, "duplicate");
        assert!(!observed.should_commit_side_effect);
        assert_eq!(observed.entry, prior.entry);
        assert_eq!(observed.window, prior.window);
        assert_eq!(observed.operation, prior.operation);
    }

    fn assert_denied(&self) -> ControlIngressReceipt {
        let delivered = self.deliver();
        let receipt = parse_control_ingress_receipt(&delivered.ingress_receipt_value).expect("ingress receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(!delivered.has_enqueued);
        assert!(delivered.queue_receipt_ref.is_none());
        assert!(!receipt.diagnostics.is_empty());
        receipt
    }

    fn assert_no_pending(&self) {
        assert!(next_pending_control_request(&self.reopen()).expect("pending scan").is_none());
    }

    fn queue(&self) -> ControlQueueReceipt {
        let value = read_preserves(
            &self.reopen(),
            &queue_receipt_path(&self.envelope.request.request_ref).expect("queue path"),
        )
        .expect("persisted queue receipt");
        parse_control_queue_receipt(&value).expect("queue receipt")
    }
}
