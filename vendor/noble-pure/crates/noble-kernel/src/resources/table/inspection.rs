impl super::Table {
    /// Host accounting only. An observation cannot be imported as live state.
    #[expect(
        tigerstyle::usize_in_public_api,
        reason = "Owner: noble-maintainers; slot is adapter-local platform indexing, matching Handle.slot rather than a portable guest scalar; lookup is read-only and retained storage is bounded by MAX_SLOTS."
    )]
    pub fn snapshot(&self, slot: usize) -> Option<crate::resources::Snapshot> {
        self.records.get(slot).copied()
    }

    /// Count every retained state, including pinned retirement invisible to GC.
    pub fn observation(&self) -> crate::resources::Observation {
        let mut result = crate::resources::Observation {
            live: 0,
            busy: 0,
            retiring: 0,
            retired: 0,
            native_pins: self.pins,
        };
        let mut index = 0;
        while index < self.records.len() {
            match self.records[index].state {
                crate::resources::State::Live => result.live += 1,
                crate::resources::State::Busy(_) => result.busy += 1,
                crate::resources::State::Retiring(_) => result.retiring += 1,
                crate::resources::State::Retired => result.retired += 1,
            }
            index += 1;
        }
        result
    }

    pub(super) fn free_slot(&self) -> Result<usize, crate::resources::Error> {
        let mut index = 0;
        while index < self.records.len() {
            match self.records[index].state {
                crate::resources::State::Retired => return Ok(index),
                crate::resources::State::Live
                | crate::resources::State::Busy(_)
                | crate::resources::State::Retiring(_) => {}
            }
            index += 1;
        }
        if index < self.limits.slots {
            Ok(index)
        } else {
            Err(crate::resources::Error::Capacity)
        }
    }

    pub(super) fn context_count(&self, context: crate::resources::Context) -> usize {
        let mut count = 0;
        let mut index = 0;
        while index < self.records.len() {
            let record = self.records[index];
            if record.handle.context == context {
                match record.state {
                    crate::resources::State::Live
                    | crate::resources::State::Busy(_)
                    | crate::resources::State::Retiring(_) => count += 1,
                    crate::resources::State::Retired => {}
                }
            }
            index += 1;
        }
        count
    }
}
