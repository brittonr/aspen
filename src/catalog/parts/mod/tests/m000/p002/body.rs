
    #[test]
    fn view_renders_octet_baseline_quarantine_metadata() {
        let dir = temp_dir("catalog-octet-baseline");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        std::fs::create_dir_all(&registry).expect("create registry");
        let review_ref = test_ref("octet-review");
        let baseline = record("octet-warning-baseline-v1", vec![
            string(crate::preserves_rail::OCTET_WARNING_BASELINE_SCHEMA),
            record("scope", vec![string("workspace")]),
            record("created-at", vec![string("2026-05-31T00:00:00Z")]),
            record("expires-at", vec![string("2026-06-30T00:00:00Z")]),
            record("octet-config-hash", vec![string("b3:config")]),
            record("octet-profile-hash", vec![string("b3:profile")]),
            record("toolchain", vec![string("rustc-test")]),
            record("source-snapshot", vec![string(test_ref("source-snapshot"))]),
            record("finding-keys", vec![sequence(vec![
                record("finding-key", vec![
                    string("b3:finding-a"),
                    string("no_unwrap"),
                    string("molten"),
                    string("src/main.rs:1"),
                    count_value(1),
                ]),
                record("finding-key", vec![
                    string("b3:finding-b"),
                    string("bool_naming"),
                    string("molten"),
                    string("src/lib.rs:1"),
                    count_value(1),
                ]),
            ])]),
            record("critical-finding-keys", vec![sequence(vec![string("b3:finding-a")])]),
            record("allowed-profiles", vec![sequence(vec![string("quarantine-ci")])]),
            record("burn-down", vec![
                record("total", vec![count_value(2)]),
                record("target-next", vec![count_value(1)]),
                record("deadline", vec![string("2026-06-30T00:00:00Z")]),
            ]),
            record("review-refs", vec![sequence(vec![string(&review_ref)])]),
            checks_value(&["baseline-findings-keyed"]),
        ]);
        let imported = crate::ledger::import_artifact(&ledger_root, &baseline).expect("import baseline");
        let viewed = view(&registry, Some(&ledger_root), &ViewInput {
            reference: imported.artifact_ref,
            include_payload: true,
            redacted: true,
            visibility: VisibilityInput::default(),
        })
        .expect("view octet baseline");
        let text = to_text(&viewed.value).expect("render catalog view");

        assert!(text.contains("octet-baseline:warning-quarantine"));
        assert!(text.contains("octet-baseline-findings:2"));
        assert!(text.contains("octet-baseline-critical:1"));
        assert!(text.contains("octet-baseline-expires-at:2026-06-30T00:00:00Z"));
        assert!(text.contains("octet-baseline-burn-down-target-next:1"));
        assert!(text.contains(&format!("octet-review-ref:{review_ref}")));
    }

    #[hegel::test(test_cases = 12)]
    fn hegel_catalog_identity_short_ids_and_visibility_are_stable(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let dir = temp_dir("catalog-hegel");
        let registry = dir.join("registry");
        let payload = record("doc", vec![string(format!("payload-{salt}"))]);
        let installed = install_fixture(&registry, "doc", payload, &[], &[]);
        let first = list(&registry, None, &ListInput {
            kind: Some("doc".to_string()),
            visibility: VisibilityInput::default(),
        })
        .expect("first list");
        let display_name = format!("display-{salt}");
        crate::artifacts::set_name_pointer(&registry, &crate::artifacts::SetNamePointerInput {
            pointer_kind: "name",
            name: &display_name,
            artifact_ref: &installed.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("set display name");
        let second = list(&registry, None, &ListInput {
            kind: Some("doc".to_string()),
            visibility: VisibilityInput::default(),
        })
        .expect("second list");
        assert_eq!(first.items.len(), second.items.len());
        assert!(first.items[0].collect_simple_record("catalog-summary-v1", None).is_some());
        let resolved = resolve_short_id(&registry, None, &ShortIdInput {
            prefix: installed.artifact_ref[7..19].to_string(),
            min_length: 8,
            visibility: VisibilityInput::default(),
        })
        .expect("resolve stable short id");
        assert_eq!(resolved.full_ref, Some(installed.artifact_ref.clone()));
        let hidden = resolve_short_id(&registry, None, &ShortIdInput {
            prefix: installed.artifact_ref[7..19].to_string(),
            min_length: 8,
            visibility: VisibilityInput {
                hidden_refs: vec![installed.artifact_ref],
                ..VisibilityInput::default()
            },
        })
        .expect("hidden short id");
        assert_eq!(hidden.decision, "deny");
    }

    type GcEvidence = crate::retention::DestructiveEvidence;
    type GcPlan = crate::retention::GcPlan;
    type GcApply = crate::retention::GcApply;
    type GcExecution = crate::retention::GcExecutionGate;
    type GcAudit = crate::retention::GcAudit;

    struct GcCase {
        object_ref: String,
        plan: GcPlan,
        apply: GcApply,
        execution: GcExecution,
        audit: GcAudit,
    }

    struct Seed<'a> {
        root: &'a Path,
        label: &'a str,
        subsystem: &'a str,
        requester_ref: String,
        object_ref: String,
        object_kind: &'static str,
        retention_class: &'static str,
        action: &'static str,
    }

    impl<'a> Seed<'a> {
        fn new(root: &'a Path, label: &'a str, subsystem: &'a str) -> Self {
            Self {
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

        fn admission(&self, kind: &str, suffix: &str) -> String {
            crate::retention::store_evidence_admission(self.root, &crate::retention::EvidenceAdmissionInput {
                kind,
                decision: "pass",
                requester_ref: &self.requester_ref,
                object_ref: &self.object_ref,
                object_kind: self.object_kind,
                retention_class: self.retention_class,
                action: self.action,
                bound_refs: &[test_ref(&format!("{}-{suffix}", self.label))],
                retained_refs: &[],
                remote_refs: &[],
                is_reference_index_complete: true,
                is_current: true,
                revoked_refs: &[],
                diagnostics: &[],
            })
            .expect("store retention GC catalog admission")
            .admission_ref
        }

        fn evidence(&self) -> GcEvidence {
            GcEvidence {
                requester_ref: Some(self.requester_ref.clone()),
                policy_refs: vec![self.admission(crate::retention::ADMISSION_KIND_POLICY, "policy")],
                authority_refs: vec![self.admission(crate::retention::ADMISSION_KIND_AUTHORITY, "authority")],
                evidence_refs: vec![self.admission(crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE, "support")],
                retained_refs: Vec::new(),
                remote_peer_refs: Vec::new(),
                remote_refs: Vec::new(),
                reference_index_refs: vec![self.admission(crate::retention::ADMISSION_KIND_REFERENCE_INDEX, "index")],
                remote_gc_refs: Vec::new(),
                remote_clearance_refs: Vec::new(),
                is_reference_index_complete: true,
            }
        }

        fn plan(&self, evidence: &GcEvidence) -> GcPlan {
            crate::retention::store_gc_plan(crate::retention::GcPlanInput {
                root: self.root,
                subsystem: self.subsystem,
                object_ref: &self.object_ref,
                object_kind: self.object_kind,
                retention_class: self.retention_class,
                action: self.action,
                evidence,
            })
            .expect("store retention GC catalog plan")
        }

        fn apply(&self, plan_ref: &str) -> GcApply {
            crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
                root: self.root,
                plan_ref,
            })
            .expect("apply retention GC catalog plan")
        }

        fn execution(&self, apply_ref: &str) -> GcExecution {
            crate::retention::store_gc_execution_gate(crate::retention::GcExecutionGateInput {
                root: self.root,
                subsystem: self.subsystem,
                action: self.action,
                object_ref: &self.object_ref,
                object_kind: self.object_kind,
                retention_class: self.retention_class,
                apply_ref: Some(apply_ref),
            })
            .expect("store retention GC catalog execution")
        }

        fn audit(&self, execution_ref: &str) -> GcAudit {
            crate::retention::audit_gc_execution(crate::retention::GcAuditInput {
                root: self.root,
                execution_ref,
            })
            .expect("audit retention GC catalog execution")
        }

        fn finish(self) -> GcCase {
            let evidence = self.evidence();
            let plan = self.plan(&evidence);
            let apply = self.apply(&plan.plan_ref);
            let execution = self.execution(&apply.apply_ref);
            let audit = self.audit(&execution.execution_ref);
            GcCase {
                object_ref: self.object_ref,
                plan,
                apply,
                execution,
                audit,
            }
        }
    }

    fn gc_case(root: &Path, label: &str, subsystem: &str) -> GcCase {
        Seed::new(root, label, subsystem).finish()
    }

    fn install_fixture(
        root: &Path,
        kind: &str,
        payload: IoValue,
        dependency_refs: &[String],
        schema_refs: &[String],
    ) -> crate::artifacts::ArtifactInstall {
        crate::artifacts::install_artifact(root, &crate::artifacts::ArtifactInstallInput {
            kind: kind.to_string(),
            payload,
            schema_refs: schema_refs.to_vec(),
            dependency_refs: dependency_refs.to_vec(),
            effect_manifest_ref: Some(test_ref("effect")),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install fixture")
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("catalog-test-ref", vec![string(label)])).expect("test ref")
    }

    fn count_value(value: u64) -> IoValue {
        crate::preserves_rail::u64_value(value)
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
