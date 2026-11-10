//! Fresh authority is reacquired from operator-owned files, never command receipts.

#[path = "authority/config.rs"]
pub(super) mod config;
#[path = "authority/verify.rs"]
mod verify;

pub(super) async fn restage(
    runtime: &mut super::Driver,
    binding: &crate::fabric::CanonicalPortBinding,
    effect: &crate::system_extension::TypedEffectRequest,
) -> crate::error::Result<crate::fabric_consistency::raft::Command> {
    if binding.binding.key.port_id != crate::collaboration_scope::COMMIT_PORT_ID
        || binding.binding.key.version != crate::collaboration_scope::COMMIT_PORT_VERSION
        || effect.operation != crate::collaboration_scope::COMMIT_OPERATION
        || effect.input_schema_ref != crate::collaboration_scope::COMMIT_INPUT_SCHEMA
        || effect.output_schema_ref != crate::collaboration_scope::COMMIT_OUTPUT_SCHEMA
        || !matches!(&effect.target, crate::system_extension::EffectTarget::FabricPort(key) if key == &binding.binding.key)
        || effect.generation != runtime.installation.generation()
    {
        return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain));
    }
    let bytes = runtime
        .application_mut()
        .values
        .materialize(&effect.request_ref, crate::collaboration_scope::MAX_RECORD_BYTES)
        .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Uncertain))?;
    if bytes.value_ref != effect.request_ref
        || crate::preserves_rail::content_ref_from_bytes(&bytes.bytes) != effect.request_ref
        || bytes.bytes.len() as u64 != effect.accounted_bytes
    {
        return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain));
    }
    let staged = crate::preserves_rail::parse_canonical_bytes(&bytes.bytes)?;
    let expected = crate::collaboration_scope::parse_staged_command(&staged)?;
    let value = runtime.application_mut().materialize_command(&expected.command_ref)?;
    let (request, _) = crate::collaboration_scope::command::parse_proposal(&value)?;
    if request.reference.as_str() != expected.request_ref {
        return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain));
    }
    let trust = runtime.authority.trust()?;
    let credential = runtime.authority.credential(&effect.request_ref)?;
    let profile = runtime.authority.time_profile().clone();
    let current = runtime.current_read(&credential.read, &trust.reference, &profile).await?;
    if current.currentness().logical_now != trust.position {
        return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Stale));
    }
    // Reserve the probe durably before sending: a timeout can never permit a
    // retry to count delayed acknowledgements from an earlier read round.
    let fresh_ref = runtime.authority.reserve_probe(&effect.request_ref, &credential)?;
    let fresh = runtime.current_read(&fresh_ref, &trust.reference, &profile).await?;
    let confirmed = fresh.currentness();
    let signed = current.currentness();
    if confirmed.service_generation != signed.service_generation
        || confirmed.consistency_epoch != signed.consistency_epoch
        || confirmed.committed_index != signed.committed_index
        || confirmed.logical_now != signed.logical_now
        || confirmed.revocation != signed.revocation
        || fresh.state != current.state
    {
        return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Stale));
    }
    // Re-read independent roots after the quorum round trip. A concurrent rotation denies.
    if runtime.authority.trust()?.reference != trust.reference {
        return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Stale));
    }
    let replay = runtime.authority.replay();
    let actual = verify::stage(runtime.application_mut(), &request, current, (&trust, &credential, replay.as_ref()))?;
    if actual.request_ref != expected.request_ref || actual.command_schema_ref != expected.command_schema_ref {
        return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Authority));
    }
    Ok(actual)
}

impl super::Driver {
    pub(super) async fn observe_scope(
        &mut self,
        request: crate::collaboration_scope::RequestRef,
    ) -> crate::error::Result<super::Observation> {
        self.application().store.ensure_serving()?;
        let trust = self.authority.trust()?;
        let observation = self.observe_read(&request).await?;
        self.application().store.ensure_serving()?;
        let state = self.application().state_ref().clone();
        let currentness = self.installation.observe_currentness(&crate::collaboration_scope::CurrentInput {
            replica: self.service.state(),
            observation: &observation,
            ledger: self.service.evidence(),
            request: &request,
            time_profile: self.authority.time_profile(),
            revocation: &trust.reference,
            state: &state,
        })?;
        // Diagnostics may reveal an index newer than the operator's logical position.
        // Only restage checks that position as authority; concurrent trust rotation denies here too.
        if self.authority.trust()?.reference != trust.reference {
            return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Stale));
        }
        Ok(super::Observation {
            read: request,
            state,
            currentness,
            unix_seconds: trust.unix_seconds,
        })
    }

    /// Real non-consuming diagnostic observation; authority is acquired only in restage.
    pub(super) async fn observe_read(
        &mut self,
        request: &crate::collaboration_scope::RequestRef,
    ) -> crate::error::Result<crate::fabric_consistency::raft::ReplicaObservation> {
        while self.control.try_recv().is_ok() {}
        self.sender
            .try_send(crate::fabric_consistency::raft::ReplicaEvent::Read {
                request_ref: request.as_str().to_owned(),
                mode: crate::fabric_consistency::ReadMode::Linearizable,
            })
            .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Partition))?;
        // The runtime timer owns the deadline; the read path never samples a clock.
        let deadline = tokio::time::sleep(self.timeout);
        tokio::pin!(deadline);
        loop {
            let is_open = self.step(deadline.as_mut()).await?;
            while let Ok(observation) = self.control.try_recv() {
                if let crate::fabric_consistency::raft::ReplicaObservationKind::Read { request_ref, .. } =
                    &observation.kind
                    && request_ref == request.as_str()
                {
                    return Ok(observation);
                }
            }
            if !is_open || deadline.is_elapsed() {
                return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Partition));
            }
        }
    }

    pub(super) async fn current_read(
        &mut self,
        request: &crate::collaboration_scope::RequestRef,
        revocation: &molten_core::nominal::RevocationRef,
        time_profile: &crate::fabric_time::AdmittedTimeProfile,
    ) -> crate::error::Result<crate::collaboration_scope::CurrentState> {
        let observation = self.observe_read(request).await?;
        let state = self.application().state_ref().clone();
        self.installation.current_scope(&crate::collaboration_scope::CurrentInput {
            replica: self.service.state(),
            observation: &observation,
            ledger: self.service.evidence(),
            request,
            time_profile,
            revocation,
            state: &state,
        })
    }
}
