
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
    let is_displaced = match claim.service.as_str() {
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
    if is_displaced {
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
