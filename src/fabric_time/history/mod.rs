/// Explicit inputs and recorded events for the retry conformance fixture.
/// This record does not authenticate historical inputs or authorize a timer effect.
pub struct FixtureRetryInput<'a> {
    pub now: super::TimeValue,
    pub attempt: u64,
    pub policy: super::RetryPolicy,
    pub jitter: Option<u64>,
    pub recorded: &'a [super::Event],
}

/// Recompute a fixture retry and reject divergent canonical observations.
// r[impl molten.audit_f12.validation]
pub fn replay_fixture_retry(profile: &super::Profile, input: FixtureRetryInput<'_>) -> crate::error::Result<()> {
    let actual = super::fixture::retry_events(profile, &input.now, input.attempt, input.policy, input.jitter)?;
    if input.recorded != actual {
        return Err(crate::error::Failure::invalid_harness("retry replay diverged from recorded delay or deadline"));
    }
    Ok(())
}

/// Explicit inputs and recorded events for a deterministic entropy stream: the admitted stream
/// request, the draws made from it in order, and the entropy events recorded for those draws.
pub struct FixtureEntropyInput<'a> {
    pub stream: super::EntropyStreamRequest,
    pub draws: &'a [super::EntropyRequest],
    pub recorded: &'a [super::Event],
}

/// Why recorded entropy evidence failed replay.
#[derive(Debug)]
pub enum EntropyReplayError {
    /// A recorded event is not decodable capability-bound entropy evidence.
    Evidence(crate::error::Failure),
    /// The entropy law rejected the replay; a recorded capability reference that differs from the
    /// admitted stream's is `EntropyError::CapabilityMismatch`.
    Entropy(super::EntropyError),
    /// The recorded event at `index` (or the event count) differs from the recomputed evidence.
    Diverged { index: usize },
}

impl From<crate::error::Failure> for EntropyReplayError {
    fn from(failure: crate::error::Failure) -> Self {
        Self::Evidence(failure)
    }
}

/// Replay recorded deterministic entropy evidence against its admitted stream. Each recorded event
/// must carry the admitted stream's capability reference before its draw is recomputed and
/// compared.
// r[impl molten.fabric_time.entropy_capability_evidence]
pub fn replay_fixture_entropy(
    profile: &super::Profile,
    input: FixtureEntropyInput<'_>,
) -> Result<(), EntropyReplayError> {
    let generation = input.stream.generation;
    let mut stream =
        super::open_entropy_stream(&profile.profile, generation, &input.stream).map_err(EntropyReplayError::Entropy)?;
    for (index, recorded) in input.recorded.iter().enumerate() {
        let capability_ref = super::entropy_event_capability_ref(recorded)?;
        super::verify_entropy_evidence_capability(&stream, &capability_ref).map_err(EntropyReplayError::Entropy)?;
        let draw = *input.draws.get(index).ok_or(EntropyReplayError::Diverged { index })?;
        let transition = super::draw_deterministic_entropy(&profile.profile, generation, &stream, draw)
            .map_err(EntropyReplayError::Entropy)?;
        if super::entropy_event(&super::entropy_evidence_metadata(&stream, &transition))? != *recorded {
            return Err(EntropyReplayError::Diverged { index });
        }
        stream = transition.next;
    }
    if input.draws.len() != input.recorded.len() {
        return Err(EntropyReplayError::Diverged {
            index: input.recorded.len(),
        });
    }
    Ok(())
}
