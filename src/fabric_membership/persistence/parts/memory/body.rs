/// Process-local test/simulation implementation of the same strict owner protocol.
#[derive(Debug, Default)]
pub struct InMemory {
    assignments: std::collections::BTreeMap<String, codec::AssignmentRecord>,
    roles: std::collections::BTreeMap<String, String>,
    pub intents: Vec<String>,
    pub commits: Vec<String>,
    pub fail_intent: bool,
    pub fail_commit: bool,
}

impl crate::fabric_membership::AssignmentPersistence for InMemory {
    fn initialize(&mut self, proposal: &crate::fabric_membership::AssignmentProposal) -> crate::fabric::FabricPortResult<crate::fabric_membership::CurrentAssignment> {
        if self.assignments.contains_key(&proposal.assignment_id) {
            return Err(crate::fabric::FabricPortError::storage("assignment already exists; initialization cannot overwrite"));
        }
        let record = codec::AssignmentRecord::propose(proposal)?;
        codec::encode_record(&record)?;
        let current = record.current()?;
        let key = role_key(&record.assignment)?;
        let predecessor = self
            .roles
            .get(&key)
            .map(|id| {
                self.assignments
                    .get(id)
                    .ok_or_else(|| crate::fabric::FabricPortError::malformed("role slot predecessor is missing"))?
                    .current()
            })
            .transpose()?;
        admit_initialization(predecessor.as_ref(), &record.assignment)?;
        self.roles.insert(key, proposal.assignment_id.clone());
        self.assignments.insert(proposal.assignment_id.clone(), record);
        Ok(current)
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
        let (commit_ref, _) = record.finish(transition, intent_ref, role_effect_ref)?;
        self.commits.push(commit_ref.clone());
        Ok(commit_ref)
    }
}

