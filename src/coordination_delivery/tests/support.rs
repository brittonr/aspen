use molten_core::coordination_delivery::*;
use molten_core::fabric_time::*;

use super::super::*;

pub(super) const SERVICE_GENERATION: u64 = 7;
pub(super) const CONSISTENCY_EPOCH: u64 = 11;
pub(super) const ENGINE_EPOCH: u64 = 13;
pub(super) const INITIAL_TICK: u64 = 100;
pub(super) const VISIBILITY_TICKS: u64 = 10;
pub(super) const RETRY_TICKS: u64 = 5;
pub(super) const RETENTION_TICKS: u64 = 20;
pub(super) const MAX_ATTEMPTS: u64 = 2;
pub(super) const CAPACITY: u32 = 8;
pub(super) const METADATA_BYTES: u32 = 32;
pub(super) const QUEUE_ID: &str = "queue:delivery";
pub(super) const ACTOR_ID: &str = "consumer-a";
pub(super) const BLAKE3_HEX_LENGTH: usize = 64;

pub(super) fn reference(hex: char) -> String {
    format!("blake3:{}", hex.to_string().repeat(BLAKE3_HEX_LENGTH))
}

pub(super) fn policy() -> DeliveryPolicy {
    DeliveryPolicy {
        schema: DELIVERY_POLICY_SCHEMA.to_string(),
        policy_id: "delivery-policy-v1".to_string(),
        visibility_timeout_ticks: VISIBILITY_TICKS,
        maximum_attempts: MAX_ATTEMPTS,
        retry_base_delay_ticks: RETRY_TICKS,
        retry_maximum_delay_ticks: RETRY_TICKS,
        retry_backoff: DeliveryBackoff::Fixed,
        ordering: DeliveryOrdering::StrictFifo,
        dead_letter_queue_id: "queue:delivery-dlq".to_string(),
        dead_letter_retention_ticks: RETENTION_TICKS,
        ready_capacity: CAPACITY,
        in_flight_capacity: CAPACITY,
        retry_capacity: CAPACITY,
        dead_letter_capacity: CAPACITY,
        metadata_byte_limit: METADATA_BYTES,
        status_item_limit: CAPACITY,
        completion_authority_ref: reference('a'),
        expiry_authority_ref: reference('b'),
        redrive_authority_ref: reference('c'),
        retention_authority_ref: reference('d'),
        retryable_failure_classes: ["transient".to_string()].into_iter().collect(),
        poison_failure_classes: ["poison".to_string()].into_iter().collect(),
        poison_item_handling: PoisonItemHandling::DeadLetter,
        non_claims: required_delivery_non_claims(),
    }
}

pub(super) fn manifest(policy: &DeliveryPolicy) -> DeliveryManifest {
    DeliveryManifest {
        schema: DELIVERY_MANIFEST_SCHEMA.to_string(),
        extension_id: "coordination-delivery".to_string(),
        service_id: "coordination-delivery-local".to_string(),
        service_generation: SERVICE_GENERATION,
        implementation_ref: reference('e'),
        time_profile_ref: reference('f'),
        policy_ref: identify_delivery_policy(policy),
        port_bindings: REQUIRED_DELIVERY_PORTS
            .into_iter()
            .enumerate()
            .map(|(index, port)| {
                let digits = ['1', '2', '3', '4', '5'];
                (port.to_string(), reference(digits[index]))
            })
            .collect(),
        non_claims: required_delivery_non_claims(),
    }
}

pub(super) fn host_binding(manifest: &DeliveryManifest) -> DeliveryHostBindingFacts {
    DeliveryHostBindingFacts {
        schema: DELIVERY_HOST_BINDING_SCHEMA.to_string(),
        system_extension_manifest_ref: reference('9'),
        extension_id: manifest.extension_id.clone(),
        service_id: manifest.service_id.clone(),
        service_generation: manifest.service_generation,
        lifecycle_running: true,
        port_bindings: manifest.port_bindings.clone(),
    }
}

pub(super) fn time_profile(manifest: &DeliveryManifest) -> AdmittedTimeProfile {
    AdmittedTimeProfile {
        profile_id: "delivery-logical-time".to_string(),
        profile_ref: manifest.time_profile_ref.clone(),
        kind: TimeProfileKind::DeterministicSimulation,
        supported_domains: vec![TimeDomain::Logical],
        max_duration_ticks: MAX_DELIVERY_TICKS,
        max_uncertainty_ticks: 0,
        max_timers: u64::from(CAPACITY),
        max_runnables: u64::from(CAPACITY),
        max_entropy_request_bytes: 1,
        max_entropy_total_bytes: 1,
        max_scheduler_concurrency: u64::from(CAPACITY),
        max_scheduler_queue_depth: u64::from(CAPACITY),
        fairness_bound_turns: Some(MAX_DELIVERY_TICKS),
        scheduler_policy: SchedulerPolicy {
            ordering: SchedulerOrdering::Fifo,
            replay: SchedulerReplayPolicy::Deterministic,
            overload: SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

pub(super) fn request(
    manifest: &DeliveryManifest,
    operation_id: char,
    logical_tick: u64,
    operation: DeliveryOperation,
) -> DeliveryRequest {
    DeliveryRequest {
        schema: DELIVERY_REQUEST_SCHEMA.to_string(),
        queue_id: QUEUE_ID.to_string(),
        operation_id: reference(operation_id),
        actor_id: ACTOR_ID.to_string(),
        service_generation: SERVICE_GENERATION,
        consistency_epoch: CONSISTENCY_EPOCH,
        engine_epoch: ENGINE_EPOCH,
        time_profile_ref: manifest.time_profile_ref.clone(),
        logical_tick,
        currentness: DeliveryCurrentness::Linearizable,
        authority_refs: vec![reference('6')],
        policy_refs: vec![manifest.policy_ref.clone()],
        resource_refs: vec![reference('7')],
        evidence_refs: vec![reference('8')],
        operation,
    }
}

pub(super) fn enqueue_request(manifest: &DeliveryManifest, operation_id: char) -> DeliveryRequest {
    request(manifest, operation_id, INITIAL_TICK, DeliveryOperation::Enqueue {
        item_ref: reference('a'),
        content_ref: reference('9'),
        metadata_ref: reference('0'),
        metadata_bytes: METADATA_BYTES,
    })
}

pub(super) fn empty_expected() -> ExpectedDeliveryState {
    ExpectedDeliveryState {
        state_ref: None,
        revision: INITIAL_DELIVERY_REVISION,
    }
}

pub(super) fn expected(published: &PublishedDeliveryState) -> ExpectedDeliveryState {
    ExpectedDeliveryState {
        state_ref: Some(published.state_ref.clone()),
        revision: published.revision,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CommitMode {
    Apply,
    UnknownBefore,
    UnknownAfter,
    Stale,
}

pub(super) struct MemoryCommitPort {
    pub(super) head: Option<PublishedDeliveryState>,
    pub(super) mode: CommitMode,
    pub(super) compare_calls: u32,
}

impl MemoryCommitPort {
    pub(super) const fn new(mode: CommitMode) -> Self {
        Self {
            head: None,
            mode,
            compare_calls: 0,
        }
    }
}

impl DeliveryCommitPort for MemoryCommitPort {
    fn load(&self, _queue_id: &str) -> DeliveryPortResult<Option<PublishedDeliveryState>> {
        Ok(self.head.clone())
    }

    fn compare_and_commit(&mut self, request: &DeliveryCommitRequest) -> DeliveryPortResult<DeliveryCommitObservation> {
        self.compare_calls += 1;
        match self.mode {
            CommitMode::Apply => {
                self.head = Some(request.next.clone());
                Ok(commit_observation(DeliveryCommitDisposition::Applied, Some(request.next.state_ref.clone())))
            }
            CommitMode::UnknownBefore => Err(DeliveryPortError::new("scripted-unknown", "unknown before apply", true)),
            CommitMode::UnknownAfter => {
                self.head = Some(request.next.clone());
                Err(DeliveryPortError::new("scripted-unknown", "unknown after apply", true))
            }
            CommitMode::Stale => Ok(commit_observation(
                DeliveryCommitDisposition::Stale,
                self.head.as_ref().map(|head| head.state_ref.clone()),
            )),
        }
    }
}

fn commit_observation(disposition: DeliveryCommitDisposition, state_ref: Option<String>) -> DeliveryCommitObservation {
    DeliveryCommitObservation {
        disposition,
        currentness: DeliveryCurrentness::Linearizable,
        durability: DeliveryDurabilityOutcome::Durable,
        engine_epoch: ENGINE_EPOCH,
        observed_state_ref: state_ref,
    }
}

pub(super) struct MemoryTimerPort {
    pub(super) fail: bool,
    pub(super) observed: Vec<String>,
}

impl DeliveryTimerPort for MemoryTimerPort {
    fn apply_timer_intents(&mut self, intents: &[DeliveryTimerIntent]) -> DeliveryPortResult<DeliveryTimerObservation> {
        let refs = intents.iter().map(|intent| intent.timer_id.clone()).collect::<Vec<_>>();
        if self.fail {
            return Err(DeliveryPortError::new("scripted-timer-failure", "timer scheduling failed", false));
        }
        self.observed.extend(refs.clone());
        Ok(DeliveryTimerObservation {
            accepted_timer_refs: refs,
            failed_timer_refs: Vec::new(),
            outcome_unknown: false,
        })
    }
}

#[derive(Default)]
pub(super) struct MemoryStatusPort {
    pub(super) status_refs: Vec<String>,
}

impl DeliveryStatusPort for MemoryStatusPort {
    fn publish_status(&mut self, status: &DeliveryStatus) -> DeliveryPortResult<DeliveryStatusObservation> {
        let status_ref = identify_canonical_delivery_status(status)
            .map_err(|error| DeliveryPortError::new("status", error.to_string(), false))?;
        self.status_refs.push(status_ref.clone());
        Ok(DeliveryStatusObservation {
            published_status_ref: Some(status_ref),
            outcome_unknown: false,
        })
    }
}

pub(super) fn timer_port(fail: bool) -> MemoryTimerPort {
    MemoryTimerPort {
        fail,
        observed: Vec::new(),
    }
}
