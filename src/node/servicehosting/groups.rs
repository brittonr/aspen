mod lifecycle;
mod report;

impl<A: super::AuthorityPort, R: super::RuntimePort> super::Host<A, R> {
    pub fn open(root: &crate::node_state::DirectoryRoot, authority: A, runtime: R) -> crate::error::Result<Self> {
        let store = super::store::Database::open(root)?;
        let state = store.load()?;
        Ok(Self {
            store,
            authority,
            runtime,
            state,
        })
    }

    pub fn install(
        &mut self,
        declaration: crate::system_extension::RecoveryGroupDeclaration,
        logical_tick: u64,
    ) -> crate::error::Result<()> {
        if !crate::system_extension::validate_recovery_group(&declaration).is_empty()
            || self.state.groups.contains_key(&declaration.group_id)
            || declaration.max_restarts_per_group > crate::system_extension::MAX_RECOVERY_HISTORY as u64
        {
            return Err(super::invalid("invalid or already installed recovery group"));
        }
        let input_ref = super::codec::declaration_ref(&declaration)?;
        let admission_ref = self.admit("install", &declaration.group_id, &input_ref, logical_tick)?;
        let mut next = self.state.clone();
        let mut pending = Vec::with_capacity(declaration.members.len().saturating_mul(2));
        for member in &declaration.members {
            if next.members.contains_key(&member.member_id) {
                return Err(super::invalid("hosted member already belongs to another group"));
            }
            let instance = self.runtime.instance(&member.member_id)?;
            if instance.service_id != member.member_id {
                return Err(super::invalid("runtime member identity differs from recovery declaration"));
            }
            pending.push(super::Action {
                member: member.member_id.clone(),
                kind: super::RuntimeAction::Start,
                before: instance.clone(),
                is_attempted: false,
            });
            pending.push(super::Action {
                member: member.member_id.clone(),
                kind: super::RuntimeAction::Checkpoint,
                before: instance.clone(),
                is_attempted: false,
            });
            next.members.insert(member.member_id.clone(), instance);
        }
        let group_id = declaration.group_id.clone();
        next.groups.insert(group_id.clone(), super::Group {
            declaration,
            history: Vec::new(),
            last_tick: logical_tick,
            admission_ref,
            pending,
            is_terminal: false,
        });
        self.commit(next)?;
        self.drive(&group_id, logical_tick)
    }

    pub fn supervise(
        &mut self,
        group_id: &str,
        exit: &crate::system_extension::RecoveryExit,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::RecoveryDecision> {
        let group = self.available_group(group_id, logical_tick)?;
        let plan = crate::system_extension::plan_recovery(&crate::system_extension::RecoveryPlanInput {
            group: &group.declaration,
            exit,
            history: &group.history,
            now_tick: logical_tick,
        })
        .map_err(|issues| super::invalid(format!("group recovery denied: {issues:?}")))?;
        let input_ref = exit_ref(exit)?;
        let admission_ref = self.admit("supervise", group_id, &input_ref, logical_tick)?;
        let mut next = self.state.clone();
        let mut actions = Vec::with_capacity(plan.actions.len());
        let mut is_terminal = false;
        for action in &plan.actions {
            match action {
                crate::system_extension::RecoveryAction::Stop { member_id } => {
                    let kind = if plan.actions.iter().any(|action| matches!(action,
                        crate::system_extension::RecoveryAction::Restart { member_id: restarted } if restarted == member_id)) {
                        super::RuntimeAction::Fence
                    } else { super::RuntimeAction::Stop };
                    actions.push(self.action(member_id, kind)?);
                }
                crate::system_extension::RecoveryAction::Restart { member_id } => {
                    actions.push(self.action(member_id, super::RuntimeAction::Restart)?);
                }
                crate::system_extension::RecoveryAction::Quarantine { .. }
                | crate::system_extension::RecoveryAction::EscalateGroup { .. } => is_terminal = true,
            }
        }
        for action in &actions {
            fence(&mut next, &action.member);
        }
        let group = next.groups.get_mut(group_id).ok_or_else(|| super::invalid("unknown recovery group"))?;
        group
            .history
            .retain(|attempt| attempt.observed_at_tick >= logical_tick.saturating_sub(group.declaration.window_ticks));
        if plan.consumed_restart {
            if group.history.len() >= crate::system_extension::MAX_RECOVERY_HISTORY {
                return Err(super::invalid("durable group restart history exhausted"));
            }
            group.history.push(crate::system_extension::RecoveryAttempt {
                member_id: exit.member_id.clone(),
                observed_at_tick: logical_tick,
            });
        }
        group.pending = actions;
        group.last_tick = logical_tick;
        group.admission_ref = admission_ref;
        group.is_terminal = is_terminal;
        self.commit(next)?;
        self.drive(group_id, logical_tick)?;
        Ok(plan.decision)
    }

    pub(super) fn admit(
        &mut self,
        operation: &str,
        subject: &str,
        input_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<String> {
        let request = self.request(operation, subject, input_ref, logical_tick)?;
        let receipt = self.authority.admit(&request)?;
        crate::preserves_rail::ContentRef::parse(&receipt)?;
        Ok(receipt)
    }

    pub(super) fn state_ref(&self) -> crate::error::Result<String> {
        crate::preserves_rail::canonical_hash(&super::codec::encode(&self.state)?)
    }

    pub(super) fn commit(&mut self, mut next: super::State) -> crate::error::Result<()> {
        next.revision =
            self.state.revision.checked_add(1).ok_or_else(|| super::invalid("hosting revision overflow"))?;
        self.store.save(&next)?;
        self.state = next;
        Ok(())
    }

    pub(super) fn available_group(&self, group_id: &str, logical_tick: u64) -> crate::error::Result<&super::Group> {
        let group = self.state.groups.get(group_id).ok_or_else(|| super::invalid("unknown recovery group"))?;
        if group.is_terminal || !group.pending.is_empty() || logical_tick < group.last_tick {
            return Err(super::invalid("recovery group is terminal, pending, or logical time regressed"));
        }
        Ok(group)
    }

    pub(super) fn member_group(&self, member: &str) -> crate::error::Result<String> {
        self.state
            .groups
            .values()
            .find(|group| group.declaration.members.iter().any(|entry| entry.member_id == member))
            .map(|group| group.declaration.group_id.clone())
            .ok_or_else(|| super::invalid("unknown hosted member"))
    }

    pub(super) fn action(&self, member: &str, kind: super::RuntimeAction) -> crate::error::Result<super::Action> {
        Ok(super::Action {
            member: member.to_string(),
            kind,
            is_attempted: false,
            before: self.state.members.get(member).cloned().ok_or_else(|| super::invalid("unknown hosted member"))?,
        })
    }
}

pub(super) fn fence(state: &mut super::State, member: &str) {
    if let Some(instance) = state.members.get_mut(member) {
        instance.is_accepting_ingress = false;
    }
    for work in state.work.values_mut().filter(|work| work.member == member) {
        work.claim = None;
        if work.state == super::WorkState::Claimed {
            work.state = super::WorkState::Pending;
        }
    }
}

pub(super) fn exit_ref(exit: &crate::system_extension::RecoveryExit) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("host-exit-v1", vec![
        crate::preserves_rail::string(&exit.member_id),
        crate::preserves_rail::bool_value(exit.was_successful),
        crate::preserves_rail::string(exit.failure.map_or("none", crate::system_extension::FailureClass::as_str)),
    ]))
}
