
fn synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("service-runtime-fixture-ref", vec![string(label)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("service-runtime-test-ref", vec![string(label)])).expect("test ref")
    }

    fn evidence() -> EvidenceInput {
        EvidenceInput {
            authority_refs: vec![test_ref("authority")],
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
            effect_profile_refs: vec![test_ref("effect")],
            source_gate_refs: vec![test_ref("source-gate")],
            scheduler_ref: Some(test_ref("scheduler")),
            effect_log_refs: vec![test_ref("effect-log")],
        }
    }

    fn manifest(service_id: &str, dependencies: Vec<String>) -> preserves::IOValue {
        let evidence = evidence();
        crate::service_records::service_manifest_value(&crate::service_records::ServiceManifestInput {
            service_id: service_id.to_string(),
            owner_authority_ref: evidence.authority_refs[0].clone(),
            target_ref: test_ref(&format!("target-{service_id}")),
            dependencies,
            provided_assertion_refs: vec![test_ref(&format!("provided-{service_id}"))],
            restart_policy_ref: test_ref(&format!("restart-{service_id}")),
            policy_refs: evidence.policy_refs,
            resource_refs: evidence.resource_refs,
            effect_profile_refs: evidence.effect_profile_refs,
        })
        .expect("service manifest")
    }

    fn demand(service_id: &str, manifest_value: &preserves::IOValue) -> preserves::IOValue {
        crate::service_records::service_demand_value(&crate::service_records::ServiceDemandInput {
            demand_id: format!("demand:{service_id}"),
            service_id: service_id.to_string(),
            requester_ref: test_ref("requester"),
            manifest_ref: Some(canonical_hash(manifest_value).expect("manifest ref")),
            policy_refs: vec![test_ref("policy")],
        })
        .expect("service demand")
    }

    #[test]
    fn rejects_malformed_content_refs() {
        for invalid in [
            "blake3:fixture",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
        ] {
            let mut evidence = evidence();
            evidence.authority_refs = vec![invalid.to_string()];
            let error = suite_value(&SuiteInput {
                manifests: vec![manifest("svc:bad-ref", Vec::new())],
                demands: Vec::new(),
                statuses: Vec::new(),
                evidence,
            })
            .expect_err("malformed service runtime ref denied");
            assert!(error.to_string().contains("canonical blake3 content ref"), "unexpected error: {error}");
        }
    }

    #[test]
    // r[verify molten.sam_service_supervision.spec.demand_start]
    // r[verify molten.sam_service_demand_runtime.spec.admitted_demand_start]
    fn two_service_demand_starts_dependency_then_frontend() {
        let suite_value = two_service_suite_value().expect("two service suite");
        let run = run_suite_value(&suite_value).expect("run services");
        assert_eq!(run.readiness_assertions.len(), 2);
        let receipts = run
            .lifecycle_receipts
            .iter()
            .map(crate::service_records::parse_service_lifecycle_receipt)
            .collect::<Result<Vec<_>>>()
            .expect("parse receipts");
        assert_eq!(receipts.iter().filter(|receipt| receipt.decision == "pass").count(), 2);
        replay_report(&run.value).expect("replay service runtime report");
    }

    #[test]
    fn missing_authority_denies_before_side_effects() {
        let run = run_with_missing_evidence(|evidence| evidence.authority_refs.clear());
        assert!(run.readiness_assertions.is_empty());
        let receipt =
            crate::service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("deny receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority")));
    }

    #[test]
    fn missing_source_gate_denies_before_side_effects() {
        let run = run_with_missing_evidence(|evidence| evidence.source_gate_refs.clear());
        assert!(run.readiness_assertions.is_empty());
        let receipt =
            crate::service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("deny receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("source-gate")));
    }

    fn run_with_missing_evidence(update: impl FnOnce(&mut EvidenceInput)) -> Run {
        let backend = manifest("svc:backend", Vec::new());
        let demand = demand("svc:backend", &backend);
        let mut evidence = evidence();
        update(&mut evidence);
        let suite_value = record("service-runtime-suite-v1", vec![
            string(RUNTIME_SUITE_SCHEMA),
            record("manifests", vec![sequence(vec![backend])]),
            record("demands", vec![sequence(vec![demand])]),
            record("statuses", vec![sequence(Vec::new())]),
            evidence_value(&evidence),
            checks_value(&["canonical-service-runtime-suite", "explicit-startup-evidence"]),
        ]);
        run_suite_value(&suite_value).expect("missing evidence emits deny report")
    }

    #[test]
    fn unmet_dependency_waits_without_readiness() {
        let frontend = manifest("svc:frontend", vec!["svc:backend".to_string()]);
        let demand = demand("svc:frontend", &frontend);
        let suite = suite_value(&SuiteInput {
            manifests: vec![frontend],
            demands: vec![demand],
            statuses: Vec::new(),
            evidence: evidence(),
        })
        .expect("suite");
        let run = run_suite_value(&suite).expect("run services");
        assert!(run.readiness_assertions.is_empty());
        let receipt =
            crate::service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("receipt");
        assert_eq!(receipt.operation, "dependency-wait");
        assert_eq!(receipt.decision, "diagnostic");
    }

    #[test]
    // r[verify molten.sam_service_demand_runtime.spec.dependency_resolution]
    fn dependency_cycle_denies_without_readiness() {
        let frontend = manifest("svc:frontend", vec!["svc:backend".to_string()]);
        let backend = manifest("svc:backend", vec!["svc:frontend".to_string()]);
        let frontend_demand = demand("svc:frontend", &frontend);
        let backend_demand = demand("svc:backend", &backend);
        let suite = suite_value(&SuiteInput {
            manifests: vec![frontend, backend],
            demands: vec![frontend_demand, backend_demand],
            statuses: Vec::new(),
            evidence: evidence(),
        })
        .expect("suite");
        let run = run_suite_value(&suite).expect("run services");
        assert!(run.readiness_assertions.is_empty());
        let receipts = run
            .lifecycle_receipts
            .iter()
            .map(crate::service_records::parse_service_lifecycle_receipt)
            .collect::<Result<Vec<_>>>()
            .expect("receipts");
        assert_eq!(receipts.len(), 2);
        assert!(receipts.iter().all(|receipt| receipt.decision == "deny"));
        assert!(
            receipts
                .iter()
                .all(|receipt| receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("cycle")))
        );
    }

    #[test]
    fn malformed_manifest_denies_before_execution() {
        let malformed = crate::preserves_rail::parse_text(
            "<service-manifest-v1 \"molten.service.manifest.v1\" <service-id \"svc:x\">>",
        )
        .expect("malformed manifest shape parses");
        let suite = SuiteInput {
            manifests: vec![malformed],
            demands: Vec::new(),
            statuses: Vec::new(),
            evidence: evidence(),
        };
        assert!(suite_value(&suite).is_err());
    }

    #[test]
    // r[verify molten.sam_service_demand_runtime.spec.owned_assertion_replay]
    fn replay_detects_changed_dependency_identity() {
        let suite_value = two_service_suite_value().expect("two service suite");
        let run = run_suite_value(&suite_value).expect("run services");
        let mut report = parse_report(&run.value).expect("parse report");
        report.statuses.pop();
        let tampered = report_value(ReportValueInput {
            suite_value: &suite_value,
            lifecycle_receipts: &report.lifecycle_receipts,
            statuses: &report.statuses,
            readiness_assertions: &report.readiness_assertions,
            replay_identities: &report.replay_identities,
            turn_contexts: &report.turn_contexts,
        })
        .expect("tampered report");
        assert!(replay_report(&tampered).is_err());
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_demand_identity_replay_and_no_side_effects_on_wait(tc: hegel::TestCase) {
        let dependency_count = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(3));
        let dependency_count_usize = usize::try_from(dependency_count).expect("bounded dependency count");
        let dependencies = (0..dependency_count_usize).map(|index| format!("svc:dep-{index}")).collect::<Vec<_>>();
        let service = manifest("svc:generated", dependencies);
        let demand = demand("svc:generated", &service);
        let suite = suite_value(&SuiteInput {
            manifests: vec![service],
            demands: vec![demand],
            statuses: Vec::new(),
            evidence: evidence(),
        })
        .expect("generated suite");
        let run = run_suite_value(&suite).expect("generated run");
        let replay = replay_report(&run.value).expect("generated replay");
        assert_eq!(replay.decision, "pass");
        if dependency_count_usize == 0 {
            assert_eq!(run.readiness_assertions.len(), 1);
        } else {
            assert!(run.readiness_assertions.is_empty());
            let receipt =
                crate::service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("receipt");
            assert_eq!(receipt.operation, "dependency-wait");
        }
    }
}
