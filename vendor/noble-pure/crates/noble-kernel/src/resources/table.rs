mod access;
mod inspection;
mod transfer;

/// Private retained authority and obligations. There is no snapshot-import API.
///
/// A host must keep one table for each non-reused `TableId`; constructing two
/// tables with the same namespace does not establish cross-host uniqueness.
/// The shell must serialize calls and retain this table across cancellation.
/// Nothing in this module authorizes a protected operation or witnesses that
/// native work stopped: it only arbitrates the separate ownership obligation.
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the production resource proof binds resources::table::Table and its decide implementation; preserving that audited semantic path is part of the extraction contract."
)]
pub struct Table {
    id: super::TableId,
    limits: super::Limits,
    records: alloc::vec::Vec<super::Snapshot>,
    generation: u64,
    scope: u64,
    pins: usize,
}

impl Table {
    /// Construct an empty bounded registry, not an owner or authority witness.
    pub fn new(id: super::TableId, limits: super::Limits) -> Result<Self, super::Error> {
        if limits.slots == 0 {
            return Err(super::Error::InvalidLimits);
        }
        if limits.slots > super::MAX_SLOTS {
            return Err(super::Error::InvalidLimits);
        }
        if limits.pins > limits.slots {
            return Err(super::Error::InvalidLimits);
        }
        if limits.owners_per_context > limits.slots {
            return Err(super::Error::InvalidLimits);
        }
        Ok(Self {
            id,
            limits,
            records: alloc::vec::Vec::with_capacity(limits.slots),
            generation: 0,
            scope: 0,
            pins: 0,
        })
    }

    /// Register a native obligation already acquired by the trusted shell.
    /// On failure that obligation stays with the shell, not this table. This
    /// operation must never be exposed as a raw guest-handle conversion.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; registration commits one retained native obligation only after slot, context and fresh-generation checks; the exclusively borrowed owned table is the explicit transition state."
    )]
    pub fn register(&mut self, granted: super::Requirement) -> Result<super::Owner, super::Error> {
        let slot = attempt!(self.free_slot());
        if self.context_count(granted.context) >= self.limits.owners_per_context {
            return Err(super::Error::ContextCapacity);
        }
        let generation = attempt!(next(
            self.generation,
            self.limits.generations,
            super::Error::GenerationExhausted,
        ));
        let handle = super::Handle {
            table: self.id,
            slot,
            generation,
            context: granted.context,
            kind: granted.kind,
            rights: granted.rights,
        };
        let record = super::Snapshot {
            handle,
            state: super::State::Live,
            last_scope: None,
            retirement: None,
        };
        if slot == self.records.len() {
            self.records.push(record);
        } else {
            self.records[slot] = record;
        }
        self.generation = generation;
        Ok(super::Owner { handle })
    }

    /// Read-only hostile-handle preflight. This is not native-access approval;
    /// every guest native operation must additionally consume an owner in begin.
    // r[impl WI-RES-03]
    // r[impl WI-SAFE-01]
    pub fn validate(
        &self,
        claim: super::Handle,
        required: super::Requirement,
    ) -> Result<super::Snapshot, super::Error> {
        match self.decide(claim, super::Event::Inspect(required)) {
            Ok(decision) => Ok(decision.record),
            Err(error) => Err(error),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; the audited dispatcher reads a runtime Vec through slice access before applying the allocation-free algebra; Vec dereference is not const."
    )]
    fn decide(
        &self,
        claim: super::Handle,
        event: super::Event,
    ) -> Result<super::Decision, super::Error> {
        match self.records.get(claim.slot) {
            Some(record) => super::transition(*record, claim, event),
            None => Err(super::Error::InvalidHandle),
        }
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; apply commits a successful semantic decision to this exclusively owned table and otherwise preserves it; runtime Vec storage makes the transition non-const."
    )]
    fn apply(
        &mut self,
        claim: super::Handle,
        event: super::Event,
    ) -> Result<super::Decision, super::Error> {
        let decision = attempt!(self.decide(claim, event));
        match self.commit(decision) {
            Ok(()) => Ok(decision),
            Err(error) => Err(error),
        }
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; commit checks pin arithmetic and retained slot membership before updating the exclusively owned Vec and pin count; Vec mutation is non-const and no external state is touched."
    )]
    fn commit(&mut self, decision: super::Decision) -> Result<(), super::Error> {
        let pins = match decision.action {
            super::Action::BorrowAdmitted => {
                if self.pins >= self.limits.pins {
                    return Err(super::Error::PinCapacity);
                }
                match self.pins.checked_add(1) {
                    Some(pins) => pins,
                    None => return Err(super::Error::InvalidRecord),
                }
            }
            super::Action::OwnerReturned(_) | super::Action::RetirementCompleted => {
                match self.pins.checked_sub(1) {
                    Some(pins) => pins,
                    None => return Err(super::Error::InvalidRecord),
                }
            }
            super::Action::Unchanged
            | super::Action::AccessRevoked
            | super::Action::LocalRelease
            | super::Action::OwnerTransferred => self.pins,
        };
        match self.records.get_mut(decision.record.handle.slot) {
            Some(record) => *record = decision.record,
            None => return Err(super::Error::InvalidHandle),
        }
        self.pins = pins;
        Ok(())
    }
}

#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; current and ceiling are the ordered position and bound of one monotone u64 identity space; the checked successor rejects its supplied exhaustion error instead of wrapping."
)]
const fn next(current: u64, ceiling: u64, exhausted: super::Error) -> Result<u64, super::Error> {
    if current >= ceiling {
        return Err(exhausted);
    }
    match current.checked_add(1) {
        Some(next) => Ok(next),
        None => Err(exhausted),
    }
}
