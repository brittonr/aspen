impl super::Table {
    /// Commit an owned boundary transfer after all scalar/data lowering has
    /// succeeded. All resource arguments, rights, receiver capacity and fresh
    /// generations are checked before any mutation. No callback or caller-sent
    /// boolean is accepted as a substitute for those checks.
    ///
    /// The returned owners belong to the receiver, in parameter order. A domain
    /// error or trap must not restore sender owners. An explicitly owner-bearing
    /// result must perform another transfer back; otherwise the receiver or host
    /// retains cleanup responsibility, including if these tokens are dropped.
    // r[impl RA-OWN-01]
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; all arguments, generations and receiver capacity are checked before committing the bounded batch to the exclusively owned table and original move-only owner vector."
    )]
    pub fn transfer(
        &mut self,
        mut owners: alloc::vec::Vec<crate::resources::Owner>,
        required: &[crate::resources::Requirement],
        receiver: crate::resources::Context,
    ) -> Result<
        crate::resources::Transferred,
        crate::resources::Rejected<alloc::vec::Vec<crate::resources::Owner>>,
    > {
        let decisions = match self.transfer_preflight(&owners, required, receiver) {
            Ok(decisions) => decisions,
            Err(error) => {
                return Err(crate::resources::Rejected {
                    error,
                    input: owners,
                })
            }
        };
        let mut index = 0;
        while index < decisions.len() {
            let decision = decisions[index];
            self.records[decision.record.handle.slot] = decision.record;
            owners[index].handle = decision.record.handle;
            self.generation = decision.record.handle.generation;
            index += 1;
        }
        Ok(crate::resources::Transferred { owners, decisions })
    }

    /// Retire every retained obligation of an invocation, including owners not
    /// reachable from a guest stack. The bounded decision list is the shell's
    /// release worklist; pins are retained and callbacks still use exact scopes.
    // r[impl RA-CLEAN-01]
    // r[impl RA-OWN-02]
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; cleanup first computes every bounded decision, then commits the invocation retirement batch to the exclusively owned table without releasing outstanding pins."
    )]
    pub fn retire_context(
        &mut self,
        context: crate::resources::Context,
        reason: crate::resources::Retirement,
    ) -> Result<alloc::vec::Vec<crate::resources::Decision>, crate::resources::Error> {
        let mut decisions = alloc::vec::Vec::with_capacity(self.context_count(context));
        let mut index = 0;
        while index < self.records.len() {
            let record = self.records[index];
            if record.handle.context == context && record.state != crate::resources::State::Retired
            {
                let decision = attempt!(crate::resources::transition(
                    record,
                    record.handle,
                    crate::resources::Event::Retire(reason),
                ));
                decisions.push(decision);
            }
            index += 1;
        }
        index = 0;
        while index < decisions.len() {
            let decision = decisions[index];
            self.records[decision.record.handle.slot] = decision.record;
            index += 1;
        }
        Ok(decisions)
    }

    fn transfer_preflight(
        &self,
        owners: &[crate::resources::Owner],
        required: &[crate::resources::Requirement],
        receiver: crate::resources::Context,
    ) -> Result<alloc::vec::Vec<crate::resources::Decision>, crate::resources::Error> {
        if owners.is_empty() {
            return Err(crate::resources::Error::ArgumentCount);
        }
        if owners.len() > self.limits.slots {
            return Err(crate::resources::Error::ArgumentCount);
        }
        if owners.len() != required.len() {
            return Err(crate::resources::Error::ArgumentCount);
        }
        let mut decisions = alloc::vec::Vec::with_capacity(owners.len());
        let mut receiver_count = self.context_count(receiver);
        let mut generation = self.generation;
        let mut index = 0;
        while index < owners.len() {
            let handle = owners[index].handle;
            match unique_argument(owners, index) {
                Ok(()) => {}
                Err(error) => return Err(error),
            }
            generation = attempt!(super::next(
                generation,
                self.limits.generations,
                crate::resources::Error::GenerationExhausted,
            ));
            let event = crate::resources::Event::Transfer {
                required: required[index],
                receiver,
                generation,
            };
            let decision = attempt!(self.decide(handle, event));
            if handle.context != receiver {
                if receiver_count >= self.limits.owners_per_context {
                    return Err(crate::resources::Error::ContextCapacity);
                }
                receiver_count += 1;
            }
            decisions.push(decision);
            index += 1;
        }
        Ok(decisions)
    }
}

fn unique_argument(
    owners: &[crate::resources::Owner],
    index: usize,
) -> Result<(), crate::resources::Error> {
    let mut previous = 0;
    while previous < index {
        if owners[previous].handle.slot == owners[index].handle.slot {
            return Err(crate::resources::Error::DuplicateOwner);
        }
        previous += 1;
    }
    Ok(())
}
