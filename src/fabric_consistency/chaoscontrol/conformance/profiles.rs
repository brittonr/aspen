//! Molten's declared ChaosControl fault-campaign matrix.
//!
//! The pinned producer contract validates a campaign; Molten declares which
//! bounded scenarios it is prepared to claim. Disk, clock, and Byzantine
//! faults remain explicit unsupported requests. Declarations are not
//! execution evidence and cannot satisfy a campaign task by themselves.

/// Identity of this bounded declaration set.
pub const MATRIX_SCHEMA: &str = "molten.chaoscontrol-fault-matrix.v1";
/// The producer-owned control that every evidence campaign must include.
pub const NO_FAULT_CONTROL_ID: &str = chaoscontrol_smr::NO_FAULT_CONTROL_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileIssue {
    Bounds,
    Unsupported,
    Invalid,
}

impl std::fmt::Display for ProfileIssue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Bounds => "bounds",
            Self::Unsupported => "unsupported",
            Self::Invalid => "invalid",
        })
    }
}

impl std::error::Error for ProfileIssue {}

/// Bounded crash-fault scenarios Molten may request from the producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scenario {
    MessageLoss,
    Reorder,
    Partition,
    QuorumLoss,
    LeaderCrash,
    FollowerCrash,
    Restart,
    SnapshotCatchUp,
}

impl Scenario {
    pub const ALL: [Self; 8] = [
        Self::MessageLoss,
        Self::Reorder,
        Self::Partition,
        Self::QuorumLoss,
        Self::LeaderCrash,
        Self::FollowerCrash,
        Self::Restart,
        Self::SnapshotCatchUp,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::MessageLoss => "molten-message-loss",
            Self::Reorder => "molten-reorder",
            Self::Partition => "molten-partition",
            Self::QuorumLoss => "molten-quorum-loss",
            Self::LeaderCrash => "molten-leader-crash",
            Self::FollowerCrash => "molten-follower-crash",
            Self::Restart => "molten-restart",
            Self::SnapshotCatchUp => "molten-snapshot-catch-up",
        }
    }

    pub const fn class(self) -> chaoscontrol_smr::FaultClass {
        match self {
            Self::MessageLoss | Self::Reorder | Self::Partition | Self::QuorumLoss => {
                chaoscontrol_smr::FaultClass::Network
            }
            Self::LeaderCrash | Self::FollowerCrash | Self::Restart | Self::SnapshotCatchUp => {
                chaoscontrol_smr::FaultClass::Process
            }
        }
    }
}

/// Fault classes Molten does not claim through this profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedFault {
    DiskCorruption,
    ClockAnomaly,
    Byzantine,
}

impl UnsupportedFault {
    pub const fn id(self) -> &'static str {
        match self {
            Self::DiskCorruption => "disk-corruption",
            Self::ClockAnomaly => "clock-anomaly",
            Self::Byzantine => "byzantine",
        }
    }

    /// Every request is denied; the typed identity is retained for diagnostics.
    pub const fn admit(self) -> Result<(), ProfileIssue> {
        Err(ProfileIssue::Unsupported)
    }
}

/// Finite limits shared by every declared campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignLimits {
    pub command_count: u64,
    pub client_count: u32,
    pub client_concurrency: u32,
    pub maximum_concurrent_faults: u32,
    pub maximum_actions: u64,
    pub virtual_duration: u64,
    pub activation_start: u64,
    pub activation_end: u64,
}

impl CampaignLimits {
    pub const DEFAULT: Self = Self {
        command_count: 8,
        client_count: 3,
        client_concurrency: 1,
        maximum_concurrent_faults: 1,
        maximum_actions: 4,
        virtual_duration: 64,
        activation_start: 8,
        activation_end: 48,
    };
}

/// The no-fault baseline run. It is not a fault campaign and selects no class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoFaultControl {
    pub campaign_id: String,
    pub command_count: u64,
    pub client_count: u32,
    pub client_concurrency: u32,
    pub virtual_duration: u64,
    pub terminal_rule: String,
}

/// One validated declaration plus the producer control it must include.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultMatrix {
    pub schema: &'static str,
    pub control: NoFaultControl,
    pub campaigns: Vec<chaoscontrol_smr::FaultCampaignProfile>,
}

/// Declare the finite scenario matrix against exact workload bounds.
// r[impl molten.consensus.chaoscontrol_fault_profile]
pub fn matrix(bounds: &chaoscontrol_smr::WorkloadBounds, limits: CampaignLimits) -> Result<FaultMatrix, ProfileIssue> {
    let control = NoFaultControl {
        campaign_id: NO_FAULT_CONTROL_ID.to_string(),
        command_count: limits.command_count,
        client_count: limits.client_count,
        client_concurrency: limits.client_concurrency,
        virtual_duration: limits.virtual_duration,
        terminal_rule: "bounded-progress-horizon".to_string(),
    };
    validate_control(&control, bounds)?;
    let campaigns = Scenario::ALL
        .into_iter()
        .map(|scenario| campaign(scenario, bounds, limits))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FaultMatrix {
        schema: MATRIX_SCHEMA,
        control,
        campaigns,
    })
}

/// Declare one scenario as a producer-validated fault campaign.
pub fn campaign(
    scenario: Scenario,
    bounds: &chaoscontrol_smr::WorkloadBounds,
    limits: CampaignLimits,
) -> Result<chaoscontrol_smr::FaultCampaignProfile, ProfileIssue> {
    if limits.activation_start >= limits.activation_end
        || limits.activation_end > limits.virtual_duration
        || limits.command_count == 0
        || limits.client_count == 0
        || limits.client_concurrency == 0
        || limits.client_concurrency > limits.client_count
        || limits.maximum_concurrent_faults == 0
        || limits.maximum_actions == 0
        || limits.virtual_duration == 0
    {
        return Err(ProfileIssue::Invalid);
    }
    let profile = chaoscontrol_smr::FaultCampaignProfile {
        campaign_id: scenario.id().to_string(),
        includes_no_fault_control: true,
        classes: vec![chaoscontrol_smr::WeightedFaultClass {
            class: scenario.class(),
            weight: 1,
        }],
        optional_features: Vec::new(),
        command_count: limits.command_count,
        client_count: limits.client_count,
        client_concurrency: limits.client_concurrency,
        maximum_concurrent_faults: limits.maximum_concurrent_faults,
        maximum_actions: limits.maximum_actions,
        virtual_duration: limits.virtual_duration,
        terminal_rule: "bounded-progress-horizon".to_string(),
        activation_start: limits.activation_start,
        activation_end: limits.activation_end,
        expected_observability: "applied-and-observed".to_string(),
    };
    chaoscontrol_smr::validate_fault_campaign(&profile, bounds).map_err(|_| ProfileIssue::Bounds)?;
    Ok(profile)
}

fn validate_control(control: &NoFaultControl, bounds: &chaoscontrol_smr::WorkloadBounds) -> Result<(), ProfileIssue> {
    if control.campaign_id != NO_FAULT_CONTROL_ID
        || control.terminal_rule.is_empty()
        || control.command_count == 0
        || control.command_count > bounds.maximum_commands
        || control.client_count == 0
        || u64::from(control.client_count) > u64::from(bounds.maximum_clients)
        || control.client_concurrency == 0
        || control.client_concurrency > control.client_count
        || control.client_concurrency > bounds.maximum_concurrency
        || control.virtual_duration == 0
        || control.virtual_duration > bounds.maximum_virtual_progress
    {
        return Err(ProfileIssue::Bounds);
    }
    Ok(())
}
