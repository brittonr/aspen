//! Closed initial crash-fault matrix; unsupported mechanisms cannot acquire live claims.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Mechanism {
    NoFault,
    ProcessRestart {
        endpoint: String,
        down_ms: u64,
    },
    TemporaryPartition {
        left: Vec<String>,
        right: Vec<String>,
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub fault: Mechanism,
    pub heal_deadline_ms: u64,
    pub recovery_deadline_ms: u64,
    pub stable_membership_ms: u64,
    pub final_read_deadline_ms: u64,
}

pub const UNSUPPORTED: &[&str] = &[
    "clock",
    "durability",
    "byzantine",
    "queue",
    "lock",
    "transaction",
    "production-readiness",
    "release-eligibility",
    "universal-consensus-correctness",
];

// r[impl molten.consensus.live_fault_profile]
pub fn admit(profile: &Profile, endpoints: &std::collections::BTreeSet<String>) -> Result<(), &'static str> {
    if endpoints.len() < 3 || endpoints.len() > 9 {
        return Err("invalid-endpoint-cohort");
    }
    for bound in [
        profile.heal_deadline_ms,
        profile.recovery_deadline_ms,
        profile.stable_membership_ms,
        profile.final_read_deadline_ms,
    ] {
        if !(1..=120_000).contains(&bound) {
            return Err("invalid-fault-bound");
        }
    }
    if profile.stable_membership_ms > profile.recovery_deadline_ms {
        return Err("unstable-recovery-bound");
    }
    match &profile.fault {
        Mechanism::NoFault => Ok(()),
        Mechanism::ProcessRestart { endpoint, down_ms } => {
            if !endpoints.contains(endpoint) {
                return Err("unknown-fault-endpoint");
            }
            admit_disruption(*down_ms, profile.heal_deadline_ms)
        }
        Mechanism::TemporaryPartition {
            left,
            right,
            duration_ms,
        } => {
            admit_disruption(*duration_ms, profile.heal_deadline_ms)?;
            let left_set: std::collections::BTreeSet<_> = left.iter().cloned().collect();
            let right_set: std::collections::BTreeSet<_> = right.iter().cloned().collect();
            if left_set.is_empty()
                || right_set.is_empty()
                || left_set.len() != left.len()
                || right_set.len() != right.len()
                || !left_set.is_disjoint(&right_set)
            {
                return Err("invalid-partition-sides");
            }
            if left_set.union(&right_set).cloned().collect::<std::collections::BTreeSet<_>>() != *endpoints {
                return Err("incomplete-partition-cohort");
            }
            Ok(())
        }
    }
}

fn admit_disruption(duration_ms: u64, heal_deadline_ms: u64) -> Result<(), &'static str> {
    if duration_ms == 0 {
        return Err("invalid-fault-bound");
    }
    if duration_ms > heal_deadline_ms {
        return Err("fault-duration-exceeds-heal-deadline");
    }
    Ok(())
}

// r[verify molten.consensus.live_fault_profile]
#[test]
fn partition_requires_exact_disjoint_complete_cohort() {
    let endpoints = ["a", "b", "c"].map(String::from).into_iter().collect();
    let mut profile = Profile {
        fault: Mechanism::TemporaryPartition {
            left: vec!["a".into()],
            right: vec!["b".into(), "c".into()],
            duration_ms: 10,
        },
        heal_deadline_ms: 100,
        recovery_deadline_ms: 100,
        stable_membership_ms: 10,
        final_read_deadline_ms: 10,
    };
    assert_eq!(admit(&profile, &endpoints), Ok(()));
    profile.fault = Mechanism::TemporaryPartition {
        left: vec!["a".into()],
        right: vec!["b".into()],
        duration_ms: 10,
    };
    assert_eq!(admit(&profile, &endpoints), Err("incomplete-partition-cohort"));
    profile.fault = Mechanism::TemporaryPartition {
        left: vec!["a".into()],
        right: vec!["a".into(), "c".into()],
        duration_ms: 10,
    };
    assert_eq!(admit(&profile, &endpoints), Err("invalid-partition-sides"));
}

// r[verify molten.consensus.live_fault_profile]
#[test]
fn disruptive_duration_must_fit_heal_deadline() {
    let endpoints = ["a", "b", "c"].map(String::from).into_iter().collect();
    let mut profile = Profile {
        fault: Mechanism::NoFault,
        heal_deadline_ms: 10,
        recovery_deadline_ms: 100,
        stable_membership_ms: 10,
        final_read_deadline_ms: 10,
    };
    assert_eq!(admit(&profile, &endpoints), Ok(()));
    for duration_ms in [profile.heal_deadline_ms, profile.heal_deadline_ms + 1] {
        let expected = if duration_ms <= profile.heal_deadline_ms {
            Ok(())
        } else {
            Err("fault-duration-exceeds-heal-deadline")
        };
        for fault in [
            Mechanism::ProcessRestart {
                endpoint: "a".into(),
                down_ms: duration_ms,
            },
            Mechanism::TemporaryPartition {
                left: vec!["a".into()],
                right: vec!["b".into(), "c".into()],
                duration_ms,
            },
        ] {
            profile.fault = fault;
            assert_eq!(admit(&profile, &endpoints), expected, "{:?}", profile.fault);
        }
    }
}

// r[verify molten.testing.live_reliability_claim_boundary]
#[test]
fn unsupported_faults_do_not_deserialize_into_supported_profile() {
    for kind in ["clock", "durability", "byzantine", "queue", "lock", "transaction"] {
        assert!(serde_json::from_value::<Mechanism>(serde_json::json!({"kind":kind})).is_err());
    }
}
