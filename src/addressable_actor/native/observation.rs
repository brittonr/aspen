use crate::addressable_actor::ActorCommitPort;

/// Read-only, three-owner evidence for the original logical result. No retry is authorized.
#[derive(Clone, Debug, serde::Serialize)]
pub struct CounterResult {
    pub actor_key_ref: String,
    pub request_id: String,
    pub value: u64,
    pub native_event_ref: String,
    pub native_commit_ref: String,
    pub semantic_state_ref: String,
    pub actor_state_ref: String,
    pub delivery_state_ref: String,
    pub receipt_ref: String,
    pub acknowledgement_ref: String,
    pub actor_completion_receipt_ref: String,
    #[serde(skip)]
    canonical_value: preserves::IOValue,
}

impl CounterResult {
    /// Exact canonical projection used for `receipt_ref`; retaining it grants no authority.
    pub fn value(&self) -> &preserves::IOValue {
        &self.canonical_value
    }
}

impl<C: crate::fabric_time::TimerClockAdapter> super::Consumer<'_, C> {
    /// Read the original completed result without dispatching, acknowledging or committing again.
    /// `None` means completion is not established, never permission for an automatic retry.
    pub fn result(
        &mut self,
        input: &crate::addressable_actor::counter::Add,
        original_ingress_ref: &str,
        logical_tick: u64,
        original_acknowledgement: Option<&molten_core::coordination_delivery::DeliveryRequest>,
    ) -> crate::error::Result<Option<CounterResult>> {
        let key = molten_core::addressable_actor::identify_actor_key(&self.configuration.actor_key);
        if input.actor_key_ref != key {
            return Err(super::invalid("counter result targets another actor key"));
        }
        let payload = input.materialize()?;
        let actor = self
            .store
            .load(&key)
            .map_err(problem)?
            .ok_or_else(|| super::invalid("counter actor metadata is absent"))?;
        super::binding::observe(self.owner, self.signing, self.placement, super::binding::Input {
            configuration: self.configuration,
            input_ref: &payload.value_ref,
            logical_tick,
        })?;
        self.signing.prepare(&self.owner.delivery_native_commit_preview(
            &key,
            original_ingress_ref,
            logical_tick,
        )?)?;
        let Some(proof) = self.owner.delivery_native_commit_optional(&key, original_ingress_ref, logical_tick)? else {
            return Ok(None);
        };
        if proof.content_ref() != payload.value_ref {
            return Err(super::invalid("counter result ingress does not bind the original logical request"));
        }
        let state = crate::addressable_actor::counter::State::decode(&proof.state().bytes)?;
        let Some(completed) = state
            .completed
            .iter()
            .find(|done| done.request_id == input.request_id && done.increment == input.increment)
        else {
            return Ok(None);
        };
        if state.actor_key_ref != key
            || !actor.state.completed_event_refs.iter().any(|event| event == proof.event_ref())
        {
            return Ok(None);
        }
        let Some(completion) = completed_evidence(&self.store, self.delivery, OriginalCompletion {
            actor_key_ref: &key,
            ingress_ref: original_ingress_ref,
            queue_id: &self.configuration.queue_id,
            item_ref: &payload.value_ref,
            acknowledgement: original_acknowledgement,
        })?
        else {
            return Ok(None);
        };
        let acknowledgement = &completion.acknowledgement;
        let molten_core::addressable_actor::ActorOperation::CompleteDelivery {
            delivery_item_ref,
            semantic_event_ref,
            semantic_commit_ref,
            ..
        } = &completion.operation
        else {
            return Err(super::invalid("retained actor completion is not a delivery completion"));
        };
        if delivery_item_ref != &payload.value_ref
            || semantic_event_ref != proof.event_ref()
            || semantic_commit_ref != proof.commit_ref()
            || !has_proof_references(&acknowledgement.evidence_refs, [
                proof.event_ref(),
                proof.commit_ref(),
                proof.state().value_ref.as_str(),
            ])
        {
            return Err(super::invalid("retained actor completion differs from original native proof"));
        }
        let delivery_state_ref = self
            .delivery
            .queue_state_ref(&self.configuration.queue_id)?
            .ok_or_else(|| super::invalid("completed counter delivery queue disappeared"))?;
        CounterResult::from_evidence(key, input, completed.result, Evidence {
            proof: &proof,
            actor,
            delivery_state_ref,
            completion: &completion,
        })
    }
}

fn has_proof_references(evidence: &[String], required: [&str; 3]) -> bool {
    required.into_iter().all(|reference| evidence.iter().any(|supplied| supplied.as_str() == reference))
}

pub(crate) struct OriginalCompletion<'a> {
    pub actor_key_ref: &'a str,
    pub ingress_ref: &'a str,
    pub queue_id: &'a str,
    pub item_ref: &'a str,
    pub acknowledgement: Option<&'a molten_core::coordination_delivery::DeliveryRequest>,
}

/// Shared read-only result boundary: terminal payload rows alone never establish an original
/// result.
pub(crate) fn completed_evidence<C: crate::fabric_time::TimerClockAdapter>(
    store: &crate::addressable_actor::Local,
    delivery: &crate::addressable_actor::delivery::Bridge<C>,
    original: OriginalCompletion<'_>,
) -> crate::error::Result<Option<crate::addressable_actor::store::completion::Evidence>> {
    let Some(completion) = store.completion(original.actor_key_ref, original.ingress_ref).map_err(problem)? else {
        return Ok(None);
    };
    if !completion.matches_original(original.acknowledgement)
        || completion.acknowledgement.queue_id != original.queue_id
    {
        return Ok(None);
    }
    let terminal = delivery.completed_acknowledgement(&completion.acknowledgement)?;
    if terminal.as_ref().is_none_or(|terminal| {
        terminal.item.item_ref != original.item_ref || terminal.item.content_ref != original.item_ref
    }) {
        return Ok(None);
    }
    Ok(Some(completion))
}

fn problem(error: impl std::fmt::Debug) -> crate::error::Failure {
    super::invalid(format!("native counter result: {error:?}"))
}

struct Evidence<'a> {
    proof: &'a crate::addressable_actor::delivery::Commit,
    actor: crate::addressable_actor::PublishedState,
    delivery_state_ref: String,
    completion: &'a crate::addressable_actor::store::completion::Evidence,
}

impl CounterResult {
    fn from_evidence(
        key: String,
        input: &crate::addressable_actor::counter::Add,
        result: u64,
        evidence: Evidence<'_>,
    ) -> crate::error::Result<Option<Self>> {
        let value = crate::preserves_rail::record("addressable-counter-result-observation-v2", vec![
            crate::preserves_rail::string(&key),
            crate::preserves_rail::string(&input.request_id),
            crate::preserves_rail::u64_value(result),
            crate::preserves_rail::string(evidence.proof.event_ref()),
            crate::preserves_rail::string(evidence.proof.commit_ref()),
            crate::preserves_rail::string(&evidence.proof.state().value_ref),
            crate::preserves_rail::string(&evidence.actor.state_ref),
            crate::preserves_rail::string(&evidence.delivery_state_ref),
            crate::preserves_rail::string(molten_core::coordination_delivery::identify_delivery_request(
                &evidence.completion.acknowledgement,
            )),
            crate::preserves_rail::string(&evidence.completion.receipt_ref),
        ]);
        Ok(Some(CounterResult {
            actor_key_ref: key,
            request_id: input.request_id.clone(),
            value: result,
            native_event_ref: evidence.proof.event_ref().to_string(),
            native_commit_ref: evidence.proof.commit_ref().to_string(),
            semantic_state_ref: evidence.proof.state().value_ref.clone(),
            actor_state_ref: evidence.actor.state_ref,
            delivery_state_ref: evidence.delivery_state_ref,
            acknowledgement_ref: molten_core::coordination_delivery::identify_delivery_request(
                &evidence.completion.acknowledgement,
            ),
            actor_completion_receipt_ref: evidence.completion.receipt_ref.clone(),
            receipt_ref: crate::preserves_rail::canonical_hash(&value)?,
            canonical_value: value,
        }))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reordered_and_additional_refs_preserve_completion_binding() {
        let expected =
            ["event", "commit", "state"].map(|label| crate::preserves_rail::content_ref_from_bytes(label.as_bytes()));
        let evidence = [
            expected[2].clone(),
            crate::preserves_rail::content_ref_from_bytes(b"additional-authorized-evidence"),
            expected[0].clone(),
            expected[1].clone(),
        ];
        assert!(super::has_proof_references(&evidence, expected.each_ref().map(String::as_str)));
    }

    #[test]
    fn every_exact_proof_ref_is_required_for_completion() {
        let expected =
            ["event", "commit", "state"].map(|label| crate::preserves_rail::content_ref_from_bytes(label.as_bytes()));
        for missing in 0..expected.len() {
            let mut evidence = expected.clone();
            evidence[missing] = crate::preserves_rail::content_ref_from_bytes(b"unrelated-evidence");
            assert!(!super::has_proof_references(&evidence, expected.each_ref().map(String::as_str)));
        }
    }
}
