/// Per-sample public read deadline ceiling, matching the admitted client deadline bound.
const MAX_SAMPLE_DEADLINE_MS: u128 = 120_000;

impl super::runtime::Host {
    pub(super) async fn recover(
        &mut self,
        request: &super::super::protocol::Request,
    ) -> super::super::protocol::Response {
        let bounds = &self.admission.profile.recovery;
        let required = bounds.stable_membership_observations;
        let timeout = std::time::Duration::from_millis(request.deadline_ms.min(bounds.timeout_ms));
        let mut deadline = match crate::fabric_time::SupervisionDeadline::after(timeout) {
            Ok(deadline) => deadline,
            Err(error) => return super::super::protocol::uncertain(request, error.to_string()),
        };
        let mut observed = None;
        for sample in 0..required {
            let remaining = match deadline.remaining() {
                Ok(remaining) if !remaining.is_zero() => remaining,
                Ok(_) => {
                    return super::super::protocol::uncertain(request, "stable-membership-recovery-deadline".into());
                }
                Err(error) => return super::super::protocol::uncertain(request, error.to_string()),
            };
            let Ok(deadline_ms) = u64::try_from(remaining.as_millis().clamp(1, MAX_SAMPLE_DEADLINE_MS)) else {
                return super::super::protocol::uncertain(request, "stable-membership-recovery-deadline".into());
            };
            let mut probe = request.clone();
            probe.deadline_ms = deadline_ms;
            probe.operation = Some(serde_json::json!({
                "id":format!("recovery-sample-{sample}"), "process":self.admission.profile.clients[0],
                "function":"read", "object_key":self.replica.ports().application.handler().object_key(),
            }));
            let response = self.perform(&probe, false).await;
            if response.outcome != super::super::protocol::Outcome::Ok {
                return super::super::protocol::uncertain(request, "recovery-quorum-currentness-unavailable".into());
            }
            let state = self.replica.state();
            let current = (state.membership.membership_ref.clone(), state.profile.service_generation);
            if observed.as_ref().is_some_and(|previous| previous != &current) {
                return super::super::protocol::uncertain(request, "membership-changed-during-recovery".into());
            }
            observed = Some(current);
        }
        let Some((membership_ref, generation)) = observed else {
            return super::super::protocol::uncertain(request, "no-stable-membership-observations".into());
        };
        super::super::protocol::response(
            request,
            super::super::protocol::Outcome::Ok,
            serde_json::json!({
                "ready":true, "stable_membership":true, "stable_membership_observations":required,
                "membership_ref":membership_ref, "generation":generation,
            }),
            None,
        )
    }
}
