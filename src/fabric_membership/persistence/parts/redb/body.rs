// Redb implementation of the assignment owner's role traits.

impl crate::fabric_membership::AssignmentLog for Redb {
    fn initialize(
        &mut self,
        proposal: &crate::fabric_membership::AssignmentProposal,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        self.initialize_slot(proposal, None)
    }

    fn current(
        &self,
        assignment_id: &str,
    ) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::CurrentAssignment>> {
        let read = self.database.begin_read().map_err(storage_error)?;
        let table = read.open_table(ASSIGNMENTS).map_err(storage_error)?;
        let Some(bytes) = table.get(assignment_id).map_err(storage_error)? else {
            return Ok(None);
        };
        let record = codec::decode_record(bytes.value())?;
        require_key(&record, assignment_id)?;
        let roles = read.open_table(ROLES).map_err(storage_error)?;
        let key = role_key(&record.assignment)?;
        let owner = roles.get(key.as_str()).map_err(storage_error)?;
        require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
        record.current().map(Some)
    }

    fn record_intent(
        &mut self,
        current: &crate::fabric_membership::RoleAssignment,
        transition: &crate::fabric_membership::AssignmentTransition,
    ) -> crate::fabric::FabricPortResult<String> {
        let write = self.database.begin_write().map_err(storage_error)?;
        let intent_ref;
        {
            let mut table = write.open_table(ASSIGNMENTS).map_err(storage_error)?;
            let mut record = {
                let bytes = table.get(current.assignment_id.as_str()).map_err(storage_error)?.ok_or_else(|| {
                    crate::fabric::FabricPortError::storage("assignment must be initialized before intent")
                })?;
                codec::decode_record(bytes.value())?
            };
            require_key(&record, &current.assignment_id)?;
            let roles = write.open_table(ROLES).map_err(storage_error)?;
            let key = role_key(&record.assignment)?;
            let owner = roles.get(key.as_str()).map_err(storage_error)?;
            require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
            let (reference, bytes) = record.prepare(current, transition)?;
            intent_ref = reference;
            table.insert(current.assignment_id.as_str(), bytes.as_slice()).map_err(storage_error)?;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(intent_ref)
    }

    fn commit(
        &mut self,
        transition: &crate::fabric_membership::AssignmentTransition,
        intent_ref: &str,
        role_effect_ref: Option<&str>,
    ) -> crate::fabric::FabricPortResult<String> {
        let write = self.database.begin_write().map_err(uncertain_error)?;
        let commit_ref;
        {
            let mut table = write.open_table(ASSIGNMENTS).map_err(uncertain_error)?;
            let mut record = {
                let bytes = table
                    .get(transition.next.assignment_id.as_str())
                    .map_err(uncertain_error)?
                    .ok_or_else(|| crate::fabric::FabricPortError::uncertain("assignment missing during commit"))?;
                codec::decode_record(bytes.value())?
            };
            require_key(&record, &transition.next.assignment_id)?;
            let roles = write.open_table(ROLES).map_err(uncertain_error)?;
            let key = role_key(&record.assignment)?;
            let owner = roles.get(key.as_str()).map_err(uncertain_error)?;
            require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
            let resolution = record.finish(transition, intent_ref, role_effect_ref)?;
            table
                .insert(transition.next.assignment_id.as_str(), resolution.bytes.as_slice())
                .map_err(uncertain_error)?;
            if let Some((_, bytes)) = &resolution.reconciliation {
                let mut reconciliations = write.open_table(RECONCILIATIONS).map_err(uncertain_error)?;
                reconciliations
                    .insert(transition.next.assignment_id.as_str(), bytes.as_slice())
                    .map_err(uncertain_error)?;
            }
            commit_ref = resolution.value;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(commit_ref)
    }
}

impl crate::fabric_membership::PendingIntentRecovery for Redb {
    fn inspect_pending(
        &self,
        assignment_id: &str,
    ) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::PendingAssignmentIntent>> {
        let read = self.database.begin_read().map_err(storage_error)?;
        let table = read.open_table(ASSIGNMENTS).map_err(storage_error)?;
        let Some(bytes) = table.get(assignment_id).map_err(storage_error)? else {
            return Ok(None);
        };
        let record = codec::decode_record(bytes.value())?;
        require_key(&record, assignment_id)?;
        let roles = read.open_table(ROLES).map_err(storage_error)?;
        let key = role_key(&record.assignment)?;
        let owner = roles.get(key.as_str()).map_err(storage_error)?;
        require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
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
}

impl crate::fabric_membership::QuarantineRecovery for Redb {
    fn quarantine_reconciliation(
        &self,
        assignment_id: &str,
    ) -> crate::fabric::FabricPortResult<Option<crate::fabric_membership::QuarantineReconciliation>> {
        let read = self.database.begin_read().map_err(storage_error)?;
        let reconciliations = match read.open_table(RECONCILIATIONS) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(storage_error(error)),
        };
        let Some(bytes) = reconciliations.get(assignment_id).map_err(storage_error)? else {
            return Ok(None);
        };
        codec::decode_reconciliation(bytes.value(), assignment_id).map(Some)
    }

    fn reconcile_legacy_quarantine(
        &mut self,
        assignment_id: &str,
        authority: &mut dyn crate::fabric_membership::QuarantineAuthorityPort,
    ) -> crate::fabric::FabricPortResult<crate::fabric_membership::QuarantineReconciliation> {
        let write = self.database.begin_write().map_err(storage_error)?;
        let reconciliation;
        {
            let table = write.open_table(ASSIGNMENTS).map_err(storage_error)?;
            let record = {
                let bytes = table
                    .get(assignment_id)
                    .map_err(storage_error)?
                    .ok_or_else(|| crate::fabric::FabricPortError::storage("assignment missing during migration"))?;
                codec::decode_record(bytes.value())?
            };
            require_key(&record, assignment_id)?;
            let roles = write.open_table(ROLES).map_err(storage_error)?;
            let key = role_key(&record.assignment)?;
            let owner = roles.get(key.as_str()).map_err(storage_error)?;
            require_role(owner.as_ref().map(|owner| owner.value()), &record.assignment)?;
            let mut reconciliations = write.open_table(RECONCILIATIONS).map_err(storage_error)?;
            if reconciliations.get(assignment_id).map_err(storage_error)?.is_some() {
                return Err(crate::fabric::FabricPortError::storage(
                    "quarantine reconciliation already recorded; migration cannot overwrite it",
                ));
            }
            let (migrated, bytes) = record.legacy_reconciliation(authority)?;
            reconciliations.insert(assignment_id, bytes.as_slice()).map_err(storage_error)?;
            reconciliation = migrated;
        }
        write.commit().map_err(uncertain_error)?;
        Ok(reconciliation)
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
