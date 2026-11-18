//! Negative cases run against the real completed transcript, never fabricated successful evidence.
pub fn verify(recorded: &super::transcript::Call) -> super::Result<Vec<&'static str>> {
    let mut reordered = recorded.clone();
    reordered.events.swap(0, 1);
    let mut duplicate = recorded.clone();
    duplicate.events.push(duplicate.events[2].clone());
    let mut cross_call = recorded.scope.clone();
    cross_call.dedup = super::reference(b"another-invocation");
    let mut altered_wire = recorded.clone();
    altered_wire.result_wire = super::bridge::encode_result("different result")?;
    let mut uncertain = recorded.clone();
    uncertain.response.delivery = crate::fabric_transport::DeliveryOutcome::Uncertain;
    let mut resources = recorded.scope.clone();
    resources.resources = super::reference(b"larger-resource-budget");
    let mut wrong_peer = recorded.scope.clone();
    wrong_peer.peer = super::reference(b"unauthorized-peer");
    let mut live_timing = recorded.clone();
    live_timing.events[1] = super::reference(b"live-wall-clock-effect");
    let mut diagnostics = super::diagnostics::Queue::default();
    let secret = rejected(
        diagnostics.observe("credential/private-payload", super::diagnostics::Status::Returned),
        "telemetry-secret-field",
    )?;
    for _ in 0..32 {
        diagnostics.observe(&recorded.reference, super::diagnostics::Status::Returned)?;
    }
    let overflow = rejected(
        diagnostics.observe(&recorded.reference, super::diagnostics::Status::Returned),
        "telemetry-queue-overflow",
    )?;
    Ok(vec![
        rejected(reordered.validate(), "reordered-transcript")?,
        rejected(duplicate.validate(), "duplicate-transcript")?,
        rejected(super::transcript::replay(&cross_call, recorded), "cross-call-replay")?,
        rejected(altered_wire.validate(), "wire-result-drift")?,
        rejected(uncertain.validate(), "uncertain-delivery")?,
        rejected(super::bridge::request(&recorded.scope, &[1]), "malformed-mapping")?,
        rejected(super::bridge::admit_shape(super::INTERFACE, super::FUNCTION, 1), "stream-future-path")?,
        rejected(super::bridge::admit_shape("wrong:world/handler", super::FUNCTION, 0), "wrong-world")?,
        rejected(super::bridge::admit_shape(super::INTERFACE, "other", 0), "unauthorized-function")?,
        rejected(resources.validate(10), "resource-contract-drift")?,
        rejected(recorded.scope.validate(recorded.scope.deadline_tick), "stale-current-context")?,
        rejected(super::transcript::replay(&wrong_peer, recorded), "unauthorized-peer-replay")?,
        rejected(super::profile::classify("production-ready"), "production-overclaim")?,
        secret,
        overflow,
        rejected(live_timing.validate(), "live-timing-replay")?,
    ])
}
fn rejected<T>(result: super::Result<T>, name: &'static str) -> super::Result<&'static str> {
    if result.is_ok() {
        return Err(format!("negative fixture unexpectedly admitted {name}"));
    }
    Ok(name)
}
