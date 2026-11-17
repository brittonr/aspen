#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; registry methods are explicit bounded state transitions on Core-owned tables; caller Prepared, Subject and host state remain immutable; checked indices precede insertion"
)]

impl crate::companion::registry::Registry {
    pub(crate) fn new(limits: crate::Limits) -> Self {
        let cap = usize::try_from(limits.nodes).ok();
        Self {
            contracts: alloc::vec::Vec::new(),
            evidence: alloc::vec::Vec::new(),
            observations: alloc::vec::Vec::new(),
            contract_cap: cap,
            evidence_cap: cap,
            observation_cap: cap,
        }
    }

    pub(crate) const fn stale(
        engine: &crate::companion::Core,
        retained: crate::companion::registry::ContextSnapshot,
    ) -> bool {
        engine.revision == u32::MAX
            || retained.policy != engine.policy
            || retained.revision != engine.revision
    }

    /// Retention identity includes the full exact statement, program and current context.
    pub(crate) fn find_contract(
        &self,
        entry: &crate::companion::registry::ContractEntry,
    ) -> Option<crate::companion::registry::ContractId> {
        let mut index = 0usize;
        let mut found = None;
        while index < self.contracts.len() {
            let existing = &self.contracts[index];
            if matches_contract(existing, entry) {
                found = u32::try_from(index)
                    .ok()
                    .map(crate::companion::registry::ContractId);
                break;
            }
            index = index.saturating_add(1);
        }
        found
    }

    pub(crate) fn add_contract(
        &mut self,
        entry: crate::companion::registry::ContractEntry,
    ) -> Result<crate::companion::registry::ContractId, crate::companion::Refusal> {
        if let Some(existing) = self.find_contract(&entry) {
            return Ok(existing);
        }
        if !has_capacity(self.contracts.len(), self.contract_cap) {
            return Err(crate::companion::Refusal::ExhaustedRegistry);
        }
        let id = attempt!(index_of(self.contracts.len()));
        self.contracts.push(entry);
        Ok(crate::companion::registry::ContractId(id))
    }

    pub(crate) fn contract(
        &self,
        id: crate::companion::registry::ContractId,
    ) -> Result<&crate::companion::registry::ContractEntry, crate::companion::Refusal> {
        let position = match usize::try_from(id.0) {
            Ok(position) => position,
            Err(_) => return Err(crate::companion::Refusal::UnknownContract),
        };
        match self.contracts.get(position) {
            Some(entry) => Ok(entry),
            None => Err(crate::companion::Refusal::UnknownContract),
        }
    }

    pub(crate) fn find_evidence(
        &self,
        contract: crate::companion::registry::ContractId,
        statement: u64,
        subject: &crate::companion::Subject,
    ) -> Option<crate::companion::registry::EvidenceId> {
        let mut index = 0usize;
        let mut found = None;
        while index < self.evidence.len() {
            let entry = &self.evidence[index];
            if entry.contract == contract
                && entry.statement == statement
                && entry.subject == *subject
            {
                found = u32::try_from(index)
                    .ok()
                    .map(crate::companion::registry::EvidenceId);
                break;
            }
            index = index.saturating_add(1);
        }
        found
    }

    /// Inspection-only cache: admission, binding and replay compare full retained observations.
    pub(crate) fn cached_events(
        &self,
        subject: crate::companion::SubjectDigest,
    ) -> Option<&[(u32, u64)]> {
        let mut index = 0usize;
        while index < self.observations.len() {
            if self.observations[index].subject == subject {
                return Some(&self.observations[index].events);
            }
            index = index.saturating_add(1);
        }
        None
    }

    pub(crate) fn add_evidence(
        &mut self,
        entry: crate::companion::registry::EvidenceEntry,
    ) -> Result<crate::companion::registry::EvidenceId, crate::companion::Refusal> {
        if !has_capacity(self.evidence.len(), self.evidence_cap) {
            return Err(crate::companion::Refusal::ExhaustedRegistry);
        }
        let id = attempt!(index_of(self.evidence.len()));
        self.evidence.push(entry);
        Ok(crate::companion::registry::EvidenceId(id))
    }

    pub(crate) fn evidence(
        &self,
        id: crate::companion::registry::EvidenceId,
    ) -> Result<&crate::companion::registry::EvidenceEntry, crate::companion::Refusal> {
        let position = match usize::try_from(id.0) {
            Ok(position) => position,
            Err(_) => return Err(crate::companion::Refusal::UnknownEvidence),
        };
        match self.evidence.get(position) {
            Some(entry) => Ok(entry),
            None => Err(crate::companion::Refusal::UnknownEvidence),
        }
    }

    pub(crate) fn next_evidence(&self) -> Result<u32, crate::companion::Refusal> {
        index_of(self.evidence.len())
    }

    pub(crate) fn record_observation(
        &mut self,
        subject: &crate::companion::Subject,
        events: &[(u32, u64)],
    ) -> Result<(), crate::companion::Refusal> {
        if !has_capacity(self.observations.len(), self.observation_cap)
            || events.len() > crate::companion::subject::OBSERVATION_CAP
        {
            return Err(crate::companion::Refusal::ExhaustedRegistry);
        }
        let mut cached = alloc::vec::Vec::with_capacity(events.len());
        let mut index = 0usize;
        while index < events.len() {
            cached.push(events[index]);
            index = index.saturating_add(1);
        }
        self.observations
            .push(crate::companion::registry::Observation {
                subject: subject.identity,
                events: cached,
            });
        Ok(())
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; matches_contract rejects unequal retained statements, programs, interfaces and contexts in order rather than asserting equality. String, Vec and derived claim/type equality require runtime trait calls."
)]
fn matches_contract(
    existing: &crate::companion::registry::ContractEntry,
    entry: &crate::companion::registry::ContractEntry,
) -> bool {
    if existing.statement != entry.statement
        || existing.exact_statement != entry.exact_statement
        || existing.claim != entry.claim
    {
        return false;
    }
    if existing.program != entry.program
        || existing.input != entry.input
        || existing.output != entry.output
    {
        return false;
    }
    if existing.input_signature != entry.input_signature
        || existing.output_signature != entry.output_signature
        || existing.policy != entry.policy
    {
        return false;
    }
    existing.revision == entry.revision
}

const fn has_capacity(count: usize, capacity: Option<usize>) -> bool {
    match capacity {
        Some(limit) => count < limit,
        None => false,
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; index_of uses non-const TryFrom to reject registry positions outside the u32 evidence/contract index domain without truncation."
)]
fn index_of(count: usize) -> Result<u32, crate::companion::Refusal> {
    match u32::try_from(count) {
        Ok(index) => Ok(index),
        Err(_) => Err(crate::companion::Refusal::ExhaustedRegistry),
    }
}
