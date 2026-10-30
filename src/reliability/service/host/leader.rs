#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerProbe {
    pub public_key: String,
    pub source_binding_ref: String,
    pub transport_session_ref: String,
}

impl super::runtime::Host {
    pub(super) async fn forward_probe(
        &mut self,
        request: &super::super::protocol::Request,
    ) -> super::super::protocol::Response {
        match self.peer_currentness(request).await {
            Ok(()) => super::super::protocol::response(
                request,
                super::super::protocol::Outcome::Ok,
                serde_json::Value::Null,
                None,
            ),
            Err(error) => super::super::protocol::uncertain(request, error.to_string()),
        }
    }

    async fn peer_currentness(&mut self, request: &super::super::protocol::Request) -> crate::error::Result<()> {
        let node = self
            .replica
            .state()
            .leader_id
            .clone()
            .ok_or_else(|| crate::error::Failure::invalid_harness("leader not observed"))?;
        let endpoint = *self
            .admission
            .peers
            .get(&node)
            .ok_or_else(|| crate::error::Failure::invalid_harness("leader outside admitted endpoint cohort"))?;
        if endpoint == self.admission.endpoint {
            return Err(crate::error::Failure::invalid_harness("observed leader not current"));
        }
        let peer = self
            .context
            .peer_probes
            .get(&node)
            .cloned()
            .ok_or_else(|| crate::error::Failure::invalid_harness("leader probe key outside signed cohort"))?;
        let mut describe = request.clone();
        describe.endpoint = endpoint.to_string();
        describe.authorization = None;
        describe.operation = None;
        describe.probe = None;
        let response = self.exchange_at(&describe, endpoint).await;
        if response.outcome != super::super::protocol::Outcome::Ok {
            return Err(crate::error::Failure::invalid_harness("leader readiness selectors unavailable"));
        }
        let mut challenge: super::super::authority::Challenge = serde_json::from_value(response.value)
            .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
        if challenge.node_id != node
            || challenge.endpoint != endpoint.to_string()
            || challenge.source_binding_ref != peer.source_binding_ref
        {
            return Err(crate::error::Failure::invalid_harness("leader probe selectors outside signed cohort"));
        }
        challenge.nonce = iroh::SecretKey::generate().to_bytes();
        describe.probe = Some(challenge.clone());
        let response = self.exchange_at(&describe, endpoint).await;
        if response.outcome != super::super::protocol::Outcome::Ok {
            return Err(crate::error::Failure::invalid_harness("leader signed quorum readiness unavailable"));
        }
        let signed: super::super::authority::SignedProbe = serde_json::from_value(response.value)
            .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
        let proof = super::super::authority::verify_peer_probe(&peer.public_key, &signed, &challenge)?;
        let state = self.replica.state();
        if !proof.ready
            || proof.group_ref != state.profile.group_binding_ref
            || proof.membership_ref != state.membership.membership_ref
            || proof.generation != state.profile.service_generation
            || proof.transport_session_ref != peer.transport_session_ref
        {
            return Err(crate::error::Failure::invalid_harness("leader signed probe is not current for this group"));
        }
        self.context.require_current("probe")
    }
}
