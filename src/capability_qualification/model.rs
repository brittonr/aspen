mod manifest;
mod scenarios;

pub const SCHEMA: &str = "molten.capability-runtime-execution.v1";
pub const PRODUCT_OBSERVATION_SCHEMA: &str = "molten.capability-product-observation.v1";
pub const PRODUCT_PLAN_SCHEMA: &str = "molten.capability-product-plan.v1";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub source_ref: String,
    pub build_ref: String,
    pub dependency_closure_ref: String,
    pub feature_set: Vec<String>,
    pub execution_profile: String,
    pub adapter_refs: Vec<String>,
    pub platform: String,
    pub environment: Environment,
}

#[derive(
    Debug,
    Clone,
    Copy,
    serde::Deserialize,
    serde::Serialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord
)]
#[serde(rename_all = "kebab-case")]
pub enum Environment {
    Simulation,
    SameProcess,
    DistinctProcess,
    NixosVm,
    CrossHost,
}

/// One reviewed capability row from `config/capability-report/capabilities.json`
/// with the single execution profile, environment and adapter its native producer
/// actually runs. A subject never mixes environments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeProducer {
    pub id: &'static str,
    pub execution_profile: &'static str,
    pub environment: Environment,
    pub adapter: &'static str,
    pub adapter_scope: &'static str,
}

pub const HARNESS: NativeProducer = NativeProducer {
    id: "harness-runtime-capability-binding",
    execution_profile: "harness-simulation-v1",
    environment: Environment::Simulation,
    adapter: "native-runtime-core",
    adapter_scope: "deterministic-logical-time-seeded-rng-no-live-ports",
};
pub const DURABLE_STATE_RECOVERY: NativeProducer = NativeProducer {
    id: "durable-state-recovery",
    execution_profile: "native-host-redb-v1",
    environment: Environment::SameProcess,
    adapter: "redb-durable-state-adapter",
    adapter_scope: "capability-rooted-redb-store-restart-by-close-and-reopen",
};
pub const EFFECT_UNCERTAINTY: NativeProducer = NativeProducer {
    id: "effect-uncertainty",
    execution_profile: "native-process",
    environment: Environment::DistinctProcess,
    adapter: "bounded-exec-live-process",
    adapter_scope: "unix-process-group-child-effect-no-automatic-retry",
};
pub const CROSS_PROCESS_TRANSPORT: NativeProducer = NativeProducer {
    id: "cross-process-transport",
    execution_profile: "iroh-cross-process-v1",
    environment: Environment::DistinctProcess,
    adapter: "iroh-direct-loopback",
    adapter_scope: "relay-disabled-loopback-peer-process",
};
pub const WORLD_REPLAY_PROMOTION_RESTORE: NativeProducer = NativeProducer {
    id: "world-replay-promotion-restore",
    execution_profile: "runtime-spine-committed-turn-v1",
    environment: Environment::SameProcess,
    adapter: "runtime-spine-committed-turn",
    adapter_scope: "capability-rooted-local-world-stores-current-admission",
};
pub const CONTROL_PLANE_FAILOVER: NativeProducer = NativeProducer {
    id: "control-plane-failover",
    execution_profile: "live-raft-static-v1",
    environment: Environment::DistinctProcess,
    adapter: "iroh-live-raft-static-three-voter",
    adapter_scope: "loopback-replica-processes-durable-restart",
};

pub const CAPABILITIES: [NativeProducer; 6] = [
    HARNESS,
    DURABLE_STATE_RECOVERY,
    EFFECT_UNCERTAINTY,
    CROSS_PROCESS_TRANSPORT,
    WORLD_REPLAY_PROMOTION_RESTORE,
    CONTROL_PLANE_FAILOVER,
];

/// Fixed, finite scenarios. Each product scenario is bound to exactly one
/// requirement, one family and one coverage kind; there is no aliasing of one
/// execution across requirements or polarities.
#[derive(
    Debug,
    Clone,
    Copy,
    serde::Deserialize,
    serde::Serialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord
)]
#[serde(rename_all = "kebab-case")]
pub enum Scenario {
    GrantedSend,
    DeniedSend,
    DurableRestartRecovery,
    DurableCorruptSnapshotDenied,
    ExecutionUnknownAfterStart,
    ExecutionPreStartDefinite,
    NativeWorkUnknownAfterEffect,
    NativeUncertainWorkNotReadmitted,
    TransportEndpointImport,
    TransportEndpointTamperDenied,
    TransportSessionAcknowledged,
    TransportSessionLostAcknowledgement,
    WorldReplayAdmitted,
    WorldReplayAuthorityDenied,
    WorldRestoreCurrentAdmission,
    WorldRestoreStaleAdmissionDenied,
    WorldActivationCurrentFacts,
    WorldActivationRevokedDenied,
    RaftFailoverCommitSurvives,
    RaftStaleLeaderFenced,
    RaftFinalQuorumEvidence,
    RaftFalseQuorumDenied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Definition {
    pub id: &'static str,
    pub capability: NativeProducer,
    pub family: &'static str,
    pub requirement: &'static str,
    pub coverage_kind: &'static str,
}

impl Scenario {
    pub const ALL: [Self; 22] = [
        Self::GrantedSend,
        Self::DeniedSend,
        Self::DurableRestartRecovery,
        Self::DurableCorruptSnapshotDenied,
        Self::ExecutionUnknownAfterStart,
        Self::ExecutionPreStartDefinite,
        Self::NativeWorkUnknownAfterEffect,
        Self::NativeUncertainWorkNotReadmitted,
        Self::TransportEndpointImport,
        Self::TransportEndpointTamperDenied,
        Self::TransportSessionAcknowledged,
        Self::TransportSessionLostAcknowledgement,
        Self::WorldReplayAdmitted,
        Self::WorldReplayAuthorityDenied,
        Self::WorldRestoreCurrentAdmission,
        Self::WorldRestoreStaleAdmissionDenied,
        Self::WorldActivationCurrentFacts,
        Self::WorldActivationRevokedDenied,
        Self::RaftFailoverCommitSurvives,
        Self::RaftStaleLeaderFenced,
        Self::RaftFinalQuorumEvidence,
        Self::RaftFalseQuorumDenied,
    ];

    pub fn id(self) -> &'static str {
        self.definition().id
    }

    pub fn coverage_kind(self) -> &'static str {
        self.definition().coverage_kind
    }

    pub fn is_harness(self) -> bool {
        self.definition().capability == HARNESS
    }

    pub fn parse(value: &str) -> crate::error::Result<Self> {
        Self::ALL
            .into_iter()
            .find(|scenario| scenario.id() == value)
            .ok_or_else(|| super::invalid("unsupported fixed qualification scenario"))
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedSubject {
    /// One exact subject per reviewed capability id.
    pub subjects: std::collections::BTreeMap<String, Subject>,
    pub binding_ref: String,
    pub validation_ref: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedReadback {
    pub subjects: std::collections::BTreeMap<String, Subject>,
    pub binding_ref: String,
    pub validation_ref: String,
    pub receipt_refs: std::collections::BTreeMap<String, String>,
}

pub struct Produced {
    pub value: preserves::IOValue,
    pub receipt_ref: String,
    pub subject: Subject,
    pub decision: &'static str,
}

#[derive(Debug)]
pub struct Validated {
    pub requirement_id: &'static str,
    pub family: &'static str,
    pub scenario: Scenario,
    pub subject: Subject,
    pub decision: &'static str,
    pub canonical_decision: &'static str,
}

pub fn adapter_ref(capability: &NativeProducer) -> crate::error::Result<String> {
    let label = if *capability == HARNESS {
        "capability-harness-adapter-v1"
    } else {
        "capability-product-adapter-v1"
    };
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(label, vec![
        crate::preserves_rail::string(capability.adapter),
        crate::preserves_rail::string(capability.adapter_scope),
    ]))
}

pub(super) fn subject(
    binding: &crate::core_api::startup_binding::Binding,
    capability: &NativeProducer,
) -> crate::error::Result<Subject> {
    Ok(Subject {
        source_ref: binding.source_ref.clone(),
        build_ref: binding.executable_ref.clone(),
        dependency_closure_ref: binding.dependencies_ref.clone(),
        feature_set: manifest::features()?,
        execution_profile: capability.execution_profile.to_string(),
        adapter_refs: vec![adapter_ref(capability)?],
        platform: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        environment: capability.environment,
    })
}

pub(super) fn subjects(
    binding: &crate::core_api::startup_binding::Binding,
) -> crate::error::Result<std::collections::BTreeMap<String, Subject>> {
    CAPABILITIES
        .iter()
        .map(|capability| Ok((capability.id.to_string(), subject(binding, capability)?)))
        .collect()
}
