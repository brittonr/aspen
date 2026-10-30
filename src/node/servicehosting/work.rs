pub(super) mod migration;

impl<A: super::AuthorityPort, R: super::RuntimePort> super::Host<A, R> {
    pub fn submit(
        &mut self,
        member: &str,
        presentation: crate::system_extension::WorkPresentation,
        payload: crate::system_extension::NativeCallbackValue,
        logical_tick: u64,
    ) -> crate::error::Result<()> {
        let group_id = self.member_group(member)?;
        self.available_group(&group_id, logical_tick)?;
        let instance = self.state.members.get(member).ok_or_else(|| super::invalid("unknown work member"))?;
        crate::preserves_rail::ContentRef::parse(&presentation.work_ref)?;
        crate::system_extension::admit_callback_value(&payload, super::MAX_HOST_PAYLOAD_BYTES as u64)
            .map_err(|error| super::invalid(format!("durable work payload denied: {error:?}")))?;
        if presentation.generation != instance.lifecycle.generation
            || presentation.incarnation != instance.lifecycle.incarnation
            || !instance.is_accepting_ingress
            || self.state.work.contains_key(&presentation.work_ref)
        {
            return Err(super::invalid("work is stale, duplicate, or member is fenced"));
        }
        self.admit("submit", member, &submission_ref(&presentation, &payload)?, logical_tick)?;
        let mut next = self.state.clone();
        next.work.insert(presentation.work_ref.clone(), super::Work {
            member: member.to_string(),
            presentation,
            payload,
            claim: None,
            state: super::WorkState::Pending,
            receipt_ref: None,
            migrations: Vec::new(),
        });
        next.groups.get_mut(&group_id).ok_or_else(|| super::invalid("unknown work group"))?.last_tick = logical_tick;
        self.commit(next)
    }

    pub fn readmit(
        &mut self,
        member: &str,
        work_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::DeliveryClaim> {
        let group_id = self.member_group(member)?;
        self.available_group(&group_id, logical_tick)?;
        let instance = self.state.members.get(member).ok_or_else(|| super::invalid("unknown work member"))?;
        let work = self.state.work.get(work_ref).ok_or_else(|| super::invalid("unknown durable work"))?;
        if work.member != member
            || work.presentation.class != crate::system_extension::WorkClass::Durable
            || work.presentation.generation != instance.lifecycle.generation
            || !instance.is_accepting_ingress
            || matches!(work.state, super::WorkState::Unknown | super::WorkState::Completed)
        {
            return Err(super::invalid(
                "work cannot be readmitted: wrong owner/class/generation, fenced, or uncertain/complete",
            ));
        }
        let receipt = self.admit("readmit", member, work_ref, logical_tick)?;
        let claim = claim_for(&self.state, member, work_ref, &receipt)?;
        let mut next = self.state.clone();
        let work = next.work.get_mut(work_ref).ok_or_else(|| super::invalid("durable work disappeared"))?;
        work.claim = Some(claim.clone());
        work.state = super::WorkState::Claimed;
        next.groups.get_mut(&group_id).ok_or_else(|| super::invalid("unknown work group"))?.last_tick = logical_tick;
        self.commit(next)?;
        Ok(claim)
    }

    // r[impl molten.system_extension.native_effect_completion]
    // r[impl molten.system_extension.native_effect_uncertainty]
    pub fn deliver(&mut self, input: &super::DeliveryInput) -> crate::error::Result<super::RuntimeDelivery> {
        let group_id = self.member_group(&input.member)?;
        self.available_group(&group_id, input.logical_tick)?;
        let instance = self.state.members.get(&input.member).ok_or_else(|| super::invalid("unknown work member"))?;
        let work = self
            .state
            .work
            .get(&input.presentation.work_ref)
            .ok_or_else(|| super::invalid("unknown retained work"))?;
        if work.member != input.member
            || work.presentation != input.presentation
            || work.payload != input.ingress.payload
            || !instance.is_accepting_ingress
            || matches!(work.state, super::WorkState::Unknown | super::WorkState::Completed)
            || work.claim.as_ref().map(|claim| claim.claim_ref.as_str()) != input.claim_ref.as_deref()
        {
            return Err(super::invalid("delivery differs from retained work/current claim or member is fenced"));
        }
        crate::system_extension::admit_work_delivery(&instance.lifecycle, &input.presentation, work.claim.as_ref())
            .map_err(|issues| super::invalid(format!("work delivery denied: {issues:?}")))?;
        let input_ref = ingress_ref(input)?;
        self.admit("deliver", &input.member, &input_ref, input.logical_tick)?;
        let mut next = self.state.clone();
        next.work
            .get_mut(&input.presentation.work_ref)
            .ok_or_else(|| super::invalid("work disappeared"))?
            .state = super::WorkState::Unknown;
        next.groups.get_mut(&group_id).ok_or_else(|| super::invalid("unknown work group"))?.last_tick =
            input.logical_tick;
        self.commit(next)?;
        let result = self.runtime.deliver(&input.member, &input.ingress, input.logical_tick)?;
        let observed = self.runtime.instance(&input.member)?;
        let mut next = self.state.clone();
        next.members.insert(input.member.clone(), observed);
        let work = next.work.get_mut(&input.presentation.work_ref).ok_or_else(|| super::invalid("work disappeared"))?;
        match &result.dispatch {
            crate::system_extension::DispatchResult::Executed {
                receipt,
                approved_effects,
                ..
            } => {
                work.receipt_ref = Some(receipt.receipt_ref.clone());
                match &result.effects {
                    super::NativeEffectDisposition::NotRequested if approved_effects.is_empty() => {
                        work.state = super::WorkState::Completed;
                    }
                    super::NativeEffectDisposition::Completed(completions)
                        if !approved_effects.is_empty() && completions.len() == approved_effects.len() =>
                    {
                        work.receipt_ref = completions.last().cloned();
                        work.state = super::WorkState::Completed;
                    }
                    _ => {}
                }
            }
            crate::system_extension::DispatchResult::Failed { receipt, .. } => {
                work.receipt_ref = Some(receipt.receipt_ref.clone())
            }
            crate::system_extension::DispatchResult::Deferred { .. } => {
                work.receipt_ref = Some(result.admission.acknowledgement_ref.clone())
            }
        }
        self.commit(next)?;
        Ok(result)
    }
}

pub(super) fn ingress_ref(input: &super::DeliveryInput) -> crate::error::Result<String> {
    let ingress = &input.ingress;
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("host-delivery-input-v1", vec![
        crate::preserves_rail::string(&input.presentation.work_ref),
        super::codec::optional(input.claim_ref.as_deref()),
        crate::preserves_rail::string(&ingress.schema),
        crate::preserves_rail::string(&ingress.request_ref),
        crate::preserves_rail::string(&ingress.endpoint_ref),
        crate::preserves_rail::string(&ingress.peer_ref),
        crate::preserves_rail::string(&ingress.service_id),
        crate::preserves_rail::string(&ingress.manifest_ref),
        crate::preserves_rail::u64_value(ingress.generation),
        crate::preserves_rail::string(&ingress.authority_ref),
        crate::preserves_rail::string(&ingress.policy_ref),
        crate::preserves_rail::string(&ingress.resource_ref),
        crate::preserves_rail::string(&ingress.transport_profile_ref),
        crate::preserves_rail::string(&ingress.alpn),
        crate::preserves_rail::string(&ingress.framing),
        crate::preserves_rail::string(&ingress.payload.value_ref),
        crate::preserves_rail::u64_value(ingress.accounted_bytes),
    ]))
}

pub(super) fn claim_for(
    state: &super::State,
    member: &str,
    work_ref: &str,
    receipt: &str,
) -> crate::error::Result<crate::system_extension::DeliveryClaim> {
    let lifecycle = &state.members.get(member).ok_or_else(|| super::invalid("unknown claim member"))?.lifecycle;
    let claim_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("host-current-claim-v1", vec![
            crate::preserves_rail::string(work_ref),
            crate::preserves_rail::string(member),
            crate::preserves_rail::u64_value(lifecycle.generation),
            crate::preserves_rail::u64_value(lifecycle.incarnation),
            crate::preserves_rail::u64_value(state.revision),
            crate::preserves_rail::string(receipt),
        ]))?;
    Ok(crate::system_extension::DeliveryClaim {
        claim_ref,
        class: crate::system_extension::WorkClass::Durable,
        generation: lifecycle.generation,
        incarnation: lifecycle.incarnation,
    })
}

pub(super) fn submission_ref(
    presentation: &crate::system_extension::WorkPresentation,
    payload: &crate::system_extension::NativeCallbackValue,
) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("host-work-submit-v1", vec![
        crate::preserves_rail::string(&presentation.work_ref),
        crate::preserves_rail::string(presentation.class.as_str()),
        crate::preserves_rail::u64_value(presentation.generation),
        crate::preserves_rail::u64_value(presentation.incarnation),
        crate::preserves_rail::string(&payload.value_ref),
    ]))
}
