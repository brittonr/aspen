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
pub struct Capability {
    pub id: &'static str,
    pub execution_profile: &'static str,
    pub environment: Environment,
    pub adapter: &'static str,
    pub adapter_scope: &'static str,
}

pub const HARNESS: Capability = Capability {
    id: "harness-runtime-capability-binding",
    execution_profile: "harness-simulation-v1",
    environment: Environment::Simulation,
    adapter: "native-runtime-core",
    adapter_scope: "deterministic-logical-time-seeded-rng-no-live-ports",
};
pub const DURABLE_STATE_RECOVERY: Capability = Capability {
    id: "durable-state-recovery",
    execution_profile: "native-host-redb-v1",
    environment: Environment::SameProcess,
    adapter: "redb-durable-state-adapter",
    adapter_scope: "capability-rooted-redb-store-restart-by-close-and-reopen",
};
pub const EFFECT_UNCERTAINTY: Capability = Capability {
    id: "effect-uncertainty",
    execution_profile: "native-process",
    environment: Environment::DistinctProcess,
    adapter: "bounded-exec-live-process",
    adapter_scope: "unix-process-group-child-effect-no-automatic-retry",
};
pub const CROSS_PROCESS_TRANSPORT: Capability = Capability {
    id: "cross-process-transport",
    execution_profile: "iroh-cross-process-v1",
    environment: Environment::DistinctProcess,
    adapter: "iroh-direct-loopback",
    adapter_scope: "relay-disabled-loopback-peer-process",
};
pub const WORLD_REPLAY_PROMOTION_RESTORE: Capability = Capability {
    id: "world-replay-promotion-restore",
    execution_profile: "runtime-spine-committed-turn-v1",
    environment: Environment::SameProcess,
    adapter: "runtime-spine-committed-turn",
    adapter_scope: "capability-rooted-local-world-stores-current-admission",
};
pub const CONTROL_PLANE_FAILOVER: Capability = Capability {
    id: "control-plane-failover",
    execution_profile: "live-raft-static-v1",
    environment: Environment::DistinctProcess,
    adapter: "iroh-live-raft-static-three-voter",
    adapter_scope: "loopback-replica-processes-durable-restart",
};

pub const CAPABILITIES: [Capability; 6] = [
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
    pub capability: Capability,
    pub family: &'static str,
    pub requirement: &'static str,
    pub coverage_kind: &'static str,
}

const POSITIVE: &str = "positive";
const NEGATIVE: &str = "negative";

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

    pub const fn definition(self) -> Definition {
        const fn define(
            id: &'static str,
            capability: Capability,
            family: &'static str,
            requirement: &'static str,
            coverage_kind: &'static str,
        ) -> Definition {
            Definition {
                id,
                capability,
                family,
                requirement,
                coverage_kind,
            }
        }
        const REQUEST_BINDING: &str = "molten.testing.capability_context.request_binding";
        const SNAPSHOT_RECOVERY: &str = "molten.fabric_durability.snapshot_recovery";
        const EXECUTION_UNCERTAINTY: &str = "molten.fabric_execution.uncertainty";
        const NATIVE_UNCERTAINTY: &str = "molten.system_extension.native_effect_uncertainty";
        const ENDPOINT: &str = "molten.fabric_transport.cross_process_endpoint";
        const SESSION: &str = "molten.fabric_transport.cross_process_session";
        const REPLAY: &str = "molten.world_replay.execution_boundary";
        const RESTORE: &str = "molten.world_snapshot.restore";
        const ACTIVATION: &str = "molten.world_branch_authority.activation";
        const LIVE_RAFT: &str = "molten.fabric_consistency.live_raft";
        const FINAL_VALIDATION: &str = "molten.fabric_consistency.final_validation";
        const HARNESS_FAMILY: &str = "harness-simulation";
        const DURABLE_FAMILY: &str = "durable-restart-same-process";
        const EFFECT_FAMILY: &str = "post-effect-uncertainty-distinct-process";
        const TRANSPORT_FAMILY: &str = "frame-and-fencing-distinct-process";
        const WORLD_FAMILY: &str = "restore-current-admission-same-process";
        const RAFT_FAMILY: &str = "quorum-loss-and-recovery-distinct-process";
        match self {
            Self::GrantedSend => define("granted-send", HARNESS, HARNESS_FAMILY, REQUEST_BINDING, POSITIVE),
            Self::DeniedSend => define("denied-send", HARNESS, HARNESS_FAMILY, REQUEST_BINDING, NEGATIVE),
            Self::DurableRestartRecovery => {
                define("durable-restart-recovery", DURABLE_STATE_RECOVERY, DURABLE_FAMILY, SNAPSHOT_RECOVERY, POSITIVE)
            }
            Self::DurableCorruptSnapshotDenied => define(
                "durable-corrupt-snapshot-denied",
                DURABLE_STATE_RECOVERY,
                DURABLE_FAMILY,
                SNAPSHOT_RECOVERY,
                NEGATIVE,
            ),
            Self::ExecutionUnknownAfterStart => define(
                "execution-unknown-after-start",
                EFFECT_UNCERTAINTY,
                EFFECT_FAMILY,
                EXECUTION_UNCERTAINTY,
                POSITIVE,
            ),
            Self::ExecutionPreStartDefinite => define(
                "execution-prestart-definite",
                EFFECT_UNCERTAINTY,
                EFFECT_FAMILY,
                EXECUTION_UNCERTAINTY,
                NEGATIVE,
            ),
            Self::NativeWorkUnknownAfterEffect => define(
                "native-work-unknown-after-effect",
                EFFECT_UNCERTAINTY,
                EFFECT_FAMILY,
                NATIVE_UNCERTAINTY,
                POSITIVE,
            ),
            Self::NativeUncertainWorkNotReadmitted => define(
                "native-uncertain-work-not-readmitted",
                EFFECT_UNCERTAINTY,
                EFFECT_FAMILY,
                NATIVE_UNCERTAINTY,
                NEGATIVE,
            ),
            Self::TransportEndpointImport => {
                define("transport-endpoint-import", CROSS_PROCESS_TRANSPORT, TRANSPORT_FAMILY, ENDPOINT, POSITIVE)
            }
            Self::TransportEndpointTamperDenied => define(
                "transport-endpoint-tamper-denied",
                CROSS_PROCESS_TRANSPORT,
                TRANSPORT_FAMILY,
                ENDPOINT,
                NEGATIVE,
            ),
            Self::TransportSessionAcknowledged => {
                define("transport-session-acknowledged", CROSS_PROCESS_TRANSPORT, TRANSPORT_FAMILY, SESSION, POSITIVE)
            }
            Self::TransportSessionLostAcknowledgement => define(
                "transport-session-lost-acknowledgement",
                CROSS_PROCESS_TRANSPORT,
                TRANSPORT_FAMILY,
                SESSION,
                NEGATIVE,
            ),
            Self::WorldReplayAdmitted => {
                define("world-replay-admitted", WORLD_REPLAY_PROMOTION_RESTORE, WORLD_FAMILY, REPLAY, POSITIVE)
            }
            Self::WorldReplayAuthorityDenied => {
                define("world-replay-authority-denied", WORLD_REPLAY_PROMOTION_RESTORE, WORLD_FAMILY, REPLAY, NEGATIVE)
            }
            Self::WorldRestoreCurrentAdmission => define(
                "world-restore-current-admission",
                WORLD_REPLAY_PROMOTION_RESTORE,
                WORLD_FAMILY,
                RESTORE,
                POSITIVE,
            ),
            Self::WorldRestoreStaleAdmissionDenied => define(
                "world-restore-stale-admission-denied",
                WORLD_REPLAY_PROMOTION_RESTORE,
                WORLD_FAMILY,
                RESTORE,
                NEGATIVE,
            ),
            Self::WorldActivationCurrentFacts => define(
                "world-activation-current-facts",
                WORLD_REPLAY_PROMOTION_RESTORE,
                WORLD_FAMILY,
                ACTIVATION,
                POSITIVE,
            ),
            Self::WorldActivationRevokedDenied => define(
                "world-activation-revoked-denied",
                WORLD_REPLAY_PROMOTION_RESTORE,
                WORLD_FAMILY,
                ACTIVATION,
                NEGATIVE,
            ),
            Self::RaftFailoverCommitSurvives => {
                define("raft-failover-commit-survives", CONTROL_PLANE_FAILOVER, RAFT_FAMILY, LIVE_RAFT, POSITIVE)
            }
            Self::RaftStaleLeaderFenced => {
                define("raft-stale-leader-fenced", CONTROL_PLANE_FAILOVER, RAFT_FAMILY, LIVE_RAFT, NEGATIVE)
            }
            Self::RaftFinalQuorumEvidence => {
                define("raft-final-quorum-evidence", CONTROL_PLANE_FAILOVER, RAFT_FAMILY, FINAL_VALIDATION, POSITIVE)
            }
            Self::RaftFalseQuorumDenied => {
                define("raft-false-quorum-denied", CONTROL_PLANE_FAILOVER, RAFT_FAMILY, FINAL_VALIDATION, NEGATIVE)
            }
        }
    }

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

pub fn adapter_ref(capability: &Capability) -> crate::error::Result<String> {
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
    capability: &Capability,
) -> crate::error::Result<Subject> {
    Ok(Subject {
        source_ref: binding.source_ref.clone(),
        build_ref: binding.executable_ref.clone(),
        dependency_closure_ref: binding.dependencies_ref.clone(),
        feature_set: features()?,
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

pub(super) fn features() -> crate::error::Result<Vec<String>> {
    let cohort = [
        ("chaoscontrol-conformance", cfg!(feature = "chaoscontrol-conformance")),
        ("doltlite-oracle", cfg!(feature = "doltlite-oracle")),
        ("component-rpc", cfg!(feature = "component-rpc")),
        ("component-rpc-telemetry", cfg!(feature = "component-rpc-telemetry")),
        ("executable-extents", cfg!(feature = "executable-extents")),
        ("job-dag-construction", cfg!(feature = "job-dag-construction")),
        ("marble-object-store", cfg!(feature = "marble-object-store")),
        ("profiler", cfg!(feature = "profiler")),
        ("profiler-alloc", cfg!(feature = "profiler-alloc")),
        ("profiler-disabled", cfg!(feature = "profiler-disabled")),
        ("profiler-perf", cfg!(feature = "profiler-perf")),
        ("spacewasm-pilot", cfg!(feature = "spacewasm-pilot")),
        ("world-snapshot-vm-cohort", cfg!(feature = "world-snapshot-vm-cohort")),
        ("content-store-object-storage", cfg!(feature = "content-store-object-storage")),
        ("test-fixtures", cfg!(feature = "test-fixtures")),
    ];
    let manifest: toml::Value = toml::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")))
        .map_err(|_| super::invalid("compiled Cargo feature manifest cannot be decoded"))?;
    let declared = manifest
        .get("features")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| super::invalid("compiled Cargo feature inventory missing"))?;
    if declared.len() != cohort.len()
        || declared.keys().any(|name| !cohort.iter().any(|(known, _)| name.as_str() == *known))
    {
        return Err(super::invalid(
            "compiled feature inventory drift; qualification producer requires explicit update",
        ));
    }
    let target_dependencies = manifest
        .get("target")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|targets| targets.values())
        .filter_map(|target| target.get("dependencies").and_then(toml::Value::as_table));
    for dependencies in
        manifest.get("dependencies").and_then(toml::Value::as_table).into_iter().chain(target_dependencies)
    {
        for (name, dependency) in dependencies {
            let is_optional = dependency.get("optional").and_then(toml::Value::as_bool) == Some(true);
            let is_explicitly_hidden = declared
                .values()
                .filter_map(toml::Value::as_array)
                .flatten()
                .filter_map(toml::Value::as_str)
                .any(|feature| feature.strip_prefix("dep:") == Some(name.as_str()));
            if is_optional && !declared.contains_key(name) && !is_explicitly_hidden {
                return Err(super::invalid(
                    "implicit optional Cargo feature has no compiled observation; qualification denied",
                ));
            }
        }
    }
    let mut result: Vec<_> =
        cohort.into_iter().filter(|(_, enabled)| *enabled).map(|(name, _)| name.to_string()).collect();
    result.sort();
    Ok(result)
}
