impl<A: super::super::AuthorityPort, R: super::super::RuntimePort> super::super::Host<A, R> {
    pub fn upgrade(&mut self, member: &str, admission_ref: &str, logical_tick: u64) -> crate::error::Result<()> {
        crate::preserves_rail::ContentRef::parse(admission_ref)?;
        let group_id = self.member_group(member)?;
        self.available_group(&group_id, logical_tick)?;
        let receipt = self.admit("upgrade", member, admission_ref, logical_tick)?;
        let mut next = self.state.clone();
        let pending = vec![
            self.action(member, super::super::RuntimeAction::Drain)?,
            self.action(member, super::super::RuntimeAction::Checkpoint)?,
            self.action(member, super::super::RuntimeAction::Upgrade {
                admission_ref: admission_ref.to_string(),
            })?,
        ];
        super::fence(&mut next, member);
        let group = next.groups.get_mut(&group_id).ok_or_else(|| super::super::invalid("unknown upgrade group"))?;
        group.pending = pending;
        group.last_tick = logical_tick;
        group.admission_ref = receipt;
        self.commit(next)?;
        self.drive(&group_id, logical_tick)
    }

    /// Reconcile an interrupted intent against actual persisted native state; never rerun it.
    pub fn reconcile(&mut self, group_id: &str, logical_tick: u64) -> crate::error::Result<()> {
        let group = self.state.groups.get(group_id).ok_or_else(|| super::super::invalid("unknown recovery group"))?;
        if logical_tick < group.last_tick {
            return Err(super::super::invalid("hosting logical time regressed"));
        }
        let input_ref = self.state_ref()?;
        self.admit("reconcile", group_id, &input_ref, logical_tick)?;
        self.drive(group_id, logical_tick)
    }

    pub(super) fn drive(&mut self, group_id: &str, logical_tick: u64) -> crate::error::Result<()> {
        loop {
            let action = self.state.groups.get(group_id).and_then(|group| group.pending.first()).cloned();
            let Some(mut action) = action else {
                return Ok(());
            };
            let observed = if action.is_attempted {
                let observed = self.runtime.instance(&action.member)?;
                // Native execution commits an instance mutation before every external callback.
                // Exact durable equality proves this action never reached that boundary.
                if observed == action.before {
                    let mut next = self.state.clone();
                    next.groups
                        .get_mut(group_id)
                        .and_then(|group| group.pending.first_mut())
                        .ok_or_else(|| super::super::invalid("pending lifecycle action disappeared"))?
                        .is_attempted = false;
                    self.commit(next)?;
                    continue;
                }
                require_postcondition(&action, &observed)?;
                observed
            } else {
                action.before = self.runtime.instance(&action.member)?;
                let mut next = self.state.clone();
                let pending = next
                    .groups
                    .get_mut(group_id)
                    .and_then(|group| group.pending.first_mut())
                    .ok_or_else(|| super::super::invalid("pending lifecycle action disappeared"))?;
                pending.before = action.before.clone();
                pending.is_attempted = true;
                self.commit(next)?;
                let observed = self.runtime.apply(&action.member, &action.kind, logical_tick)?;
                require_postcondition(&action, &observed)?;
                observed
            };
            let mut next = self.state.clone();
            next.members.insert(action.member, observed);
            let group = next.groups.get_mut(group_id).ok_or_else(|| super::super::invalid("unknown recovery group"))?;
            group.pending.remove(0);
            group.last_tick = logical_tick;
            self.commit(next)?;
        }
    }
}

fn require_postcondition(
    action: &super::super::Action,
    observed: &crate::system_extension::NativeInstanceRecord,
) -> crate::error::Result<()> {
    let before = &action.before;
    if observed.instance_id != before.instance_id
        || observed.service_id != before.service_id
        || observed.extension_id != before.extension_id
    {
        return Err(super::super::invalid("native lifecycle observation changed member identity"));
    }
    let is_same_generation = observed.lifecycle.generation == before.lifecycle.generation;
    let is_same_incarnation = observed.lifecycle.incarnation == before.lifecycle.incarnation;
    let phase = observed.lifecycle.phase;
    let is_completed = match action.kind {
        super::super::RuntimeAction::Start => {
            is_same_generation
                && phase == crate::system_extension::LifecyclePhase::Running
                && is_same_incarnation
                && observed.callback_sequence > before.callback_sequence
        }
        super::super::RuntimeAction::Stop => {
            is_same_generation && is_same_incarnation && phase == crate::system_extension::LifecyclePhase::Stopped
        }
        super::super::RuntimeAction::Fence => {
            is_same_generation && is_same_incarnation && phase == crate::system_extension::LifecyclePhase::Failed
        }
        super::super::RuntimeAction::Restart => {
            is_same_generation
                && phase == crate::system_extension::LifecyclePhase::Running
                && observed.lifecycle.incarnation > before.lifecycle.incarnation
                && before.lifecycle.restart_attempts.checked_add(1) == Some(observed.lifecycle.restart_attempts)
        }
        super::super::RuntimeAction::Drain => {
            is_same_generation && is_same_incarnation && phase == crate::system_extension::LifecyclePhase::Drained
        }
        super::super::RuntimeAction::Checkpoint => {
            is_same_generation
                && is_same_incarnation
                && observed.checkpoint_ref.is_some()
                && observed.callback_sequence > before.callback_sequence
                && matches!(
                    phase,
                    crate::system_extension::LifecyclePhase::Running | crate::system_extension::LifecyclePhase::Drained
                )
        }
        super::super::RuntimeAction::Upgrade { ref admission_ref } => {
            let observed_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "host-native-replacement-v1",
                vec![
                    crate::preserves_rail::string(&observed.manifest_ref),
                    crate::preserves_rail::string(&observed.executable_ref),
                    crate::preserves_rail::string(&observed.profile_ref),
                ],
            ))?;
            before.lifecycle.generation.checked_add(1) == Some(observed.lifecycle.generation)
                && is_same_incarnation
                && phase == crate::system_extension::LifecyclePhase::Running
                && observed.state_ref.is_some()
                && observed_ref == *admission_ref
        }
    };
    if !is_completed {
        return Err(super::super::invalid("native lifecycle intent remains uncertain; callback is not repeated"));
    }
    Ok(())
}
