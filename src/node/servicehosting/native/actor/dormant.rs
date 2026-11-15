//! Bounded dormant durable staging in the existing Host work owner.

use crate::node_service_hosting::AuthorityPort;
use crate::node_service_hosting::RuntimePort;

impl<A: AuthorityPort, R: RuntimePort> crate::node_service_hosting::Host<A, R> {
    /// Describe staging exactly these durable bytes while the current native actor is Drained.
    /// This grants no authority, runs no callback, and does not reopen ingress.
    pub fn actor_submit_dormant_preview(
        &self,
        member: &str,
        presentation: &crate::system_extension::WorkPresentation,
        payload: &crate::system_extension::NativeCallbackValue,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        let current = self.actor_current(member, logical_tick)?;
        if current.lifecycle.phase != crate::system_extension::LifecyclePhase::Drained
            || current.is_accepting_ingress
            || presentation.class != crate::system_extension::WorkClass::Durable
            || presentation.generation != current.lifecycle.generation
            || presentation.incarnation != current.lifecycle.incarnation
            || self.state.work.contains_key(&presentation.work_ref)
            || self.state.work.len() >= crate::node_service_hosting::MAX_HOST_WORK
        {
            return Err(crate::node_service_hosting::invalid(
                "dormant staging requires current Drained durable work with available unique bounded storage",
            ));
        }
        crate::preserves_rail::ContentRef::parse(&presentation.work_ref)?;
        crate::system_extension::admit_callback_value(
            payload,
            crate::node_service_hosting::MAX_HOST_PAYLOAD_BYTES as u64,
        )
        .map_err(|error| crate::node_service_hosting::invalid(format!("dormant work payload denied: {error:?}")))?;
        let submission = crate::node_service_hosting::work::submission_ref(presentation, payload)?;
        let input_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "host-actor-dormant-submit-v1",
            vec![
                crate::system_extension::canonical_instance_record(&current)?.value,
                crate::preserves_rail::string(submission),
            ],
        ))?;
        self.request("actor-submit-dormant", member, &input_ref, logical_tick)
    }

    /// Persist durable work in the same Host store under exact current authority, without a
    /// callback. The dormant generation/incarnation stays explicit. Recovery requires existing
    /// Host readmission to mint its actual new-incarnation claim before any later native
    /// delivery is admitted.
    pub fn actor_submit_dormant(
        &mut self,
        member: &str,
        presentation: crate::system_extension::WorkPresentation,
        payload: crate::system_extension::NativeCallbackValue,
        logical_tick: u64,
    ) -> crate::error::Result<()> {
        let request = self.actor_submit_dormant_preview(member, &presentation, &payload, logical_tick)?;
        self.admit(&request.operation, member, &request.input_ref, logical_tick)?;
        let group_id = self.member_group(member)?;
        let mut next = self.state.clone();
        next.work.insert(presentation.work_ref.clone(), crate::node_service_hosting::Work {
            member: member.to_string(),
            presentation,
            payload,
            claim: None,
            state: crate::node_service_hosting::WorkState::Pending,
            receipt_ref: None,
            migrations: Vec::new(),
        });
        next.groups
            .get_mut(&group_id)
            .ok_or_else(|| crate::node_service_hosting::invalid("unknown dormant work group"))?
            .last_tick = logical_tick;
        self.commit(next)
    }
}
