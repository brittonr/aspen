
    fn replay_rollup(verify: &IoValue) -> crate::deterministic_replay::ReplayRollupReceipt {
        crate::deterministic_replay::rollup_replay_receipts(&[crate::deterministic_replay::ReplayRollupInput {
            expected_ref: Some(canonical_hash(verify).expect("verify ref")),
            value: verify.clone(),
        }])
        .expect("replay rollup")
    }

    fn replay_index(
        verify: &IoValue,
        rollup: &crate::deterministic_replay::ReplayRollupReceipt,
    ) -> crate::deterministic_replay::ReplayIndexReceipt {
        crate::deterministic_replay::index_replay_evidence(&[
            crate::deterministic_replay::ReplayIndexInput {
                expected_ref: Some(canonical_hash(verify).expect("verify ref")),
                value: verify.clone(),
            },
            crate::deterministic_replay::ReplayIndexInput {
                expected_ref: Some(rollup.rollup_ref.clone()),
                value: rollup.value.clone(),
            },
        ])
        .expect("replay index")
    }

    fn assert_replay_verify_search(registry: &Path, ledger_root: &Path, fixture: &ReplayCase) {
        let request = mcp_request_value("search_replay_evidence", vec![
            record("stage", vec![string("verify")]),
            record("decision", vec![string("deny")]),
            record("final-state-ref", vec![string(&fixture.final_state_ref)]),
        ])
        .expect("replay verify request");
        let call = call(registry, Some(ledger_root), &request).expect("replay verify call");
        assert_eq!(call.decision, "pass");
        let text = to_text(&call.response_value).expect("replay verify response");
        assert!(text.contains("deterministic-replay:verify"));
        assert!(text.contains(&fixture.expected_report_ref));
    }

    fn assert_replay_divergence_search(registry: &Path, ledger_root: &Path, fixture: &ReplayCase) {
        let request = mcp_request_value("search_replay_evidence", vec![
            record("stage", vec![string("first-divergence")]),
            record("divergence", vec![string("effect-response")]),
            record("handler-profile-ref", vec![string(&fixture.handler_profile_ref)]),
            record("actual-ref", vec![string(&fixture.actual_ref)]),
        ])
        .expect("replay divergence request");
        let call = call(registry, Some(ledger_root), &request).expect("replay divergence call");
        assert_eq!(call.decision, "pass");
        let text = to_text(&call.response_value).expect("replay divergence response");
        assert!(text.contains("deterministic-replay:first-divergence"));
        assert!(text.contains("actor:helper"));
        assert!(to_text(&call.receipt_value).expect("replay MCP receipt").contains("mutating-tools-denied"));
    }

    fn assert_replay_rollup_search(registry: &Path, ledger_root: &Path) {
        let request = mcp_request_value("search_replay_evidence", vec![
            record("stage", vec![string("rollup")]),
            record("text", vec![string("replay-rollup-decision:deny")]),
        ])
        .expect("replay rollup request");
        let call = call(registry, Some(ledger_root), &request).expect("replay rollup call");
        assert_eq!(call.decision, "pass");
        assert!(
            to_text(&call.response_value)
                .expect("replay rollup response")
                .contains("deterministic-replay:rollup")
        );
    }

    fn assert_replay_index_search(registry: &Path, ledger_root: &Path) {
        let request = mcp_request_value("search_replay_evidence", vec![
            record("stage", vec![string("index")]),
            record("text", vec![string("replay-index-decision:deny")]),
        ])
        .expect("replay index request");
        let call = call(registry, Some(ledger_root), &request).expect("replay index call");
        assert_eq!(call.decision, "pass");
        assert!(to_text(&call.response_value).expect("replay index response").contains("deterministic-replay:index"));
    }

    struct RetentionGcAuditFixture {
        object_ref: String,
        execution_ref: String,
        audit: crate::retention::GcAudit,
    }

    fn gc_audit_fixture(root: &Path, label: &str, subsystem: &str) -> RetentionGcAuditFixture {
        let seed = make_seed(root, label, subsystem);
        let evidence = seed_evidence(&seed);
        let plan = crate::retention::store_gc_plan(crate::retention::GcPlanInput {
            root: seed.root,
            subsystem: seed.subsystem,
            object_ref: &seed.object_ref,
            object_kind: seed.object_kind,
            retention_class: seed.retention_class,
            action: seed.action,
            evidence: &evidence,
        })
        .expect("store retention GC catalog MCP plan");
        let apply = crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
            root: seed.root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply retention GC catalog MCP plan");
        let execution = crate::retention::store_gc_execution_gate(crate::retention::GcExecutionGateInput {
            root: seed.root,
            subsystem: seed.subsystem,
            action: seed.action,
            object_ref: &seed.object_ref,
            object_kind: seed.object_kind,
            retention_class: seed.retention_class,
            apply_ref: Some(&apply.apply_ref),
        })
        .expect("store retention GC catalog MCP execution");
        let audit = crate::retention::audit_gc_execution(crate::retention::GcAuditInput {
            root: seed.root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit retention GC catalog MCP execution");
        RetentionGcAuditFixture {
            object_ref: seed.object_ref,
            execution_ref: execution.execution_ref,
            audit,
        }
    }

    struct GcSeed<'a> {
        root: &'a Path,
        label: &'a str,
        subsystem: &'a str,
        requester_ref: String,
        object_ref: String,
        object_kind: &'static str,
        retention_class: &'static str,
        action: &'static str,
    }

    fn make_seed<'a>(root: &'a Path, label: &'a str, subsystem: &'a str) -> GcSeed<'a> {
        GcSeed {
            root,
            label,
            subsystem,
            requester_ref: test_ref(&format!("{label}-requester")),
            object_ref: test_ref(&format!("{label}-object")),
            object_kind: "chunk",
            retention_class: crate::retention::CLASS_DURABLE_VALUE,
            action: crate::retention::ACTION_DELETE,
        }
    }

    fn seed_evidence(seed: &GcSeed<'_>) -> crate::retention::DestructiveEvidence {
        crate::retention::DestructiveEvidence {
            requester_ref: Some(seed.requester_ref.clone()),
            policy_refs: vec![seed_admission(seed, crate::retention::ADMISSION_KIND_POLICY, "policy")],
            authority_refs: vec![seed_admission(
                seed,
                crate::retention::ADMISSION_KIND_AUTHORITY,
                "authority",
            )],
            evidence_refs: vec![seed_admission(
                seed,
                crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
                "support",
            )],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: vec![seed_admission(
                seed,
                crate::retention::ADMISSION_KIND_REFERENCE_INDEX,
                "index",
            )],
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        }
    }

    fn seed_admission(seed: &GcSeed<'_>, kind: &str, suffix: &str) -> String {
        crate::retention::store_evidence_admission(seed.root, &crate::retention::EvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref: &seed.requester_ref,
            object_ref: &seed.object_ref,
            object_kind: seed.object_kind,
            retention_class: seed.retention_class,
            action: seed.action,
            bound_refs: &[test_ref(&format!("{}-{suffix}", seed.label))],
            retained_refs: &[],
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("store retention GC catalog MCP admission")
        .admission_ref
    }

    fn install_fixture(
        root: &Path,
        kind: &str,
        payload: IoValue,
        dependency_refs: &[String],
        schema_refs: &[String],
    ) -> ArtifactInstall {
        crate::artifacts::install_artifact(root, &crate::artifacts::ArtifactInstallInput {
            kind: kind.to_string(),
            payload,
            schema_refs: schema_refs.to_vec(),
            dependency_refs: dependency_refs.to_vec(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install fixture")
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("catalog-mcp-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
