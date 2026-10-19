
impl VirtualClockAdapter {
    pub fn new(
        profile: &super::AdmittedTimeProfile,
        initial_virtual_ticks: u64,
        wall_base_nanos: u64,
    ) -> crate::error::Result<Self> {
        if profile.kind != super::TimeProfileKind::DeterministicSimulation {
            return Err(crate::error::Failure::invalid_harness(
                "virtual clock requires an admitted deterministic simulation profile",
            ));
        }
        Ok(Self {
            profile_ref: profile.profile_ref.clone(),
            virtual_ticks: initial_virtual_ticks,
            logical_position: 0,
            wall_base_nanos,
            wall_offset_nanos: 0,
            observation_sequence: 0,
            wall_uncertainty_nanos: 0,
        })
    }

    // r[impl molten.fabric_time.live_sim_parity]
    pub fn advance(&mut self, delta_ticks: u64) -> crate::error::Result<super::VirtualInstant> {
        self.virtual_ticks = self
            .virtual_ticks
            .checked_add(delta_ticks)
            .ok_or_else(|| crate::error::Failure::invalid_harness("virtual time overflow"))?;
        Ok(self.observe_virtual())
    }

    pub fn advance_logical(&mut self) -> crate::error::Result<u64> {
        self.logical_position = self
            .logical_position
            .checked_add(1)
            .ok_or_else(|| crate::error::Failure::invalid_harness("logical time overflow"))?;
        Ok(self.logical_position)
    }

    pub fn observe_virtual(&self) -> super::VirtualInstant {
        super::VirtualInstant {
            profile_ref: self.profile_ref.clone(),
            ticks: self.virtual_ticks,
        }
    }

    pub fn observe_wall(&mut self) -> crate::error::Result<super::WallClockObservation> {
        self.observation_sequence = self
            .observation_sequence
            .checked_add(1)
            .ok_or_else(|| crate::error::Failure::invalid_harness("virtual wall observation sequence overflow"))?;
        let base = i128::from(self.wall_base_nanos)
            .checked_add(i128::from(self.virtual_ticks))
            .and_then(|value| value.checked_add(self.wall_offset_nanos))
            .ok_or_else(|| crate::error::Failure::invalid_harness("virtual wall clock overflow"))?;
        let unix_nanos = u64::try_from(base)
            .map_err(|_| crate::error::Failure::invalid_harness("virtual wall clock underflow"))?;
        Ok(super::WallClockObservation {
            profile_ref: self.profile_ref.clone(),
            unix_nanos,
            uncertainty_nanos: self.wall_uncertainty_nanos,
            observation_sequence: self.observation_sequence,
        })
    }

    pub fn inject_wall_jump(&mut self, signed_delta_nanos: i128) -> crate::error::Result<()> {
        self.wall_offset_nanos = self
            .wall_offset_nanos
            .checked_add(signed_delta_nanos)
            .ok_or_else(|| crate::error::Failure::invalid_harness("virtual wall fault offset overflow"))?;
        Ok(())
    }

    pub fn set_wall_uncertainty(&mut self, uncertainty_nanos: u64) {
        self.wall_uncertainty_nanos = uncertainty_nanos;
    }
}

impl TimerClockAdapter for VirtualClockAdapter {
    fn profile_ref(&self) -> &str {
        &self.profile_ref
    }

    fn timer_domain(&self) -> super::TimeDomain {
        super::TimeDomain::Virtual
    }

    fn now_ticks(&mut self) -> FabricPortResult<u64> {
        Ok(self.virtual_ticks)
    }

    fn await_ticks(&mut self, target_ticks: u64) -> FabricPortResult<u64> {
        if target_ticks < self.virtual_ticks {
            return Err(FabricPortError::malformed(format!(
                "virtual adapter cannot move backwards from {} to {target_ticks}",
                self.virtual_ticks
            )));
        }
        self.virtual_ticks = target_ticks;
        Ok(self.virtual_ticks)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterConformanceObservation {
    pub domain: super::TimeDomain,
    pub timer_action: super::TimerAction,
    pub delivery_count: u64,
    pub stale_generation_discarded: bool,
    pub cancellation_prevented_delivery: bool,
    pub scheduler_selected: bool,
    pub scheduler_cancellation_recorded: bool,
    pub entropy_bound_rejected: bool,
}

// r[impl molten.modularity.fabric_boundary.adapters.clock]
// r[impl molten.fabric_time.live_sim_parity]
pub fn run_timer_adapter_conformance<A: TimerClockAdapter>(
    profile: &super::AdmittedTimeProfile,
    adapter: &mut A,
    service_id: &str,
    generation: u64,
) -> crate::error::Result<AdapterConformanceObservation> {
    if adapter.profile_ref() != profile.profile_ref {
        return Err(crate::error::Failure::invalid_harness("timer adapter profile mismatch"));
    }
    let start = adapter.now_ticks()?;
    let delay = match profile.kind {
        super::TimeProfileKind::Live => LIVE_CONFORMANCE_DELAY_NANOS,
        super::TimeProfileKind::DeterministicSimulation => 1,
    };
    let deadline = start
        .checked_add(delay)
        .ok_or_else(|| crate::error::Failure::invalid_harness("conformance deadline overflow"))?;
    let request = conformance_timer_request(TimerRequestInput {
        profile,
        service_id,
        generation,
        domain: adapter.timer_domain(),
        deadline_ticks: deadline,
        sequence: 0,
    });
    let timer = super::schedule_timer(profile, generation, 0, &request)
        .map_err(|error| core_error("schedule conformance timer", error))?;
    let observed = adapter.await_ticks(deadline)?;
    let fired = super::poll_timer(&timer, generation, observed, 1)
        .map_err(|error| core_error("poll conformance timer", error))?;

    let stale_request = conformance_timer_request(TimerRequestInput {
        profile,
        service_id,
        generation,
        domain: adapter.timer_domain(),
        deadline_ticks: deadline,
        sequence: 1,
    });
    let stale_timer = super::schedule_timer(profile, generation, 0, &stale_request)
        .map_err(|error| core_error("schedule stale probe", error))?;
    let stale = super::poll_timer(&stale_timer, generation.saturating_add(1), observed, 1)
        .map_err(|error| core_error("poll stale probe", error))?;

    let cancel_request = conformance_timer_request(TimerRequestInput {
        profile,
        service_id,
        generation,
        domain: adapter.timer_domain(),
        deadline_ticks: deadline,
        sequence: CANCELLATION_TIMER_SEQUENCE,
    });
    let cancel_timer_state = super::schedule_timer(profile, generation, 0, &cancel_request)
        .map_err(|error| core_error("schedule cancellation probe", error))?;
    let cancelled =
        super::cancel_timer(&cancel_timer_state, generation).map_err(|error| core_error("cancel probe", error))?;
    let is_cancellation_prevented_delivery = matches!(
        super::poll_timer(&cancelled.next, generation, observed, 1),
        Err(super::TimerError::TerminalTimer(_))
    );

    let (scheduler_selected, scheduler_cancellation_recorded) =
        run_scheduler_conformance(profile, service_id, generation)?;
    let is_entropy_bound_rejected = run_entropy_conformance(profile, generation)?;

    Ok(AdapterConformanceObservation {
        domain: adapter.timer_domain(),
        timer_action: fired.action,
        delivery_count: fired.delivery_count,
        stale_generation_discarded: stale.action == super::TimerAction::DiscardedStaleGeneration,
        cancellation_prevented_delivery: is_cancellation_prevented_delivery,
        scheduler_selected,
        scheduler_cancellation_recorded,
        entropy_bound_rejected: is_entropy_bound_rejected,
    })
}

