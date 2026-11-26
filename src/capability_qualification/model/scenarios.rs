//! The fixed capability, family, requirement and coverage binding of every
//! scenario.

const POSITIVE: &str = "positive";
const NEGATIVE: &str = "negative";

impl super::Scenario {
    pub const fn definition(self) -> super::Definition {
        const fn define(
            id: &'static str,
            capability: super::NativeProducer,
            family: &'static str,
            requirement: &'static str,
            coverage_kind: &'static str,
        ) -> super::Definition {
            super::Definition {
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
            Self::GrantedSend => define("granted-send", super::HARNESS, HARNESS_FAMILY, REQUEST_BINDING, POSITIVE),
            Self::DeniedSend => define("denied-send", super::HARNESS, HARNESS_FAMILY, REQUEST_BINDING, NEGATIVE),
            Self::DurableRestartRecovery => define(
                "durable-restart-recovery",
                super::DURABLE_STATE_RECOVERY,
                DURABLE_FAMILY,
                SNAPSHOT_RECOVERY,
                POSITIVE,
            ),
            Self::DurableCorruptSnapshotDenied => define(
                "durable-corrupt-snapshot-denied",
                super::DURABLE_STATE_RECOVERY,
                DURABLE_FAMILY,
                SNAPSHOT_RECOVERY,
                NEGATIVE,
            ),
            Self::ExecutionUnknownAfterStart => define(
                "execution-unknown-after-start",
                super::EFFECT_UNCERTAINTY,
                EFFECT_FAMILY,
                EXECUTION_UNCERTAINTY,
                POSITIVE,
            ),
            Self::ExecutionPreStartDefinite => define(
                "execution-prestart-definite",
                super::EFFECT_UNCERTAINTY,
                EFFECT_FAMILY,
                EXECUTION_UNCERTAINTY,
                NEGATIVE,
            ),
            Self::NativeWorkUnknownAfterEffect => define(
                "native-work-unknown-after-effect",
                super::EFFECT_UNCERTAINTY,
                EFFECT_FAMILY,
                NATIVE_UNCERTAINTY,
                POSITIVE,
            ),
            Self::NativeUncertainWorkNotReadmitted => define(
                "native-uncertain-work-not-readmitted",
                super::EFFECT_UNCERTAINTY,
                EFFECT_FAMILY,
                NATIVE_UNCERTAINTY,
                NEGATIVE,
            ),
            Self::TransportEndpointImport => define(
                "transport-endpoint-import",
                super::CROSS_PROCESS_TRANSPORT,
                TRANSPORT_FAMILY,
                ENDPOINT,
                POSITIVE,
            ),
            Self::TransportEndpointTamperDenied => define(
                "transport-endpoint-tamper-denied",
                super::CROSS_PROCESS_TRANSPORT,
                TRANSPORT_FAMILY,
                ENDPOINT,
                NEGATIVE,
            ),
            Self::TransportSessionAcknowledged => define(
                "transport-session-acknowledged",
                super::CROSS_PROCESS_TRANSPORT,
                TRANSPORT_FAMILY,
                SESSION,
                POSITIVE,
            ),
            Self::TransportSessionLostAcknowledgement => define(
                "transport-session-lost-acknowledgement",
                super::CROSS_PROCESS_TRANSPORT,
                TRANSPORT_FAMILY,
                SESSION,
                NEGATIVE,
            ),
            Self::WorldReplayAdmitted => {
                define("world-replay-admitted", super::WORLD_REPLAY_PROMOTION_RESTORE, WORLD_FAMILY, REPLAY, POSITIVE)
            }
            Self::WorldReplayAuthorityDenied => define(
                "world-replay-authority-denied",
                super::WORLD_REPLAY_PROMOTION_RESTORE,
                WORLD_FAMILY,
                REPLAY,
                NEGATIVE,
            ),
            Self::WorldRestoreCurrentAdmission => define(
                "world-restore-current-admission",
                super::WORLD_REPLAY_PROMOTION_RESTORE,
                WORLD_FAMILY,
                RESTORE,
                POSITIVE,
            ),
            Self::WorldRestoreStaleAdmissionDenied => define(
                "world-restore-stale-admission-denied",
                super::WORLD_REPLAY_PROMOTION_RESTORE,
                WORLD_FAMILY,
                RESTORE,
                NEGATIVE,
            ),
            Self::WorldActivationCurrentFacts => define(
                "world-activation-current-facts",
                super::WORLD_REPLAY_PROMOTION_RESTORE,
                WORLD_FAMILY,
                ACTIVATION,
                POSITIVE,
            ),
            Self::WorldActivationRevokedDenied => define(
                "world-activation-revoked-denied",
                super::WORLD_REPLAY_PROMOTION_RESTORE,
                WORLD_FAMILY,
                ACTIVATION,
                NEGATIVE,
            ),
            Self::RaftFailoverCommitSurvives => {
                define("raft-failover-commit-survives", super::CONTROL_PLANE_FAILOVER, RAFT_FAMILY, LIVE_RAFT, POSITIVE)
            }
            Self::RaftStaleLeaderFenced => {
                define("raft-stale-leader-fenced", super::CONTROL_PLANE_FAILOVER, RAFT_FAMILY, LIVE_RAFT, NEGATIVE)
            }
            Self::RaftFinalQuorumEvidence => define(
                "raft-final-quorum-evidence",
                super::CONTROL_PLANE_FAILOVER,
                RAFT_FAMILY,
                FINAL_VALIDATION,
                POSITIVE,
            ),
            Self::RaftFalseQuorumDenied => define(
                "raft-false-quorum-denied",
                super::CONTROL_PLANE_FAILOVER,
                RAFT_FAMILY,
                FINAL_VALIDATION,
                NEGATIVE,
            ),
        }
    }
}
