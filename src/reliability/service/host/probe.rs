impl super::runtime::Host {
    pub(super) async fn probe(
        &mut self,
        request: &super::super::protocol::Request,
    ) -> super::super::protocol::Response {
        match self.probe_checked(request).await {
            Ok(value) => super::super::protocol::response(request, super::super::protocol::Outcome::Ok, value, None),
            Err(error) => super::super::protocol::uncertain(request, error.to_string()),
        }
    }

    async fn probe_checked(
        &mut self,
        request: &super::super::protocol::Request,
    ) -> crate::error::Result<serde_json::Value> {
        self.context.require_current("probe")?;
        let before =
            super::super::readiness::current::read(&self.context.authorization.root, &self.context.health_path)?;
        let Some(challenge) = request.probe.as_ref() else {
            return serde_json::to_value(super::super::authority::Challenge {
                nonce: [0; 32],
                node_id: self.replica.state().node_id.clone(),
                endpoint: self.admission.endpoint.to_string(),
                source_binding_ref: before.source_binding_ref,
                startup_ref: before.startup_ref,
                health_ref: before.health_ref,
            })
            .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()));
        };
        if challenge.node_id != self.replica.state().node_id
            || challenge.endpoint != self.admission.endpoint.to_string()
            || challenge.source_binding_ref != before.source_binding_ref
            || challenge.startup_ref != before.startup_ref
            || challenge.health_ref != before.health_ref
        {
            return Err(crate::error::Failure::invalid_harness(
                "readiness challenge differs from rooted actual node state",
            ));
        }
        let mut currentness = request.clone();
        currentness.phase = super::super::protocol::Phase::Recover;
        currentness.probe = None;
        // A follower asks the observed leader for authenticated quorum-currentness evidence;
        // unsigned probe requests cannot be transformed into authenticated workload calls.
        let response = if self.replica.state().role == crate::fabric_consistency::raft::ReplicaRole::Leader {
            self.recover(&currentness).await
        } else {
            self.forward_probe(request).await
        };
        if response.outcome != super::super::protocol::Outcome::Ok {
            return Err(crate::error::Failure::invalid_harness("readiness public quorum probe unavailable"));
        }
        let after =
            super::super::readiness::current::read(&self.context.authorization.root, &self.context.health_path)?;
        if before.source_binding_ref != after.source_binding_ref
            || before.startup_ref != after.startup_ref
            || before.health_ref != after.health_ref
            || before.run_ref != after.run_ref
        {
            return Err(crate::error::Failure::invalid_harness("node state changed during readiness probe"));
        }
        self.context.authorization.grant.verify_current(&self.context.authorization.root)?;
        super::control::check_bootstrap(&self.context.authorization)?;
        let bootstrap_ref = super::control::bootstrap_ref(&self.context.authorization.root)?;
        let state = self.replica.state();
        let signed = super::super::authority::sign_probe(
            &super::super::authority::load_probe_key(
                &self.context.authorization.root,
                &self.context.authorization.grant,
            )?,
            &self.context.authorization.grant,
            challenge,
            super::super::authority::ProbeState {
                grant_ref: self.context.authorization.grant.grant_ref().into(),
                generation: state.profile.service_generation,
                group_ref: state.profile.group_binding_ref.clone(),
                membership_ref: state.membership.membership_ref.clone(),
                transport_session_ref: self.context.authorization.grant.grant().transport_session_ref.clone(),
                bootstrap_ref,
                ready: true,
            },
        )?;
        self.context.authorization.grant.verify_current(&self.context.authorization.root)?;
        serde_json::to_value(signed).map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))
    }
}
