
    fn fake_live_send_receipt(
        from_peer: &str,
        to_node: &str,
        envelope_label: &str,
        transport_ref: &str,
        ticket_label: &str,
    ) -> IoValue {
        crate::preserves_rail::record("node-control-live-send-receipt-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA),
            crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
            crate::preserves_rail::record("transport", vec![crate::preserves_rail::string("iroh-gossip")]),
            crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(
                crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
            )]),
            crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(from_peer)]),
            crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(to_node)]),
            crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(fake_ref(
                ticket_label,
            ))]),
            crate::preserves_rail::record("receiver-endpoint", vec![crate::preserves_rail::string(to_node)]),
            crate::preserves_rail::record("receiver-addresses", vec![crate::preserves_rail::sequence(vec![
                crate::preserves_rail::string(fake_ref(&format!("{ticket_label}-address"))),
            ])]),
            crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(fake_ref(envelope_label))]),
            crate::preserves_rail::record("transport-receipt", vec![optional_ref_value(Some(transport_ref))]),
            crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(Vec::new())]),
            checks_value(&[
                ("receiver-ticket-bound", "pass"),
                ("receiver-address-bound", "pass"),
                ("receiver-address-supported", "pass"),
                ("receiver-ticket-expected", "pass"),
                ("operation-id-bound", "pass"),
                ("sender-state-root-evidence", "pass"),
                ("join-or-publish-succeeded", "pass"),
                ("canonical-envelope-ref", "pass"),
                ("live-iroh-gossip", "pass"),
                ("transport-is-not-authority", "pass"),
                ("durable-inbox-boundary", "pass"),
            ]),
        ])
    }

    fn store_test_remote_clearance(input: TestRemoteClearanceInput<'_>) -> String {
        store_remote_gc_clearance(input.root, &RemoteGcClearanceInput {
            decision: "pass",
            requester_ref: input.requester_ref,
            peer_ref: input.peer_ref,
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            remote_ref: input.remote_ref,
            policy_ref: input.policy_ref,
            authority_ref: input.authority_ref,
            evidence_refs: &[fake_ref(input.label)],
            retained_refs: input.retained_refs,
            is_current: input.is_current,
            revoked_refs: input.revoked_refs,
            diagnostics: &[],
        })
        .expect("store test remote clearance")
        .clearance_ref
    }

    fn scoped_ref(
        root: &Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        is_current: bool,
        revoked_refs: &[String],
    ) -> String {
        store_test_admission(TestAdmissionInput {
            root,
            kind,
            label,
            requester_ref,
            object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current,
            revoked_refs,
        })
    }

    fn store_test_admission(input: TestAdmissionInput<'_>) -> String {
        store_evidence_admission(input.root, &EvidenceAdmissionInput {
            kind: input.kind,
            decision: "pass",
            requester_ref: input.requester_ref,
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            bound_refs: &[fake_ref(input.label)],
            retained_refs: &[],
            remote_refs: input.remote_refs,
            is_reference_index_complete: input.is_reference_index_complete,
            is_current: input.is_current,
            revoked_refs: input.revoked_refs,
            diagnostics: &[],
        })
        .expect("store test admission")
        .admission_ref
    }

    struct TestPlanFixture {
        requester_ref: String,
        object_ref: String,
        evidence: DestructiveEvidence,
    }

    struct Flow {
        plan: GcPlan,
        apply: GcApply,
        execution: GcExecutionGate,
        audit: GcAudit,
    }

    fn passing_flow(root: &Path, fixture: &TestPlanFixture, subsystem: &str) -> Flow {
        let plan = store_gc_plan(GcPlanInput {
            root,
            subsystem,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store plan");
        let apply = apply_gc_plan(GcApplyFromPlanInput {
            root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply plan");
        let execution = store_gc_execution_gate(GcExecutionGateInput {
            root,
            subsystem,
            action: ACTION_DELETE,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            apply_ref: Some(&apply.apply_ref),
        })
        .expect("store execution");
        let audit = audit_gc_execution(GcAuditInput {
            root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit execution");
        Flow {
            plan,
            apply,
            execution,
            audit,
        }
    }

    struct SeedInput<'a> {
        root: &'a Path,
        kind: &'a str,
        label: String,
        requester_ref: &'a str,
        object_ref: &'a str,
        remote_refs: &'a [String],
    }

    fn seed_ref(input: SeedInput<'_>) -> String {
        store_test_admission(TestAdmissionInput {
            root: input.root,
            kind: input.kind,
            label: &input.label,
            requester_ref: input.requester_ref,
            object_ref: input.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_refs: input.remote_refs,
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        })
    }

    fn seed_set(
        root: &Path,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        remote_refs: &[String],
    ) -> [String; 5] {
        let empty_refs: &[String] = &[];
        [
            (ADMISSION_KIND_POLICY, "policy", empty_refs),
            (ADMISSION_KIND_AUTHORITY, "authority", empty_refs),
            (ADMISSION_KIND_SUPPORTING_EVIDENCE, "support", empty_refs),
            (ADMISSION_KIND_REFERENCE_INDEX, "index", empty_refs),
            (ADMISSION_KIND_REMOTE_GC, "remote-gc", remote_refs),
        ]
        .map(|(kind, suffix, remote_refs)| {
            seed_ref(SeedInput {
                root,
                kind,
                label: format!("{label}-{suffix}"),
                requester_ref,
                object_ref,
                remote_refs,
            })
        })
    }

    struct DenyCase {
        requester: String,
        object: String,
        remotes: Vec<String>,
        peers: Vec<String>,
        wrong_peer: String,
        policy: String,
        authority: String,
        support: String,
        index: String,
        gc: String,
    }

    impl DenyCase {
        fn base(&self) -> DestructiveEvidence {
            DestructiveEvidence {
                requester_ref: Some(self.requester.clone()),
                policy_refs: vec![self.policy.clone()],
                authority_refs: vec![self.authority.clone()],
                evidence_refs: vec![self.support.clone()],
                retained_refs: Vec::new(),
                remote_peer_refs: self.peers.clone(),
                remote_refs: self.remotes.clone(),
                reference_index_refs: vec![self.index.clone()],
                remote_gc_refs: vec![self.gc.clone()],
                remote_clearance_refs: Vec::new(),
                is_reference_index_complete: true,
            }
        }

        fn scoped(&self, stored_ref: String) -> DestructiveEvidence {
            let mut evidence = self.base();
            evidence.remote_refs = vec![self.remotes[0].clone()];
            evidence.remote_peer_refs = vec![self.peers[0].clone()];
            evidence.remote_clearance_refs = vec![stored_ref];
            evidence
        }
    }

    struct DenyRefs {
        partial: String,
        wrong_peer: String,
        stale: String,
        retained: String,
    }

    struct ClearInput<'a> {
        root: &'a Path,
        case: &'a DenyCase,
        label: &'a str,
        peer: &'a str,
        remote: &'a str,
        is_current: bool,
        revoked_refs: &'a [String],
        retained_refs: &'a [String],
    }
