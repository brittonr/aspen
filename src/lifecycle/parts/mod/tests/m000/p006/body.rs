    #[derive(Debug, Default, PartialEq, Eq)]
    struct AssertionKeys {
        started: Vec<String>,
        ready: Vec<String>,
        failed: Vec<String>,
        complete: Vec<String>,
    }

    impl AssertionKeys {
        fn new(bound: usize) -> Self {
            Self {
                started: Vec::with_capacity(bound),
                ready: Vec::with_capacity(bound),
                failed: Vec::with_capacity(bound),
                complete: Vec::with_capacity(bound),
            }
        }
    }

    fn projected_keys(rows: &[super::ServiceStateMappingRow]) -> AssertionKeys {
        let mut keys = AssertionKeys::new(rows.len());
        for row in rows {
            let key = format!("{}:{}", row.vocabulary.as_str(), row.local_value);
            let projected = project_row(row, row.is_normal_finish());
            if projected.contains(&super::ServiceStateAssertion::Started) {
                keys.started.push(key.clone());
            }
            if projected.contains_ready() {
                keys.ready.push(key.clone());
            }
            if projected.values().contains(&super::ServiceStateAssertion::Failed) {
                keys.failed.push(key.clone());
            }
            if projected.contains_complete() {
                keys.complete.push(key);
            }
        }
        keys
    }

    fn flagged_keys(rows: &[super::ServiceStateMappingRow]) -> AssertionKeys {
        let mut keys = AssertionKeys::new(rows.len());
        for row in rows {
            let key = format!("{}:{}", row.vocabulary.as_str(), row.local_value);
            if row.is_started() {
                keys.started.push(key.clone());
            }
            if row.is_ready() {
                keys.ready.push(key.clone());
            }
            if row.is_failed() {
                keys.failed.push(key.clone());
            }
            if row.is_normal_finish() {
                keys.complete.push(key);
            }
        }
        keys
    }

    #[test]
    fn mapping_table_matches_every_local_value() {
        // r[verify molten.runtime_spine.service_state_union]
        for row in super::service_state_mapping() {
            let key = format!("{}:{}", row.vocabulary.as_str(), row.local_value);
            let projected = project_row(row, row.is_normal_finish());
            assert_eq!(
                projected.contains(&super::ServiceStateAssertion::Started),
                row.is_started(),
                "started mismatch for {key}"
            );
            assert_eq!(
                projected.contains_ready(),
                row.is_ready(),
                "ready mismatch for {key}"
            );
            assert_eq!(
                projected.values().contains(&super::ServiceStateAssertion::Failed),
                row.is_failed(),
                "failed mismatch for {key}"
            );
            assert_eq!(
                projected.contains_complete(),
                row.is_normal_finish(),
                "complete mismatch for {key}"
            );
        }
    }

    #[test]
    fn mapping_table_reverse_rows_match_the_forward_projection() {
        let rows = super::service_state_mapping();
        let projected = projected_keys(rows);
        let flagged = flagged_keys(rows);
        assert_eq!(projected.started, flagged.started);
        assert_eq!(projected.ready, flagged.ready);
        assert_eq!(projected.failed, flagged.failed);
        assert_eq!(projected.complete, flagged.complete);
        assert_eq!(
            projected.ready,
            vec![
                "lifecycle:ready".to_string(),
                "extension:running".to_string(),
                "extension:checkpointing".to_string(),
                "readiness:ready".to_string(),
            ]
        );
        assert_eq!(
            projected.failed,
            vec![
                "lifecycle:failed".to_string(),
                "extension:failed".to_string(),
                "extension:quarantined".to_string(),
                "readiness:failed".to_string(),
            ]
        );
        assert_eq!(
            projected.complete,
            vec!["lifecycle:stopped".to_string(), "extension:stopped".to_string()]
        );
    }

    #[test]
    fn service_demand_waits_without_start_until_dependencies_are_ready() {
        let demand_ref = content_ref_from_bytes(b"service-demand");
        let manifest_ref = content_ref_from_bytes(b"service-manifest");
        let dependency_ref = content_ref_from_bytes(b"database-ready");
        let authority_ref = content_ref_from_bytes(b"service-authority");
        let resource_ref = content_ref_from_bytes(b"service-resource");
        let evidence_ref = content_ref_from_bytes(b"service-evidence");
        let pending = super::ServiceAssertionSet::from_values(Vec::new()).expect("pending assertions");
        let wait = super::evaluate_service_demand(&super::ServiceDemandEvaluationInput {
            service_id: "service:frontend",
            demand_ref: &demand_ref,
            manifest_ref: &manifest_ref,
            dependencies: &[super::ServiceDependencyInput {
                dependency_ref: &dependency_ref,
                requirement: super::ServiceRequirement::Ready,
                assertions: &pending,
            }],
            authority_refs: std::slice::from_ref(&authority_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("dependency wait");
        assert_eq!(wait.decision, "wait");
        assert_eq!(wait.lifecycle_kind, "dependency-wait");
        assert!(!wait.start_side_effect_admitted);
        assert!(wait.readiness_assertion.is_none());

        let ready_assertions = super::ServiceAssertionSet::from_values(vec![
            super::ServiceStateAssertion::Started,
            super::ServiceStateAssertion::Ready,
        ])
        .expect("ready assertions");
        let ready = super::evaluate_service_demand(&super::ServiceDemandEvaluationInput {
            service_id: "service:frontend",
            demand_ref: &demand_ref,
            manifest_ref: &manifest_ref,
            dependencies: &[super::ServiceDependencyInput {
                dependency_ref: &dependency_ref,
                requirement: super::ServiceRequirement::Ready,
                assertions: &ready_assertions,
            }],
            authority_refs: std::slice::from_ref(&authority_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("service ready demand");
        assert_eq!(ready.decision, "pass");
        assert_eq!(ready.lifecycle_kind, "start");
        assert!(ready.start_side_effect_admitted);
        assert!(ready.readiness_assertion.is_some());
    }

    #[test]
    fn dependent_waits_for_up_until_the_dependency_completes() {
        let demand_ref = content_ref_from_bytes(b"service-demand-up");
        let manifest_ref = content_ref_from_bytes(b"service-manifest-up");
        let dependency_ref = content_ref_from_bytes(b"migration-ready");
        let authority_ref = content_ref_from_bytes(b"service-authority-up");
        let resource_ref = content_ref_from_bytes(b"service-resource-up");
        let evidence_ref = content_ref_from_bytes(b"service-evidence-up");
        let running = assertions(vec![super::ServiceStateAssertion::Started]);
        let ready = assertions(vec![
            super::ServiceStateAssertion::Started,
            super::ServiceStateAssertion::Ready,
        ]);
        let complete = assertions(vec![
            super::ServiceStateAssertion::Started,
            super::ServiceStateAssertion::Complete,
        ]);

        let waiting = super::evaluate_service_demand(&super::ServiceDemandEvaluationInput {
            service_id: "service:frontend",
            demand_ref: &demand_ref,
            manifest_ref: &manifest_ref,
            dependencies: &[super::ServiceDependencyInput {
                dependency_ref: &dependency_ref,
                requirement: super::ServiceRequirement::Up,
                assertions: &running,
            }],
            authority_refs: std::slice::from_ref(&authority_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("up dependency wait");
        assert_eq!(waiting.decision, "wait");
        assert_eq!(waiting.lifecycle_kind, "dependency-wait");

        for satisfied in [&ready, &complete] {
            let admitted = super::evaluate_service_demand(&super::ServiceDemandEvaluationInput {
                service_id: "service:frontend",
                demand_ref: &demand_ref,
                manifest_ref: &manifest_ref,
                dependencies: &[super::ServiceDependencyInput {
                    dependency_ref: &dependency_ref,
                    requirement: super::ServiceRequirement::Up,
                    assertions: satisfied,
                }],
                authority_refs: std::slice::from_ref(&authority_ref),
                resource_refs: std::slice::from_ref(&resource_ref),
                evidence_refs: std::slice::from_ref(&evidence_ref),
            })
            .expect("up dependency admitted");
            assert_eq!(admitted.decision, "pass");
            assert!(admitted.start_side_effect_admitted);
        }
    }
