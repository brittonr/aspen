impl super::Accounting {
    pub const fn empty() -> Self {
        Self {
            acquired: super::Obligations::empty(),
            consumed_inputs: 0,
            retirement_withdrawn_inputs: 0,
            retired: super::Obligations::empty(),
            delivered: super::Obligations::empty(),
            cleaned: super::Obligations::empty(),
            pins_acquired: 0,
            pins_released: 0,
            rejected_result_bytes: 0,
            completion_accepted: false,
            wake_queued: false,
            wake_removed: false,
            reservation_released: false,
        }
    }
}

impl super::Decision {
    /// Local accounting only; the shell applies each committed delta once.
    pub const fn accounting(&self) -> super::Accounting {
        self.accounting
    }

    pub(super) const fn unchanged(record: super::Snapshot, action: super::Action) -> Self {
        Self {
            record,
            action,
            accounting: super::Accounting::empty(),
        }
    }
}
