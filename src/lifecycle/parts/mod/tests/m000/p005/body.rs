    fn assertions(values: Vec<super::ServiceStateAssertion>) -> super::ServiceAssertionSet {
        super::ServiceAssertionSet::from_values(values).expect("service assertions")
    }

    fn project(
        lifecycle_state: Option<super::State>,
        extension_phase: Option<&str>,
        readiness: Option<super::ServiceReadinessOutcome>,
        completed_normally: bool,
        user_defined: &[String],
    ) -> super::ServiceAssertionSet {
        super::project_service_assertions(&super::ServiceAssertionInput {
            lifecycle_state,
            extension_phase,
            readiness,
            completed_normally,
            user_defined,
        })
        .expect("projected service assertions")
    }

    fn readiness_outcome(local_value: &str) -> super::ServiceReadinessOutcome {
        match local_value {
            "ready" => super::ServiceReadinessOutcome::Ready,
            "failed" => super::ServiceReadinessOutcome::Failed,
            other => panic!("unexpected readiness row {other}"),
        }
    }

    fn project_row(
        row: &super::ServiceStateMappingRow,
        completed_normally: bool,
    ) -> super::ServiceAssertionSet {
        let (lifecycle_state, extension_phase, readiness) = match row.vocabulary {
            super::ServiceStateVocabulary::Lifecycle => {
                (super::State::from_as_str(row.local_value), None, None)
            }
            super::ServiceStateVocabulary::Extension => (None, Some(row.local_value), None),
            super::ServiceStateVocabulary::Readiness => (None, None, Some(readiness_outcome(row.local_value))),
        };
        project(lifecycle_state, extension_phase, readiness, completed_normally, &[])
    }

    #[test]
    fn one_shot_completion_projects_complete_and_is_up() {
        // r[verify molten.runtime_spine.service_state_union]
        let complete = project(Some(super::State::Stopped), Some("stopped"), None, true, &[]);
        assert!(complete.contains_complete());
        assert!(complete.is_up());
        assert!(!complete.contains_ready());
    }

    #[test]
    fn intentional_stop_is_not_complete() {
        let stopped = project(Some(super::State::Stopped), None, None, false, &[]);
        assert!(!stopped.contains_complete());
        assert!(!stopped.is_up());
    }

    #[test]
    fn ready_service_holds_started_and_ready_together() {
        let ready = project(Some(super::State::Ready), None, None, false, &[]);
        assert!(ready.contains_ready());
        assert!(ready.contains(&super::ServiceStateAssertion::Started));
        assert!(ready.is_up());
    }

    #[test]
    fn extension_and_readiness_vocabularies_map_onto_the_shared_set() {
        let extension_ready = project(None, Some("running"), None, false, &[]);
        assert!(extension_ready.contains_ready());
        assert!(extension_ready.contains(&super::ServiceStateAssertion::Started));

        let readiness_failed = project(None, None, Some(super::ServiceReadinessOutcome::Failed), false, &[]);
        assert!(readiness_failed.values().contains(&super::ServiceStateAssertion::Failed));
        assert!(!readiness_failed.is_up());
    }

    #[test]
    fn process_that_exits_before_readiness_reports_failed_not_ready() {
        let failed = project(Some(super::State::Failed), Some("failed"), None, false, &[]);
        assert!(failed.values().contains(&super::ServiceStateAssertion::Failed));
        assert!(!failed.contains_ready());
        assert!(!failed.is_up());
    }

    #[test]
    fn user_defined_state_satisfies_no_built_in_requirement() {
        let custom = project(None, None, None, false, &["migrating".to_string()]);
        assert!(custom.contains_user_defined("migrating"));
        assert!(!custom.contains_ready());
        assert!(!custom.contains_complete());
        assert!(!custom.is_up());
        for requirement in [
            super::ServiceRequirement::Ready,
            super::ServiceRequirement::Complete,
            super::ServiceRequirement::Up,
        ] {
            assert!(!requirement.satisfied_by(&custom));
        }
    }

    #[test]
    fn projection_does_not_change_the_lifecycle_state() {
        let before = super::State::Ready;
        let projected = project(Some(before), None, None, false, &[]);
        assert_eq!(before, super::State::Ready);
        assert!(projected.contains_ready());
    }

    #[test]
    fn projection_rejects_empty_user_defined_state() {
        let error = super::project_service_assertions(&super::ServiceAssertionInput {
            lifecycle_state: None,
            extension_phase: None,
            readiness: None,
            completed_normally: false,
            user_defined: &[String::new()],
        })
        .expect_err("empty user-defined state");
        assert!(error.to_string().contains("user-defined service state"));
    }

    #[test]
    fn projection_rejects_unknown_extension_phase() {
        let error = super::project_service_assertions(&super::ServiceAssertionInput {
            lifecycle_state: None,
            extension_phase: Some("not-a-phase"),
            readiness: None,
            completed_normally: false,
            user_defined: &[],
        })
        .expect_err("unknown extension phase");
        assert!(error.to_string().contains("unknown system-extension phase"));
    }
