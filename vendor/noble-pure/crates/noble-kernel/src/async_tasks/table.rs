mod admission;
mod callbacks;
mod inspection;
mod ownership;

/// One serialized, non-reused task namespace. All native storage is external.
/// No API accepts caller snapshots as replacements for privately retained state.
pub struct Table {
    id: super::TableId,
    limits: super::Limits,
    records: alloc::vec::Vec<super::Snapshot>,
    generation: u64,
}

/// Exclusive preparation prevents another admission from stealing preflighted
/// capacity. It holds no witness/owner borrow and changes no retained state.
/// The host may validate and commit separate authority/resource tables while
/// retaining this value, at one serialized admission boundary.
pub struct Admission<'a> {
    table: &'a mut Table,
    record: super::Snapshot,
}

impl Table {
    pub fn new(id: super::TableId, limits: super::Limits) -> Result<Self, super::Error> {
        attempt!(super::bounds::validate_limits(limits));
        let mut records = alloc::vec::Vec::new();
        if records.try_reserve_exact(limits.tasks).is_err() {
            return Err(super::Error::StorageUnavailable);
        }
        Ok(Self {
            id,
            limits,
            records,
            generation: 0,
        })
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; the dispatcher reads a runtime Vec through slice access before applying the allocation-free transition; Vec dereference is not const on the pinned compiler."
    )]
    fn decide(
        &self,
        claim: super::Handle,
        event: super::Event,
    ) -> Result<super::Decision, super::Error> {
        if claim.table.0 != self.id.0 {
            return Err(super::Error::InvalidHandle);
        }
        match self.records.get(claim.slot) {
            Some(record) => super::transition(*record, claim, event),
            None => Err(super::Error::InvalidHandle),
        }
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; this exclusive table mutation commits only a successful pure decision for the authenticated retained slot; no caller-supplied snapshot is installed."
    )]
    fn apply(
        &mut self,
        claim: super::Handle,
        event: super::Event,
    ) -> Result<super::Decision, super::Error> {
        let decision = attempt!(self.decide(claim, event));
        // The successful decision already validated this exact retained slot.
        self.records[claim.slot] = decision.record;
        Ok(decision)
    }
}
