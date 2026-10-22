// Shared definitions for the service recovery composition.
//
// One durable stateful service, one dependent worker in the same recovery
// group, and one independent sibling outside it. Both harness tracks consume
// these definitions.

use molten_core::system_extension::*;

/// The three services in the composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceRole {
    /// The durable stateful service the dependent worker needs.
    Stateful,
    /// The dependent worker, in the same recovery group and ordered after the
    /// stateful service.
    DependentWorker,
    /// The independent sibling, outside the recovery group.
    Sibling,
}

impl ServiceRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stateful => "stateful",
            Self::DependentWorker => "dependent-worker",
            Self::Sibling => "sibling",
        }
    }
}

/// The properties the composition asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceProperty {
    /// Unaffected services continue while a member fails and recovers.
    UnaffectedServicesContinue,
    /// Stale instances lose influence.
    StaleInstancesLoseInfluence,
    /// Durable logical work is not silently lost and is readmitted under a
    /// current claim.
    DurableWorkIsReadmitted,
    /// Uncertain external effects are surfaced as unknown outcomes and not
    /// blindly repeated.
    UncertainEffectsAreNotRepeated,
    /// Restart storms terminate or escalate within the admitted window.
    StormsTerminateWithinWindow,
}

impl ServiceProperty {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnaffectedServicesContinue => "unaffected-services-continue",
            Self::StaleInstancesLoseInfluence => "stale-instances-lose-influence",
            Self::DurableWorkIsReadmitted => "durable-work-is-readmitted",
            Self::UncertainEffectsAreNotRepeated => "uncertain-effects-are-not-repeated",
            Self::StormsTerminateWithinWindow => "storms-terminate-within-window",
        }
    }
}

/// The shared composition definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupDefinitions {
    pub group: RecoveryGroupDeclaration,
    pub stateful_facade: AdmittedFacadeManifest,
    pub worker_facade: AdmittedFacadeManifest,
    pub sibling_facade: AdmittedFacadeManifest,
    pub durable_profile_ref: String,
    pub durable_descriptor_ref: String,
}

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const HASH_E: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const FACADE_MANIFEST_SCHEMA: &str = "molten.system-extension.facade-manifest.v1";
const STATEFUL_SERVICE_ID: &str = "molten.composition.stateful";
const WORKER_SERVICE_ID: &str = "molten.composition.worker";
const SIBLING_SERVICE_ID: &str = "molten.composition.sibling";
const GROUP_ID: &str = "molten.composition.group";
const RECOVERY_WINDOW_TICKS: u64 = 100;
const RECOVERY_BACKOFF_TICKS: u64 = 5;
const MAX_RESTARTS_PER_MEMBER: u64 = 4;
const MAX_RESTARTS_PER_GROUP: u64 = 3;

fn facade_input(profile: FacadeProfile, service_id: &str) -> FacadeManifestInput {
    FacadeManifestInput {
        schema: FACADE_MANIFEST_SCHEMA.to_string(),
        extension_manifest_ref: HASH_A.to_string(),
        service_id: service_id.to_string(),
        profile,
        event_schema_ref: HASH_B.to_string(),
        state_schema_ref: HASH_C.to_string(),
        reply_schema_ref: HASH_D.to_string(),
        completion_schema_ref: None,
        phase_table: Vec::new(),
        concurrency_limit: 2,
    }
}

fn stateful_facade() -> Result<AdmittedFacadeManifest, Vec<FacadeAdmissionIssue>> {
    let mut input = facade_input(FacadeProfile::StateMachine, STATEFUL_SERVICE_ID);
    let mut idle = FacadePhaseEntry {
        phase: "idle".to_string(),
        legal_events: vec!["accept".to_string()],
        postponable_events: vec!["drain-request".to_string()],
        postponement_bound: Some(2),
        state_timeout_ticks: Some(30),
        timeout_fencing: Some(TimeoutFencing::IncarnationAndPhase),
    };
    idle.postponement_bound = Some(2);
    let running = FacadePhaseEntry {
        phase: "running".to_string(),
        legal_events: vec!["apply".to_string()],
        postponable_events: vec!["drain-request".to_string()],
        postponement_bound: Some(2),
        state_timeout_ticks: None,
        timeout_fencing: None,
    };
    input.phase_table = vec![idle, running];
    admit_facade_manifest(&input)
}

fn service_facade(service_id: &str) -> Result<AdmittedFacadeManifest, Vec<FacadeAdmissionIssue>> {
    admit_facade_manifest(&facade_input(FacadeProfile::Service, service_id))
}

/// The composition definitions: three services, one recovery group.
pub fn group_definitions() -> Result<GroupDefinitions, crate::error::Failure> {
    let group = RecoveryGroupDeclaration {
        group_id: GROUP_ID.to_string(),
        members: vec![
            RecoveryGroupMember {
                member_id: STATEFUL_SERVICE_ID.to_string(),
                restart_class: RestartClass::Permanent,
                after: Vec::new(),
            },
            RecoveryGroupMember {
                member_id: WORKER_SERVICE_ID.to_string(),
                restart_class: RestartClass::Permanent,
                after: vec![STATEFUL_SERVICE_ID.to_string()],
            },
        ],
        window_ticks: RECOVERY_WINDOW_TICKS,
        backoff_ticks: RECOVERY_BACKOFF_TICKS,
        max_restarts_per_member: MAX_RESTARTS_PER_MEMBER,
        max_restarts_per_group: MAX_RESTARTS_PER_GROUP,
        elapsed_source_ref: HASH_E.to_string(),
    };
    let stateful = stateful_facade().map_err(|issues| {
        crate::error::Failure::invalid_harness(format!("the stateful facade is not admitted: {issues:?}"))
    })?;
    let worker = service_facade(WORKER_SERVICE_ID).map_err(|issues| {
        crate::error::Failure::invalid_harness(format!("the worker facade is not admitted: {issues:?}"))
    })?;
    let sibling = service_facade(SIBLING_SERVICE_ID).map_err(|issues| {
        crate::error::Failure::invalid_harness(format!("the sibling facade is not admitted: {issues:?}"))
    })?;
    Ok(GroupDefinitions {
        group,
        stateful_facade: stateful,
        worker_facade: worker,
        sibling_facade: sibling,
        durable_profile_ref: HASH_A.to_string(),
        durable_descriptor_ref: HASH_B.to_string(),
    })
}
