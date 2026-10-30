//! Negative cases run against the real completed transcript, never fabricated successful evidence.
pub fn verify(recorded: &super::transcript::Transcript) -> super::Result<Vec<&'static str>> {
    let mut cases = Vec::new();
    let mut reordered = recorded.clone();
    reordered.events.swap(0, 1);
    rejected(reordered.validate(), "reordered-transcript", &mut cases)?;
    let mut duplicate = recorded.clone();
    duplicate.events.push(duplicate.events[2].clone());
    rejected(duplicate.validate(), "duplicate-transcript", &mut cases)?;
    let mut cross_call = recorded.scope.clone();
    cross_call.dedup = super::reference(b"another-invocation");
    rejected(super::transcript::replay(&cross_call, recorded), "cross-call-replay", &mut cases)?;
    let mut altered_wire = recorded.clone();
    altered_wire.result_wire = super::bridge::encode_result("different result")?;
    rejected(altered_wire.validate(), "wire-result-drift", &mut cases)?;
    let mut uncertain = recorded.clone();
    uncertain.response.delivery = crate::fabric_transport::DeliveryOutcome::Uncertain;
    rejected(uncertain.validate(), "uncertain-delivery", &mut cases)?;
    rejected(super::bridge::request(&recorded.scope, &[1]), "malformed-mapping", &mut cases)?;
    rejected(super::bridge::admit_shape(super::INTERFACE, super::FUNCTION, 1), "stream-future-path", &mut cases)?;
    rejected(super::bridge::admit_shape("wrong:world/handler", super::FUNCTION, 0), "wrong-world", &mut cases)?;
    rejected(super::bridge::admit_shape(super::INTERFACE, "other", 0), "unauthorized-function", &mut cases)?;
    let mut resources = recorded.scope.clone();
    resources.resources = super::reference(b"larger-resource-budget");
    rejected(resources.validate(10), "resource-contract-drift", &mut cases)?;
    rejected(recorded.scope.validate(recorded.scope.deadline_tick), "stale-current-context", &mut cases)?;
    let mut wrong_peer = recorded.scope.clone();
    wrong_peer.peer = super::reference(b"unauthorized-peer");
    rejected(super::transcript::replay(&wrong_peer, recorded), "unauthorized-peer-replay", &mut cases)?;
    rejected(super::profile::classify("production-ready"), "production-overclaim", &mut cases)?;
    let mut diagnostics = super::diagnostics::Diagnostics::default();
    rejected(
        diagnostics.observe("credential/private-payload", super::diagnostics::Status::Returned),
        "telemetry-secret-field",
        &mut cases,
    )?;
    for _ in 0..32 {
        diagnostics.observe(&recorded.reference, super::diagnostics::Status::Returned)?;
    }
    rejected(
        diagnostics.observe(&recorded.reference, super::diagnostics::Status::Returned),
        "telemetry-queue-overflow",
        &mut cases,
    )?;
    let mut live_timing = recorded.clone();
    live_timing.events[1] = super::reference(b"live-wall-clock-effect");
    rejected(live_timing.validate(), "live-timing-replay", &mut cases)?;
    Ok(cases)
}
fn rejected<T>(result: super::Result<T>, name: &'static str, cases: &mut Vec<&'static str>) -> super::Result<()> {
    if result.is_ok() {
        return Err(format!("negative fixture unexpectedly admitted {name}"));
    }
    cases.push(name);
    Ok(())
}
