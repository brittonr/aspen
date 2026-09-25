
fn run_entropy_conformance(profile: &super::AdmittedTimeProfile, generation: u64) -> crate::error::Result<bool> {
    let (mode, seed) = match profile.kind {
        super::TimeProfileKind::Live => (super::EntropyMode::ProductionCryptographic, None),
        super::TimeProfileKind::DeterministicSimulation => (super::EntropyMode::DeterministicSimulation, Some(1)),
    };
    let stream = super::open_entropy_stream(profile, generation, &super::EntropyStreamRequest {
        profile_ref: profile.profile_ref.clone(),
        stream_id: "adapter-conformance-stream".to_string(),
        purpose: "adapter-conformance-bound".to_string(),
        capability_ref: CONFORMANCE_CAPABILITY_REF.to_string(),
        generation,
        mode,
        explicit_simulation_seed: seed,
        explicit_simulation_seed_ref: seed.map(|_| CONFORMANCE_CAPABILITY_REF.to_string()),
    })
    .map_err(|error| core_error("open conformance entropy stream", error))?;
    let over_limit = profile
        .max_entropy_request_bytes
        .checked_add(1)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("conformance entropy bound overflow"))?;
    let request = super::EntropyRequest::Bytes { count: over_limit };
    let is_rejected = match mode {
        super::EntropyMode::DeterministicSimulation => matches!(
            super::draw_deterministic_entropy(profile, generation, &stream, request),
            Err(super::EntropyError::RequestLimitExceeded { .. })
        ),
        super::EntropyMode::ProductionCryptographic => matches!(
            super::consume_production_entropy(profile, generation, &stream, request, Vec::new()),
            Err(super::EntropyError::RequestLimitExceeded { .. })
        ),
    };
    Ok(is_rejected)
}

#[derive(Debug, Default)]
pub struct OperatingSystemEntropySource;

impl CryptographicEntropySource for OperatingSystemEntropySource {
    fn source_id(&self) -> &'static str {
        "unix-dev-urandom"
    }

    fn fill_secret(&mut self, output: &mut [u8]) -> FabricPortResult<()> {
        #[cfg(unix)]
        {
            let mut source = std::fs::File::open(OS_ENTROPY_PATH)
                .map_err(|error| FabricPortError::capability(format!("open production entropy source: {error}")))?;
            source
                .read_exact(output)
                .map_err(|error| FabricPortError::capability(format!("read production entropy source: {error}")))
        }
        #[cfg(not(unix))]
        {
            let _ = output;
            Err(FabricPortError::capability("production entropy adapter has no admitted source on this platform"))
        }
    }
}

pub struct ProductionEntropyAdapter<S: CryptographicEntropySource> {
    source: S,
}

impl<S: CryptographicEntropySource> ProductionEntropyAdapter<S> {
    pub const fn new(source: S) -> Self {
        Self { source }
    }

    pub fn source_id(&self) -> &'static str {
        self.source.source_id()
    }

    pub fn draw(
        &mut self,
        profile: &super::AdmittedTimeProfile,
        active_generation: u64,
        state: &super::EntropyStreamState,
        request: super::EntropyRequest,
    ) -> crate::error::Result<(super::EntropyTransition, super::EntropyEvidenceMetadata)> {
        let output_len = usize::try_from(request.requested_bytes())
            .map_err(|_| crate::error::MoltenError::invalid_harness("entropy request length overflow"))?;
        let mut secret = vec![0; output_len];
        self.source.fill_secret(&mut secret)?;
        let transition = super::consume_production_entropy(profile, active_generation, state, request, secret)
            .map_err(|error| core_error("consume production entropy", error))?;
        let metadata = super::entropy_evidence_metadata(state, &transition);
        Ok((transition, metadata))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FabricTimeFault {
    BackwardWallJump { ticks: u64 },
    ForwardWallJump { ticks: u64 },
    DelayTimer { key: super::TimerKey, ticks: u64 },
    DropTimerDelivery { key: super::TimerKey },
    SaturateSchedulerQueue,
    CancelTimer { key: super::TimerKey },
    PartitionWindow { until_ticks: u64 },
}

pub fn apply_clock_fault(clock: &mut VirtualClockAdapter, fault: &FabricTimeFault) -> crate::error::Result<bool> {
    match fault {
        FabricTimeFault::BackwardWallJump { ticks } => {
            clock.inject_wall_jump(-i128::from(*ticks))?;
            Ok(true)
        }
        FabricTimeFault::ForwardWallJump { ticks } => {
            clock.inject_wall_jump(i128::from(*ticks))?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub fn validate_scheduler_fault_outcome(
    fault: &FabricTimeFault,
    transition: &super::SchedulerTransition,
) -> crate::error::Result<()> {
    if matches!(fault, FabricTimeFault::SaturateSchedulerQueue)
        && !matches!(transition.action, super::SchedulerAction::RejectedOverload | super::SchedulerAction::Backpressure)
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "scheduler saturation fault did not produce an explicit overload outcome",
        ));
    }
    Ok(())
}

pub fn poll_timer_with_fault(
    state: &super::TimerState,
    active_generation: u64,
    now_ticks: u64,
    delivery_capacity: u64,
    fault: Option<&FabricTimeFault>,
) -> crate::error::Result<super::TimerTransition> {
    let mut faulted = state.clone();
    let mut capacity = delivery_capacity;
    if let Some(fault) = fault {
        match fault {
            FabricTimeFault::DelayTimer { key, ticks } if key == &state.key => {
                faulted.next_deadline_ticks = faulted
                    .next_deadline_ticks
                    .checked_add(*ticks)
                    .ok_or_else(|| crate::error::MoltenError::invalid_harness("faulted timer deadline overflow"))?;
            }
            FabricTimeFault::DropTimerDelivery { key } if key == &state.key => {
                faulted.overload = super::TimerOverloadPolicy::DropDue;
                capacity = 0;
            }
            FabricTimeFault::CancelTimer { key } if key == &state.key => {
                return super::cancel_timer(&faulted, active_generation)
                    .map_err(|error| core_error("cancel faulted timer", error));
            }
            _ => {}
        }
    }
    super::poll_timer(&faulted, active_generation, now_ticks, capacity)
        .map_err(|error| core_error("poll faulted timer", error))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultedDeadlineDecision {
    Evaluated(super::DeadlineDecision),
    PartitionIndeterminate { until_ticks: u64, observed_ticks: u64 },
}

pub fn evaluate_deadline_with_fault(
    profile: &super::AdmittedTimeProfile,
    active_generation: u64,
    deadline: &super::Deadline,
    observed: &super::TimeValue,
    fault: Option<&FabricTimeFault>,
) -> crate::error::Result<FaultedDeadlineDecision> {
    if let Some(FabricTimeFault::PartitionWindow { until_ticks }) = fault
        && observed.ticks() <= *until_ticks
    {
        return Ok(FaultedDeadlineDecision::PartitionIndeterminate {
            until_ticks: *until_ticks,
            observed_ticks: observed.ticks(),
        });
    }
    super::evaluate_deadline(profile, active_generation, deadline, observed)
        .map(FaultedDeadlineDecision::Evaluated)
        .map_err(|error| core_error("evaluate faulted deadline", error))
}

struct TimerRequestInput<'a> {
    profile: &'a super::AdmittedTimeProfile,
    service_id: &'a str,
    generation: u64,
    domain: super::TimeDomain,
    deadline_ticks: u64,
    sequence: u64,
}

fn conformance_timer_request(input: TimerRequestInput<'_>) -> super::TimerScheduleRequest {
    let TimerRequestInput {
        profile,
        service_id,
        generation,
        domain,
        deadline_ticks,
        sequence,
    } = input;
    super::TimerScheduleRequest {
        profile_ref: profile.profile_ref.clone(),
        key: super::TimerKey {
            service_id: service_id.to_string(),
            generation,
            sequence,
        },
        domain,
        deadline_ticks,
        kind: super::TimerKind::OneShot,
        ordering_key: sequence,
        coalescing: super::TimerCoalescingPolicy::CoalesceLatest,
        lateness: super::TimerLatenessPolicy::DeliverRegardless,
        overload: super::TimerOverloadPolicy::RejectAndRetain,
        resource_charge: super::TimerResourceCharge::single_slot(),
    }
}

fn duration_to_u64_nanos(duration: std::time::Duration) -> crate::error::Result<u64> {
    let seconds = duration
        .as_secs()
        .checked_mul(NANOS_PER_SECOND)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("duration seconds overflow"))?;
    seconds
        .checked_add(u64::from(duration.subsec_nanos()))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("duration nanoseconds overflow"))
}

fn core_error(label: &str, error: impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label}: {error:?}"))
}
