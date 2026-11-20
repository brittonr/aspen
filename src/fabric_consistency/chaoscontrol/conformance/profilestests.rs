use std::collections::BTreeSet;

// r[verify molten.consensus.chaoscontrol_fault_profile]
#[test]
fn matrix_declares_the_control_and_every_bounded_scenario() {
    let plan = super::testfixtures::plan();
    let matrix = super::profiles::matrix(&plan.profile.bounds, super::profiles::CampaignLimits::DEFAULT)
        .expect("bounded matrix");
    assert_eq!(matrix.schema, super::profiles::MATRIX_SCHEMA);
    assert_eq!(matrix.control.campaign_id, super::profiles::NO_FAULT_CONTROL_ID);
    assert_eq!(matrix.campaigns.len(), super::profiles::Scenario::ALL.len());
    let mut identities = BTreeSet::new();
    let mut classes = BTreeSet::new();
    for campaign in &matrix.campaigns {
        assert!(identities.insert(campaign.campaign_id.as_str()));
        assert!(campaign.includes_no_fault_control);
        assert_eq!(campaign.expected_observability, "applied-and-observed");
        assert_eq!(campaign.classes.len(), 1);
        classes.insert(campaign.classes[0].class);
    }
    assert_eq!(
        classes,
        BTreeSet::from([
            chaoscontrol_smr::FaultClass::Network,
            chaoscontrol_smr::FaultClass::Process,
        ])
    );
}

#[test]
fn every_declared_campaign_is_producer_valid() {
    let plan = super::testfixtures::plan();
    for scenario in super::profiles::Scenario::ALL {
        let campaign =
            super::profiles::campaign(scenario, &plan.profile.bounds, super::profiles::CampaignLimits::DEFAULT)
                .expect("scenario declaration");
        chaoscontrol_smr::validate_fault_campaign(&campaign, &plan.profile.bounds)
            .expect("producer campaign validation");
        assert_eq!(campaign.classes[0].class, scenario.class());
    }
}

#[test]
fn unsupported_faults_are_typed_denials() {
    for fault in [
        super::profiles::UnsupportedFault::DiskCorruption,
        super::profiles::UnsupportedFault::ClockAnomaly,
        super::profiles::UnsupportedFault::Byzantine,
    ] {
        assert_eq!(fault.admit(), Err(super::profiles::ProfileIssue::Unsupported));
    }
    let plan = super::testfixtures::plan();
    let matrix = super::profiles::matrix(&plan.profile.bounds, super::profiles::CampaignLimits::DEFAULT)
        .expect("bounded matrix");
    for campaign in &matrix.campaigns {
        for weighted in &campaign.classes {
            assert!(!matches!(
                weighted.class,
                chaoscontrol_smr::FaultClass::Storage
                    | chaoscontrol_smr::FaultClass::Clock
                    | chaoscontrol_smr::FaultClass::Scheduler
            ));
        }
    }
}

#[test]
fn campaign_limits_cannot_exceed_the_workload_bounds() {
    let plan = super::testfixtures::plan();
    let mut bounds = plan.profile.bounds.clone();
    bounds.maximum_commands = 4;
    assert_eq!(
        super::profiles::matrix(&bounds, super::profiles::CampaignLimits::DEFAULT).unwrap_err(),
        super::profiles::ProfileIssue::Bounds
    );
    let mut bounds = plan.profile.bounds.clone();
    bounds.maximum_fault_actions = 2;
    assert_eq!(
        super::profiles::matrix(&bounds, super::profiles::CampaignLimits::DEFAULT).unwrap_err(),
        super::profiles::ProfileIssue::Bounds
    );
    let mut bounds = plan.profile.bounds.clone();
    bounds.maximum_clients = 1;
    assert_eq!(
        super::profiles::matrix(&bounds, super::profiles::CampaignLimits::DEFAULT).unwrap_err(),
        super::profiles::ProfileIssue::Bounds
    );
}

#[test]
fn activation_window_must_be_finite_and_inside_the_virtual_duration() {
    let plan = super::testfixtures::plan();
    for limits in [
        super::profiles::CampaignLimits {
            activation_start: 8,
            activation_end: 8,
            ..super::profiles::CampaignLimits::DEFAULT
        },
        super::profiles::CampaignLimits {
            activation_start: 8,
            activation_end: 65,
            ..super::profiles::CampaignLimits::DEFAULT
        },
        super::profiles::CampaignLimits {
            client_concurrency: 4,
            ..super::profiles::CampaignLimits::DEFAULT
        },
    ] {
        assert_eq!(
            super::profiles::campaign(super::profiles::Scenario::Partition, &plan.profile.bounds, limits).unwrap_err(),
            super::profiles::ProfileIssue::Invalid
        );
    }
}

#[test]
fn scenario_identities_and_classes_are_stable() {
    let expected = [
        ("molten-message-loss", chaoscontrol_smr::FaultClass::Network),
        ("molten-reorder", chaoscontrol_smr::FaultClass::Network),
        ("molten-partition", chaoscontrol_smr::FaultClass::Network),
        ("molten-quorum-loss", chaoscontrol_smr::FaultClass::Network),
        ("molten-leader-crash", chaoscontrol_smr::FaultClass::Process),
        ("molten-follower-crash", chaoscontrol_smr::FaultClass::Process),
        ("molten-restart", chaoscontrol_smr::FaultClass::Process),
        ("molten-snapshot-catch-up", chaoscontrol_smr::FaultClass::Process),
    ];
    for (scenario, (id, class)) in super::profiles::Scenario::ALL.into_iter().zip(expected) {
        assert_eq!(scenario.id(), id);
        assert_eq!(scenario.class(), class);
    }
}
