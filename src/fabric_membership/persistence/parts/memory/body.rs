/// Process-local test/simulation implementation of the same strict owner protocol.
#[derive(Debug, Default)]
pub struct InMemory {
    assignments: std::collections::BTreeMap<String, codec::AssignmentRecord>,
    roles: std::collections::BTreeMap<String, String>,
    reconciliations: std::collections::BTreeMap<String, Vec<u8>>,
    pub intents: Vec<String>,
    pub commits: Vec<String>,
    pub fail_intent: bool,
    pub fail_commit: bool,
}

impl crate::fabric_membership::AssignmentPersistence for InMemory {
    fn initialize(&mut self, proposal: &crate::fabric_membership::AssignmentProposal) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        self.initialize_slot(proposal, None)
    }

    fn current(&self, assignment_id: &str) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::CurrentAssignment>> {
        let Some(record) = self.assignments.get(assignment_id) else {
            return Ok(None);
        };
        let key = role_key(&record.assignment)?;
        require_role(self.roles.get(&key).map(String::as_str), &record.assignment)?;
        record.current().map(Some)
    }

    fn record_intent(
        &mut self,
        current: &crate::fabric_membership::RoleAssignment,
        transition: &crate::fabric_membership::AssignmentTransition,
    ) -> crate::fabric::FabricPortResult<String> {
        if self.fail_intent {
            return Err(crate::fabric::FabricPortError::storage("injected assignment intent failure"));
        }
        let key = role_key(current)?;
        require_role(self.roles.get(&key).map(String::as_str), current)?;
        let record = self
            .assignments
            .get_mut(&current.assignment_id)
            .ok_or_else(|| crate::fabric::FabricPortError::storage("assignment must be initialized before intent"))?;
        let (intent_ref, _) = record.prepare(current, transition)?;
        self.intents.push(intent_ref.clone());
        Ok(intent_ref)
    }

    fn commit(
        &mut self,
        transition: &crate::fabric_membership::AssignmentTransition,
        intent_ref: &str,
        role_effect_ref: Option<&str>,
    ) -> crate::fabric::FabricPortResult<String> {
        if self.fail_commit {
            return Err(crate::fabric::FabricPortError::uncertain("injected assignment commit failure"));
        }
        let key = role_key(&transition.next)?;
        require_role(self.roles.get(&key).map(String::as_str), &transition.next)?;
        let record = self
            .assignments
            .get_mut(&transition.next.assignment_id)
            .ok_or_else(|| crate::fabric::FabricPortError::uncertain("assignment missing during commit"))?;
        let resolution = record.finish(transition, intent_ref, role_effect_ref)?;
        if let Some((_, bytes)) = resolution.reconciliation {
            self.reconciliations.insert(transition.next.assignment_id.clone(), bytes);
        }
        self.commits.push(resolution.value.clone());
        Ok(resolution.value)
    }

    fn inspect_pending(&self, assignment_id: &str) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::PendingAssignmentIntent>> {
        let Some(record) = self.assignments.get(assignment_id) else {
            return Ok(None);
        };
        let key = role_key(&record.assignment)?;
        require_role(self.roles.get(&key).map(String::as_str), &record.assignment)?;
        record.pending_intent()
    }

    fn abort_pre_effect_intent(
        &mut self,
        assignment_id: &str,
        intent_ref: &str,
        lifecycle: &mut dyn crate::fabric_membership::ExtensionRoleLifecyclePort,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        self.recover(assignment_id, |record| record.abort(intent_ref, lifecycle))
    }

    fn reconcile_pending_intent(
        &mut self,
        assignment_id: &str,
        intent_ref: &str,
        reconciliation: crate::fabric_membership::PendingIntentReconciliation<'_>,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::ReconciledAssignment> {
        self.recover(assignment_id, |record| record.reconcile(intent_ref, reconciliation))
    }

    fn quarantine_reconciliation(&self, assignment_id: &str) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::QuarantineReconciliation>> {
        self.quarantine_reconciliation_record(assignment_id)
    }

    fn reconcile_legacy_quarantine(
        &mut self,
        assignment_id: &str,
        authority: &mut dyn crate::fabric_membership::QuarantineAuthorityPort,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::QuarantineReconciliation> {
        let record = self
            .assignments
            .get(assignment_id)
            .ok_or_else(|| crate::fabric::FabricPortError::storage("assignment missing during migration"))?;
        let key = role_key(&record.assignment)?;
        require_role(self.roles.get(&key).map(String::as_str), &record.assignment)?;
        if self.reconciliations.contains_key(assignment_id) {
            return Err(crate::fabric::FabricPortError::storage("quarantine reconciliation already recorded; migration cannot overwrite it"));
        }
        let (migrated, bytes) = record.legacy_reconciliation(authority)?;
        self.reconciliations.insert(assignment_id.to_string(), bytes);
        Ok(migrated)
    }

    fn initialize_quarantine_successor(
        &mut self,
        proposal: &crate::fabric_membership::AssignmentProposal,
        clearance: &crate::fabric_membership::QuarantineClearance,
        fencing: &mut dyn crate::fabric_membership::FencingAuthorityPort,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        let fence = observe_successor_fence(proposal, fencing)?;
        self.initialize_slot(proposal, Some(QuarantineAdmission { clearance, fence }))
    }
}

impl InMemory {
    fn initialize_slot(
        &mut self,
        proposal: &crate::fabric_membership::AssignmentProposal,
        quarantine: Option<QuarantineAdmission<'_>>,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        if self.assignments.contains_key(&proposal.assignment_id) {
            return Err(crate::fabric::FabricPortError::storage("assignment already exists; initialization cannot overwrite"));
        }
        let record = codec::AssignmentRecord::propose(proposal)?;
        codec::encode_record(&record)?;
        let current = record.current()?;
        let key = role_key(&record.assignment)?;
        let predecessor_id = self.roles.get(&key).cloned();
        let predecessor = predecessor_id
            .as_ref()
            .map(|id| {
                self.assignments
                    .get(id)
                    .ok_or_else(|| crate::fabric::FabricPortError::malformed("role slot predecessor is missing"))?
                    .current()
            })
            .transpose()?;
        let reconciliation = match (&quarantine, &predecessor_id) {
            (Some(_), Some(id)) => self.quarantine_reconciliation_record(id)?,
            _ => None,
        };
        admit_initialization(predecessor.as_ref(), &record.assignment, quarantine, reconciliation.as_ref())?;
        self.roles.insert(key, proposal.assignment_id.clone());
        self.assignments.insert(proposal.assignment_id.clone(), record);
        Ok(current)
    }

    fn quarantine_reconciliation_record(&self, assignment_id: &str) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::QuarantineReconciliation>> {
        self.reconciliations
            .get(assignment_id)
            .map(|bytes| codec::decode_reconciliation(bytes, assignment_id))
            .transpose()
    }

    /// Resolves one role-slot-owning record on a copy and replaces it only on success.
    fn recover<T>(
        &mut self,
        assignment_id: &str,
        resolve: impl FnOnce(&mut codec::AssignmentRecord) -> crate::fabric::FabricPortResult<codec::Resolution<T>>,
    ) -> crate::fabric::FabricPortResult<T> {
        let record = self
            .assignments
            .get(assignment_id)
            .ok_or_else(|| crate::fabric::FabricPortError::storage("assignment missing during recovery"))?;
        let key = role_key(&record.assignment)?;
        require_role(self.roles.get(&key).map(String::as_str), &record.assignment)?;
        let mut next = record.clone();
        let resolution = resolve(&mut next)?;
        self.assignments.insert(assignment_id.to_string(), next);
        if let Some((_, bytes)) = resolution.reconciliation {
            self.reconciliations.insert(assignment_id.to_string(), bytes);
        }
        Ok(resolution.value)
    }
}

