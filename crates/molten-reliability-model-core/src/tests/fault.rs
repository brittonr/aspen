//! Closed crash-fault matrix regressions.

fn endpoints() -> alloc::collections::BTreeSet<alloc::string::String> {
    ["a", "b", "c"].map(alloc::string::String::from).into_iter().collect()
}

// r[verify molten.consensus.live_fault_profile]
#[test]
fn partition_requires_exact_disjoint_complete_cohort() {
    let endpoints = endpoints();
    let mut profile = crate::fault::Profile {
        fault: crate::fault::Mechanism::TemporaryPartition {
            left: alloc::vec!["a".into()],
            right: alloc::vec!["b".into(), "c".into()],
            duration_ms: 10,
        },
        heal_deadline_ms: 100,
        recovery_deadline_ms: 100,
        stable_membership_ms: 10,
        final_read_deadline_ms: 10,
    };
    assert_eq!(crate::fault::admit(&profile, &endpoints), Ok(()));
    profile.fault = crate::fault::Mechanism::TemporaryPartition {
        left: alloc::vec!["a".into()],
        right: alloc::vec!["b".into()],
        duration_ms: 10,
    };
    assert_eq!(crate::fault::admit(&profile, &endpoints), Err(crate::fault::Issue::IncompletePartitionCohort));
    profile.fault = crate::fault::Mechanism::TemporaryPartition {
        left: alloc::vec!["a".into()],
        right: alloc::vec!["a".into(), "c".into()],
        duration_ms: 10,
    };
    assert_eq!(crate::fault::admit(&profile, &endpoints), Err(crate::fault::Issue::InvalidPartitionSides));
}

// r[verify molten.consensus.live_fault_profile]
#[test]
fn disruptive_duration_must_fit_heal_deadline() {
    let endpoints = endpoints();
    let mut profile = crate::fault::Profile {
        fault: crate::fault::Mechanism::NoFault,
        heal_deadline_ms: 10,
        recovery_deadline_ms: 100,
        stable_membership_ms: 10,
        final_read_deadline_ms: 10,
    };
    assert_eq!(crate::fault::admit(&profile, &endpoints), Ok(()));
    for duration_ms in [profile.heal_deadline_ms, profile.heal_deadline_ms + 1] {
        let expected = if duration_ms <= profile.heal_deadline_ms {
            Ok(())
        } else {
            Err(crate::fault::Issue::FaultDurationExceedsHealDeadline)
        };
        for fault in [
            crate::fault::Mechanism::ProcessRestart {
                endpoint: "a".into(),
                down_ms: duration_ms,
            },
            crate::fault::Mechanism::TemporaryPartition {
                left: alloc::vec!["a".into()],
                right: alloc::vec!["b".into(), "c".into()],
                duration_ms,
            },
        ] {
            profile.fault = fault;
            assert_eq!(crate::fault::admit(&profile, &endpoints), expected, "{:?}", profile.fault);
        }
    }
}

#[test]
fn cohort_bounds_unknown_endpoints_and_typed_issue_names_fail_closed() {
    let profile = crate::fault::Profile {
        fault: crate::fault::Mechanism::ProcessRestart {
            endpoint: "z".into(),
            down_ms: 5,
        },
        heal_deadline_ms: 10,
        recovery_deadline_ms: 100,
        stable_membership_ms: 10,
        final_read_deadline_ms: 10,
    };
    assert_eq!(crate::fault::admit(&profile, &endpoints()), Err(crate::fault::Issue::UnknownFaultEndpoint));
    let two: alloc::collections::BTreeSet<alloc::string::String> =
        ["a", "b"].map(alloc::string::String::from).into_iter().collect();
    assert_eq!(crate::fault::admit(&profile, &two), Err(crate::fault::Issue::InvalidEndpointCohort));
    let mut unstable = profile.clone();
    unstable.fault = crate::fault::Mechanism::NoFault;
    unstable.stable_membership_ms = 101;
    assert_eq!(crate::fault::admit(&unstable, &endpoints()), Err(crate::fault::Issue::UnstableRecoveryBound));
    let mut unbounded = unstable;
    unbounded.final_read_deadline_ms = 0;
    assert_eq!(crate::fault::admit(&unbounded, &endpoints()), Err(crate::fault::Issue::InvalidFaultBound));
    assert_eq!(crate::fault::Issue::IncompletePartitionCohort.as_str(), "incomplete-partition-cohort");
    assert_eq!(std::string::ToString::to_string(&crate::fault::Issue::InvalidFaultBound), "invalid-fault-bound");
}

// r[verify molten.testing.live_reliability_claim_boundary]
#[test]
fn unsupported_faults_do_not_deserialize_into_supported_profile() {
    for kind in ["clock", "durability", "byzantine", "queue", "lock", "transaction"] {
        assert!(serde_json::from_value::<crate::fault::Mechanism>(serde_json::json!({"kind":kind})).is_err());
    }
}
