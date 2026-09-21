/// Explicit inputs and recorded events for the retry conformance fixture.
/// This record does not authenticate historical inputs or authorize a timer effect.
pub struct FixtureRetryInput<'a> {
    pub now: super::TimeValue,
    pub attempt: u64,
    pub policy: super::RetryPolicy,
    pub jitter: Option<u64>,
    pub recorded: &'a [super::CanonicalTimeEvent],
}

/// Recompute a fixture retry and reject divergent canonical observations.
// r[impl molten.audit_f12.validation]
pub fn replay_fixture_retry(
    profile: &super::CanonicalTimeProfile,
    input: FixtureRetryInput<'_>,
) -> crate::error::Result<()> {
    let actual = super::fixture::retry_events(profile, &input.now, input.attempt, input.policy, input.jitter)?;
    if input.recorded != actual {
        return Err(crate::error::MoltenError::invalid_harness(
            "retry replay diverged from recorded delay or deadline",
        ));
    }
    Ok(())
}
