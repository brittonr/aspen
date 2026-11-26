
#[derive(Debug, Clone, Copy)]
struct CommitIntentValueInput<'a> {
    request_ref: &'a str,
    before_state_ref: &'a str,
    unbound_after_state_ref: &'a str,
    token_claim: Option<&'a CoordinationTokenClaim>,
    time_sample: &'a CoordinationTimeSample,
}

// r[impl molten.coordination.truthful_tokens_assertions_and_expiry]
/// The control-plane commit target of a coordination mutation. It names no fencing-token ref: tokens bind
/// the commit receipt of this intent, so the committed state is derived from the intent and that receipt.
/// It records the clock sample the mutation was evaluated at, so replay is deterministic.
fn coordination_commit_intent_value(input: CommitIntentValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.request_ref, "coordination commit intent request ref")?;
    validate_ref(input.before_state_ref, "coordination commit intent before state ref")?;
    validate_ref(input.unbound_after_state_ref, "coordination commit intent unbound after state ref")?;
    validate_ref(&input.time_sample.time_basis_ref, "coordination commit intent time basis ref")?;
    let claims = input.token_claim.map_or_else(Vec::new, |claim| vec![token_claim_value(claim)]);
    Ok(record("coordination-commit-intent-v1", vec![
        string(COORDINATION_COMMIT_INTENT_SCHEMA),
        record("request", vec![string(input.request_ref)]),
        record("before-state", vec![string(input.before_state_ref)]),
        record("unbound-after-state", vec![string(input.unbound_after_state_ref)]),
        record("token-claims", vec![sequence(claims)]),
        record("time", vec![string(&input.time_sample.time_basis_ref), u64_value(input.time_sample.ticks)]),
        checks_value(&[
            ("commit-intent-bound", "pass"),
            ("token-bound-after-commit", "pass"),
            ("shell-clock-sampled", "pass"),
        ]),
    ]))
}

fn token_claim_value(claim: &CoordinationTokenClaim) -> IoValue {
    record("token-claim", vec![
        string(&claim.service),
        string(&claim.key),
        string(&claim.owner),
        u64_value(claim.token),
        u64_value(claim.lease_epoch),
        u64_value(claim.lease_expires_at),
    ])
}

/// Issues the claimed fencing token bound to `commit_receipt_ref` and inserts the lock or election entry
/// carrying exactly that token's `token_ref`.
fn bind_token_claim(
    state: &mut CoordinationState,
    claim: &CoordinationTokenClaim,
    commit_receipt_ref: &str,
) -> Result<FencingToken> {
    let value = fencing_token_value(&claim.key, &claim.owner, claim.token, claim.lease_epoch, commit_receipt_ref)?;
    let token = parse_fencing_token(&value)?;
    let displaced = match claim.service.as_str() {
        SERVICE_LOCK => state
            .locks
            .insert(claim.key.clone(), LockState {
                owner: claim.owner.clone(),
                token: claim.token,
                token_ref: token.token_ref.clone(),
                lease_epoch: claim.lease_epoch,
                lease_expires_at: claim.lease_expires_at,
            })
            .is_some(),
        SERVICE_ELECTION => state
            .elections
            .insert(claim.key.clone(), ElectionState {
                leader: claim.owner.clone(),
                token: claim.token,
                token_ref: token.token_ref.clone(),
                lease_expires_at: claim.lease_expires_at,
            })
            .is_some(),
        other => return Err(MoltenError::invalid_harness(format!("coordination service {other} issues no fencing token"))),
    };
    if displaced {
        return Err(MoltenError::invalid_harness(format!(
            "coordination {} token claim for {} would displace a current holder",
            claim.service, claim.key
        )));
    }
    Ok(token)
}

/// One reading of the coordination clock: ticks on a named time basis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationTimeSample {
    pub time_basis_ref: String,
    pub ticks: u64,
}

// r[impl molten.coordination.truthful_tokens_assertions_and_expiry]
/// The shell-owned coordination time port. Coordination never reads time from requests or derives it from
/// mutation counts: the runtime samples this clock once per admitted mutation. Production code obtains a
/// clock only from [`CoordinationClock::live`], the node monotonic clock. Deterministic clocks exist for
/// simulation runtimes ([`new_coordination_simulation_runtime`]) and, in test builds, as fixtures.
pub struct CoordinationClock {
    time_basis_ref: String,
    source: ClockSource,
}

enum ClockSource {
    Live(LiveClock),
    Deterministic(Box<dyn crate::fabric_time::TimerClockAdapter + Send>),
}

/// Sealed live clock: only [`CoordinationClock::live`] constructs one. Its ticks are nanoseconds since its
/// origin on the node monotonic source, so one reading of that source converts into every live basis. It
/// keeps the provenance its basis ref commits to: the admitted lease-clock profile, the wall instant its
/// origin was anchored at, and a process-local open sequence.
struct LiveClock {
    origin: std::time::Instant,
    profile_ref: String,
    anchor_unix_nanos: u64,
    open_sequence: u64,
}

impl LiveClock {
    fn basis_ref(&self) -> Result<String> {
        canonical_hash(&record("coordination-time-basis-v1", vec![
            string(&self.profile_ref),
            string("node-monotonic-nanos"),
            u64_value(self.anchor_unix_nanos),
            u64_value(self.open_sequence),
        ]))
    }

    /// This basis's ticks at one reading of the node monotonic source.
    fn ticks_at(&self, instant: std::time::Instant) -> crate::fabric::FabricPortResult<u64> {
        let elapsed = instant.checked_duration_since(self.origin).ok_or_else(|| {
            crate::fabric::FabricPortError::malformed("node monotonic reading precedes the coordination clock origin")
        })?;
        u64::try_from(elapsed.as_nanos())
            .map_err(|_| crate::fabric::FabricPortError::malformed("coordination clock ticks overflow u64 nanoseconds"))
    }
}

/// One reading of the node monotonic source, shared by every live coordination basis.
#[allow(
    tigerstyle::ambient_clock,
    reason = "the live coordination clock is the documented node monotonic capability; this is its single read of the host monotonic source"
)]
fn read_node_monotonic_instant() -> std::time::Instant {
    std::time::Instant::now()
}

static LIVE_COORDINATION_CLOCK_OPENS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl std::fmt::Debug for CoordinationClock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CoordinationClock")
            .field("time_basis_ref", &self.time_basis_ref)
            .field("live", &self.is_live())
            .finish_non_exhaustive()
    }
}

impl CoordinationClock {
    /// The node monotonic clock (nanosecond ticks since its own monotonic origin) under the admitted live
    /// lease-clock profile. Its basis ref binds that profile, the wall instant the origin was anchored at,
    /// and a process-local open sequence, so two live clocks never share a basis.
    pub fn live() -> Result<Self> {
        let profile = crate::fabric_time::admit_profile(&coordination_live_time_profile())?;
        let mut wall = crate::fabric_time::LiveClockAdapter::new(&profile.profile, 0)?;
        let anchor = wall.observe_wall()?;
        let open_sequence = LIVE_COORDINATION_CLOCK_OPENS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let live = LiveClock {
            origin: read_node_monotonic_instant(),
            profile_ref: profile.profile_ref,
            anchor_unix_nanos: anchor.unix_nanos,
            open_sequence,
        };
        Ok(Self {
            time_basis_ref: live.basis_ref()?,
            source: ClockSource::Live(live),
        })
    }

    /// Test-only: binds an arbitrary timer adapter to a basis.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new(
        time_basis_ref: impl Into<String>,
        adapter: Box<dyn crate::fabric_time::TimerClockAdapter + Send>,
    ) -> Result<Self> {
        let time_basis_ref = time_basis_ref.into();
        validate_ref(&time_basis_ref, "coordination time basis ref")?;
        Ok(Self {
            time_basis_ref,
            source: ClockSource::Deterministic(adapter),
        })
    }

    /// Test-only deterministic clock reading `ticks` on the fixture basis; tests move time by installing a
    /// fixture clock with later ticks.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture(ticks: u64) -> Self {
        Self::deterministic(coordination_fixture_time_basis_ref(), ticks)
    }

    /// Test-only deterministic clock reading `ticks` on an explicit basis.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_on_basis(time_basis_ref: impl Into<String>, ticks: u64) -> Result<Self> {
        let time_basis_ref = time_basis_ref.into();
        validate_ref(&time_basis_ref, "coordination time basis ref")?;
        Ok(Self::deterministic(time_basis_ref, ticks))
    }

    /// A deterministic clock frozen at `ticks`; outside tests only simulation runtimes hold one.
    fn deterministic(time_basis_ref: String, ticks: u64) -> Self {
        Self {
            source: ClockSource::Deterministic(Box::new(FixtureClockAdapter {
                profile_ref: time_basis_ref.clone(),
                ticks,
            })),
            time_basis_ref,
        }
    }

    pub fn time_basis_ref(&self) -> &str {
        &self.time_basis_ref
    }

    pub fn is_live(&self) -> bool {
        matches!(self.source, ClockSource::Live(_))
    }

    pub fn sample(&mut self) -> crate::fabric::FabricPortResult<CoordinationTimeSample> {
        let ticks = match &mut self.source {
            ClockSource::Live(live) => live.ticks_at(read_node_monotonic_instant())?,
            ClockSource::Deterministic(adapter) => adapter.now_ticks()?,
        };
        Ok(CoordinationTimeSample {
            time_basis_ref: self.time_basis_ref.clone(),
            ticks,
        })
    }

    /// Verifies that this is a sealed live clock whose basis ref is the one its provenance commits to: the
    /// admitted lease-clock profile and the node monotonic origin it counts from.
    fn verify_live_provenance(&self) -> Result<()> {
        let ClockSource::Live(live) = &self.source else {
            return Err(MoltenError::invalid_harness(
                "coordination accepts only a live node monotonic clock here; deterministic clocks belong to simulation runtimes",
            ));
        };
        let profile = crate::fabric_time::admit_profile(&coordination_live_time_profile())?;
        if live.profile_ref != profile.profile_ref || live.basis_ref()? != self.time_basis_ref {
            return Err(MoltenError::invalid_harness(
                "coordination live clock provenance does not match its time basis",
            ));
        }
        Ok(())
    }
}

/// Basis ref of deterministic simulation and fixture clocks.
pub fn coordination_fixture_time_basis_ref() -> String {
    content_ref_from_bytes(b"molten.coordination.fixture-clock.v1")
}

#[derive(Debug)]
struct FixtureClockAdapter {
    profile_ref: String,
    ticks: u64,
}

impl crate::fabric_time::TimerClockAdapter for FixtureClockAdapter {
    fn profile_ref(&self) -> &str {
        &self.profile_ref
    }

    fn timer_domain(&self) -> crate::fabric_time::TimeDomain {
        crate::fabric_time::TimeDomain::Virtual
    }

    fn now_ticks(&mut self) -> crate::fabric::FabricPortResult<u64> {
        Ok(self.ticks)
    }

    fn await_ticks(&mut self, target_ticks: u64) -> crate::fabric::FabricPortResult<u64> {
        if target_ticks <= self.ticks {
            return Ok(self.ticks);
        }
        Err(crate::fabric::FabricPortError::malformed(format!(
            "coordination fixture clock at {} does not advance to {target_ticks} by waiting",
            self.ticks
        )))
    }
}

const COORDINATION_LIVE_TIME_PROFILE_ID: &str = "molten.coordination.lease-clock";
/// The lease clock admits one monotonic reading at a time and no timers, runnables, or entropy of its own.
const COORDINATION_LIVE_TIME_UNIT_BOUND: u64 = 1;

fn coordination_live_time_profile() -> crate::fabric_time::TimeProfileDescriptor {
    use crate::fabric_time as time;
    time::TimeProfileDescriptor {
        schema: time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: COORDINATION_LIVE_TIME_PROFILE_ID.to_string(),
        profile_ref: content_ref_from_bytes(COORDINATION_LIVE_TIME_PROFILE_ID.as_bytes()),
        kind: time::TimeProfileKind::Live,
        supported_domains: time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: MAX_COORDINATION_LEASE_TICKS,
        max_uncertainty_ticks: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_timers: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_runnables: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_entropy_request_bytes: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_entropy_total_bytes: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_scheduler_concurrency: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_scheduler_queue_depth: COORDINATION_LIVE_TIME_UNIT_BOUND,
        fairness_bound_turns: None,
        scheduler_policy: time::SchedulerPolicy {
            ordering: time::SchedulerOrdering::PriorityThenFifo,
            replay: time::SchedulerReplayPolicy::RecordedChoiceRequired,
            overload: time::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: time::TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

/// Admits a clock sample for the next mutation of `state`: it must come from the committed time basis,
/// may not regress, and may not pass [`MAX_COORDINATION_TIME_TICKS`], so every lease expiry stays
/// representable.
fn admit_time_sample(state: &CoordinationState, sample: &CoordinationTimeSample) -> Result<u64> {
    if sample.time_basis_ref != state.time_basis_ref {
        return Err(MoltenError::invalid_harness(format!(
            "coordination clock basis {} does not match committed time basis {}",
            sample.time_basis_ref, state.time_basis_ref
        )));
    }
    if sample.ticks < state.time_ticks {
        return Err(MoltenError::invalid_harness(format!(
            "coordination clock sample {} regresses below committed time {}",
            sample.ticks, state.time_ticks
        )));
    }
    if sample.ticks > MAX_COORDINATION_TIME_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "coordination time exhausted at {}; no lease granted there would be representable",
            sample.ticks
        )));
    }
    Ok(sample.ticks)
}

/// Lease expiry for a lock acquire or election granted at clock reading `now`. Reads an optional
/// `<lease ttl>`; without one the lease lasts [`DEFAULT_COORDINATION_LEASE_TICKS`].
fn lease_expiry(payload: Option<&IoValue>, now: u64) -> Result<u64> {
    let ttl = match payload {
        Some(value) => {
            let fields = simple_record(value, "lease", 1)?;
            required_u64(&fields[0], "coordination lease ttl")?
        }
        None => DEFAULT_COORDINATION_LEASE_TICKS,
    };
    if ttl == 0 || ttl > MAX_COORDINATION_LEASE_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "coordination lease ttl {ttl} must be within 1..={MAX_COORDINATION_LEASE_TICKS} clock ticks"
        )));
    }
    now.checked_add(ttl)
        .ok_or_else(|| MoltenError::invalid_harness("coordination lease expiry overflows clock ticks"))
}

const COORDINATION_CLOCK_TRANSITION_SCHEMA: &str = "molten.coordination.clock-transition.v1";
const COORDINATION_CLOCK_NAMESPACE: &str = "coordination:clock";
const COORDINATION_CLOCK_REGISTRY_NAME: &str = "time-basis";
const COORDINATION_CLOCK_SESSION: &str = "coordination:clock-transition";

/// A committed change of coordination time basis (a clock epoch change).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationClockTransition {
    pub transition_ref: String,
    /// The retiring basis and its ticks at the shared reading (the last committed sample if the retiring clock
    /// no longer reads, as after a restart).
    pub from: CoordinationTimeSample,
    /// The new basis and its ticks at that same reading: the start of the new basis.
    pub to: CoordinationTimeSample,
    pub before_state_ref: String,
    pub after_state_ref: String,
    pub raft_commit_ref: String,
    pub value: IoValue,
    pub evidence_values: Vec<IoValue>,
}

// r[impl molten.coordination.truthful_tokens_assertions_and_expiry]
/// Node capability, not a request operation: the node shell recreates the runtime's live coordination clock
/// (for example after the monotonic clock source is reopened) and commits the basis change. No coordination
/// request, payload, or CLI command reaches it. The caller supplies no clock: the runtime opens a fresh
/// sealed live clock itself, so no frozen or arbitrary clock can be installed.
///
/// Leases are re-anchored with their remaining TTL, translated through one shared reading of the node monotonic
/// source: every check runs first, then a single instant is converted into both the retiring and the new
/// basis, and each lease's new expiry is `new_now + (old_expiry - old_now)`, saturating at zero. No time is
/// lost between the two readings, so an expired lease stays expired and repeated recreations, whatever
/// authority ref accompanies them, never extend a lease. Fencing state is preserved. The committed transition
/// record carries both readings, so replaying it reproduces the re-anchored state.
pub fn recreate_coordination_live_clock(
    runtime: &mut CoordinationRuntime,
    authority_refs: &[String],
) -> Result<CoordinationClockTransition> {
    transition_coordination_clock(runtime, CoordinationClock::live()?, authority_refs)
}

/// Installs an admitted live clock: it must be a sealed live clock whose basis ref matches its provenance,
/// and the runtime's current clock must be live. Simulation runtimes keep their deterministic clock.
fn transition_coordination_clock(
    runtime: &mut CoordinationRuntime,
    clock: CoordinationClock,
    authority_refs: &[String],
) -> Result<CoordinationClockTransition> {
    clock.verify_live_provenance()?;
    if !runtime.clock.is_live() {
        return Err(MoltenError::invalid_harness(
            "coordination simulation runtimes keep their deterministic clock; only live runtimes change clock basis",
        ));
    }
    commit_clock_transition(runtime, clock, authority_refs, live_shared_reading)
}

/// The retiring and the new basis's ticks at one reading of a shared instant source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SharedClockReading {
    retiring_ticks: u64,
    next_ticks: u64,
}

/// Converts one node monotonic reading into both live bases. If the retiring clock no longer reads on the
/// committed basis (as after a restart), the retiring side is credited only up to the last committed sample.
/// That conservative gap can keep a lease live for the unobserved time between that sample and the restart,
/// but it never expires a lease early.
fn live_shared_reading(
    retiring: &CoordinationClock,
    state: &CoordinationState,
    next: &CoordinationClock,
) -> Result<SharedClockReading> {
    let ClockSource::Live(next_live) = &next.source else {
        return Err(MoltenError::invalid_harness("coordination clock transitions translate only between live clocks"));
    };
    let instant = read_node_monotonic_instant();
    let next_ticks = next_live.ticks_at(instant).map_err(MoltenError::from)?;
    let retiring_ticks = match &retiring.source {
        ClockSource::Live(live) if retiring.time_basis_ref == state.time_basis_ref => {
            live.ticks_at(instant).map_err(MoltenError::from)?
        }
        _ => state.time_ticks,
    };
    Ok(SharedClockReading { retiring_ticks, next_ticks })
}

/// Commits a clock basis transition whose clock the caller has already admitted. All checks run before
/// `read_shared` takes the single reading that both bases are translated through.
fn commit_clock_transition(
    runtime: &mut CoordinationRuntime,
    clock: CoordinationClock,
    authority_refs: &[String],
    read_shared: impl FnOnce(&CoordinationClock, &CoordinationState, &CoordinationClock) -> Result<SharedClockReading>,
) -> Result<CoordinationClockTransition> {
    if clock.time_basis_ref() == runtime.state.time_basis_ref {
        return Err(MoltenError::invalid_harness(
            "coordination clock is already on the committed time basis; a clock transition needs a new basis",
        ));
    }
    if authority_refs.is_empty() {
        return Err(MoltenError::invalid_harness("coordination clock transition requires authority evidence"));
    }
    validate_refs(authority_refs, "coordination clock transition authority ref")?;
    let reading = read_shared(&runtime.clock, &runtime.state, &clock)?;
    if reading.retiring_ticks < runtime.state.time_ticks {
        return Err(MoltenError::invalid_harness(format!(
            "coordination retiring clock reading {} regresses below committed time {}",
            reading.retiring_ticks, runtime.state.time_ticks
        )));
    }
    let from = CoordinationTimeSample {
        time_basis_ref: runtime.state.time_basis_ref.clone(),
        ticks: reading.retiring_ticks,
    };
    let to = CoordinationTimeSample {
        time_basis_ref: clock.time_basis_ref().to_string(),
        ticks: reading.next_ticks,
    };
    let after_state = rebase_coordination_time(&runtime.state, &from, &to)?;
    let before_snapshot = snapshot_from_state(&runtime.state)?;
    let after_snapshot = snapshot_from_state(&after_state)?;
    let value = coordination_clock_transition_value(&from, &to, &before_snapshot.state_ref, &after_snapshot.state_ref)?;
    let transition_ref = canonical_hash(&value)?;
    let proposal = propose_clock_transition(runtime, &transition_ref, &before_snapshot.state_ref, authority_refs)?;
    if proposal.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "control-plane commit denied for coordination clock transition: {}",
            proposal.registry_receipt.diagnostics.join("; ")
        )));
    }
    runtime.next_sequence = runtime
        .next_sequence
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("coordination raft sequence overflow"))?;
    runtime.state = after_state;
    runtime.clock = clock;
    let mut evidence_values = vec![
        proposal.envelope.value.clone(),
        proposal.commit_receipt.value.clone(),
        proposal.registry_receipt.value.clone(),
    ];
    if let Some(log_entry) = &proposal.log_entry {
        evidence_values.push(log_entry.value.clone());
    }
    evidence_values.push(value.clone());
    evidence_values.push(after_snapshot.value.clone());
    Ok(CoordinationClockTransition {
        transition_ref,
        from,
        to,
        before_state_ref: before_snapshot.state_ref,
        after_state_ref: after_snapshot.state_ref,
        raft_commit_ref: proposal.commit_receipt.receipt_ref,
        value,
        evidence_values,
    })
}

fn propose_clock_transition(
    runtime: &mut CoordinationRuntime,
    transition_ref: &str,
    before_state_ref: &str,
    authority_refs: &[String],
) -> Result<Proposal> {
    let command = crate::raft_control_plane::control_registry_command_value(
        &crate::raft_control_plane::ControlRegistryCommandInput {
            operation: "set-coordination-state".to_string(),
            namespace: COORDINATION_CLOCK_NAMESPACE.to_string(),
            name: COORDINATION_CLOCK_REGISTRY_NAME.to_string(),
            target_ref: Some(transition_ref.to_string()),
        },
    )?;
    let envelope =
        crate::raft_control_plane::raft_command_envelope_value(&crate::raft_control_plane::RaftCommandEnvelopeInput {
            group_ref: runtime.raft.manifest.manifest_ref.clone(),
            client_session: COORDINATION_CLOCK_SESSION.to_string(),
            sequence: runtime.next_sequence,
            command,
            authority_refs: authority_refs.to_vec(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![before_state_ref.to_string(), transition_ref.to_string()],
        })?;
    crate::raft_control_plane::propose_control_registry_command(&mut runtime.raft, &envelope)
}

fn coordination_clock_transition_value(
    from: &CoordinationTimeSample,
    to: &CoordinationTimeSample,
    before_state_ref: &str,
    after_state_ref: &str,
) -> Result<IoValue> {
    validate_ref(&from.time_basis_ref, "coordination clock transition from basis ref")?;
    validate_ref(&to.time_basis_ref, "coordination clock transition to basis ref")?;
    validate_ref(before_state_ref, "coordination clock transition before state ref")?;
    validate_ref(after_state_ref, "coordination clock transition after state ref")?;
    Ok(record("coordination-clock-transition-v1", vec![
        string(COORDINATION_CLOCK_TRANSITION_SCHEMA),
        record("from", vec![string(&from.time_basis_ref), u64_value(from.ticks)]),
        record("to", vec![string(&to.time_basis_ref), u64_value(to.ticks)]),
        record("before-state", vec![string(before_state_ref)]),
        record("after-state", vec![string(after_state_ref)]),
        checks_value(&[
            ("leases-reanchored-remaining-ttl", "pass"),
            ("fencing-state-preserved", "pass"),
        ]),
    ]))
}

/// Pure core of a clock epoch change: moves committed time to the new basis start `to` and re-anchors each
/// lease to expire its remaining TTL after that start. The remaining TTL is the old expiry minus `from.ticks`,
/// the latest observation of the retiring basis, saturating at zero.
fn rebase_coordination_time(
    state: &CoordinationState,
    from: &CoordinationTimeSample,
    to: &CoordinationTimeSample,
) -> Result<CoordinationState> {
    validate_ref(&to.time_basis_ref, "coordination clock transition basis ref")?;
    if to.ticks > MAX_COORDINATION_TIME_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "coordination time exhausted at {}; no lease granted there would be representable",
            to.ticks
        )));
    }
    let mut next = state.clone();
    for lock in next.locks.values_mut() {
        lock.lease_expires_at = reanchored_expiry(lock.lease_expires_at, from.ticks, to.ticks)?;
    }
    for election in next.elections.values_mut() {
        election.lease_expires_at = reanchored_expiry(election.lease_expires_at, from.ticks, to.ticks)?;
    }
    next.time_basis_ref = to.time_basis_ref.clone();
    next.time_ticks = to.ticks;
    Ok(next)
}

fn reanchored_expiry(expires_at: u64, observed_old_ticks: u64, new_start: u64) -> Result<u64> {
    new_start
        .checked_add(expires_at.saturating_sub(observed_old_ticks))
        .ok_or_else(|| MoltenError::invalid_harness("coordination lease re-anchoring overflows clock ticks"))
}
