use super::*;
use crate::fabric_time::CryptographicEntropySource;

const ELECTION_STREAM_ID: &str = "raft-election";
const ELECTION_PURPOSE: &str = "raft-election-timeout";
const FIRST_TIMER_SEQUENCE: u64 = 0;
const NEXT_TIMER_SEQUENCE: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokioReplicaTimeConfig {
    pub profile: crate::fabric_time::AdmittedTimeProfile,
    pub generation: u64,
    pub service_id: String,
    pub capability_ref: String,
    pub entropy_binding_ref: String,
    pub tick_duration: std::time::Duration,
    pub heartbeat_ticks: u64,
    pub election_min_ticks: u64,
    pub election_max_ticks: u64,
}

pub struct TokioReplicaTimePort<S: CryptographicEntropySource> {
    profile: crate::fabric_time::AdmittedTimeProfile,
    generation: u64,
    service_id: String,
    tick_duration: std::time::Duration,
    heartbeat_ticks: u64,
    election_min_ticks: u64,
    election_max_ticks: u64,
    entropy_binding_ref: String,
    entropy: crate::fabric_time::ProductionEntropyAdapter<S>,
    entropy_stream: crate::fabric_time::EntropyStreamState,
    sender: tokio::sync::mpsc::Sender<ReplicaEvent>,
    next_timer_sequence: u64,
    election_handle: Option<tokio::task::JoinHandle<()>>,
    heartbeat_handle: Option<tokio::task::JoinHandle<()>>,
}

impl<S: CryptographicEntropySource> TokioReplicaTimePort<S> {
    pub fn new(
        config: TokioReplicaTimeConfig,
        source: S,
        sender: tokio::sync::mpsc::Sender<ReplicaEvent>,
    ) -> crate::error::Result<Self> {
        validate_time_configuration(&config)?;
        let entropy_stream = crate::fabric_time::open_entropy_stream(
            &config.profile,
            config.generation,
            &crate::fabric_time::EntropyStreamRequest {
                profile_ref: config.profile.profile_ref.clone(),
                stream_id: ELECTION_STREAM_ID.to_string(),
                purpose: ELECTION_PURPOSE.to_string(),
                capability_ref: config.capability_ref,
                generation: config.generation,
                mode: crate::fabric_time::EntropyMode::ProductionCryptographic,
                explicit_simulation_seed: None,
                explicit_simulation_seed_ref: None,
            },
        )
        .map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("live Raft entropy stream denied: {error:?}"))
        })?;
        Ok(Self {
            profile: config.profile,
            generation: config.generation,
            service_id: config.service_id,
            tick_duration: config.tick_duration,
            heartbeat_ticks: config.heartbeat_ticks,
            election_min_ticks: config.election_min_ticks,
            election_max_ticks: config.election_max_ticks,
            entropy_binding_ref: config.entropy_binding_ref,
            entropy: crate::fabric_time::ProductionEntropyAdapter::new(source),
            entropy_stream,
            sender,
            next_timer_sequence: FIRST_TIMER_SEQUENCE,
            election_handle: None,
            heartbeat_handle: None,
        })
    }

    pub fn timer_profile_ref(&self) -> &str {
        &self.profile.profile_ref
    }

    pub fn entropy_binding_ref(&self) -> &str {
        &self.entropy_binding_ref
    }

    pub const fn service_generation(&self) -> u64 {
        self.generation
    }

    pub fn cancel_all(&mut self) {
        abort(&mut self.election_handle);
        abort(&mut self.heartbeat_handle);
    }

    fn election_delay(&mut self) -> crate::error::Result<(u64, String)> {
        let span = self
            .election_max_ticks
            .checked_sub(self.election_min_ticks)
            .and_then(|difference| difference.checked_add(1))
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("live Raft election span overflow"))?;
        let (transition, metadata) = self.entropy.draw(
            &self.profile,
            self.generation,
            &self.entropy_stream,
            crate::fabric_time::EntropyRequest::BoundedChoice { upper_exclusive: span },
        )?;
        let crate::fabric_time::EntropyValue::Choice(choice) = transition.value else {
            return Err(crate::error::MoltenError::invalid_harness(
                "live Raft election entropy did not produce a bounded choice",
            ));
        };
        self.entropy_stream = transition.next;
        let delay = self
            .election_min_ticks
            .checked_add(choice)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("live Raft election delay overflow"))?;
        Ok((delay, crate::fabric_time::canonical_entropy_event(&metadata)?.evidence_ref))
    }

    fn timer_evidence(&mut self, delay_ticks: u64, ordering_key: u64) -> crate::error::Result<String> {
        let timer = crate::fabric_time::schedule_timer(
            &self.profile,
            self.generation,
            0,
            &crate::fabric_time::TimerScheduleRequest {
                profile_ref: self.profile.profile_ref.clone(),
                key: crate::fabric_time::TimerKey {
                    service_id: self.service_id.clone(),
                    generation: self.generation,
                    sequence: self.next_timer_sequence,
                },
                domain: crate::fabric_time::TimeDomain::Monotonic,
                deadline_ticks: delay_ticks,
                kind: crate::fabric_time::TimerKind::OneShot,
                ordering_key,
                coalescing: crate::fabric_time::TimerCoalescingPolicy::CoalesceLatest,
                lateness: crate::fabric_time::TimerLatenessPolicy::DeliverRegardless,
                overload: crate::fabric_time::TimerOverloadPolicy::RejectAndRetain,
                resource_charge: crate::fabric_time::TimerResourceCharge::single_slot(),
            },
        )
        .map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("live Raft timer plan denied: {error:?}"))
        })?;
        self.next_timer_sequence = self
            .next_timer_sequence
            .checked_add(NEXT_TIMER_SEQUENCE)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("live Raft timer sequence overflow"))?;
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-timer-plan-v1", vec![
            crate::preserves_rail::string(&self.profile.profile_ref),
            crate::preserves_rail::string(&timer.key.service_id),
            crate::preserves_rail::u64_value(timer.key.generation),
            crate::preserves_rail::u64_value(timer.key.sequence),
            crate::preserves_rail::u64_value(timer.next_deadline_ticks),
            crate::preserves_rail::u64_value(timer.ordering_key),
        ]))
    }

    fn duration(&self, ticks: u64) -> crate::error::Result<std::time::Duration> {
        let multiplier = u32::try_from(ticks)
            .map_err(|_| crate::error::MoltenError::invalid_harness("live Raft timer ticks exceed u32"))?;
        self.tick_duration
            .checked_mul(multiplier)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("live Raft timer duration overflow"))
    }
}

impl TokioReplicaTimePort<crate::fabric_time::OperatingSystemEntropySource> {
    pub fn new_operating_system(
        config: TokioReplicaTimeConfig,
        sender: tokio::sync::mpsc::Sender<ReplicaEvent>,
    ) -> crate::error::Result<Self> {
        Self::new(config, crate::fabric_time::OperatingSystemEntropySource, sender)
    }
}

impl<S: CryptographicEntropySource> ReplicaTimeEffects for TokioReplicaTimePort<S> {
    fn arm_election_timer(&mut self, timer_ref: &str) -> crate::error::Result<String> {
        crate::preserves_rail::validate_content_ref(timer_ref)?;
        let (delay_ticks, entropy_evidence_ref) = self.election_delay()?;
        let timer_evidence_ref = self.timer_evidence(delay_ticks, self.next_timer_sequence)?;
        let duration = self.duration(delay_ticks)?;
        abort(&mut self.election_handle);
        let sender = self.sender.clone();
        let event_timer_ref = timer_ref.to_string();
        self.election_handle = Some(tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            let _closed_event = sender
                .send(ReplicaEvent::ElectionTimeout {
                    timer_ref: event_timer_ref,
                })
                .await
                .err();
        }));
        combined_timer_ref(
            "election",
            &timer_evidence_ref,
            Some(timer_ref),
            &self.entropy_binding_ref,
            Some(&entropy_evidence_ref),
        )
    }

    fn arm_heartbeat_timer(&mut self) -> crate::error::Result<String> {
        let timer_evidence_ref = self.timer_evidence(self.heartbeat_ticks, self.next_timer_sequence)?;
        let duration = self.duration(self.heartbeat_ticks)?;
        abort(&mut self.heartbeat_handle);
        let sender = self.sender.clone();
        self.heartbeat_handle = Some(tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            let _closed_event = sender.send(ReplicaEvent::HeartbeatTimeout).await.err();
        }));
        combined_timer_ref("heartbeat", &timer_evidence_ref, None, &self.entropy_binding_ref, None)
    }
}

impl<S: CryptographicEntropySource> Drop for TokioReplicaTimePort<S> {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

fn validate_time_configuration(config: &TokioReplicaTimeConfig) -> crate::error::Result<()> {
    if config.profile.kind != crate::fabric_time::TimeProfileKind::Live {
        return Err(crate::error::MoltenError::invalid_harness("Tokio Raft time port requires a live time profile"));
    }
    crate::preserves_rail::validate_content_ref(&config.entropy_binding_ref)?;
    if config.generation == 0 || config.service_id.is_empty() || config.tick_duration.is_zero() {
        return Err(crate::error::MoltenError::invalid_harness(
            "Tokio Raft time port requires generation, service id, and positive tick duration",
        ));
    }
    if config.heartbeat_ticks == 0
        || config.election_min_ticks <= config.heartbeat_ticks
        || config.election_max_ticks < config.election_min_ticks
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "Tokio Raft time bounds require heartbeat < election minimum <= election maximum",
        ));
    }
    if config.election_max_ticks > config.profile.max_duration_ticks {
        return Err(crate::error::MoltenError::invalid_harness(
            "Tokio Raft election bound exceeds the admitted time profile",
        ));
    }
    Ok(())
}

fn abort(handle: &mut Option<tokio::task::JoinHandle<()>>) {
    if let Some(handle) = handle.take() {
        handle.abort();
    }
}

fn combined_timer_ref(
    kind: &str,
    time_plan_ref: &str,
    protocol_timer_ref: Option<&str>,
    entropy_binding_ref: &str,
    entropy_evidence_ref: Option<&str>,
) -> crate::error::Result<String> {
    let protocol_timer = optional_ref(protocol_timer_ref);
    let entropy_evidence = optional_ref(entropy_evidence_ref);
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-timer-arm-v1", vec![
        crate::preserves_rail::string(kind),
        crate::preserves_rail::string(time_plan_ref),
        protocol_timer,
        crate::preserves_rail::string(entropy_binding_ref),
        entropy_evidence,
    ]))
}

fn optional_ref(reference: Option<&str>) -> preserves::IOValue {
    reference.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}
