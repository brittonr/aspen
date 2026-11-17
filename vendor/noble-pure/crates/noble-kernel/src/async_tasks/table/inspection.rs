impl crate::async_tasks::Table {
    /// Read-only caller-context validation, not native access or result delivery.
    pub fn inspect(
        &self,
        handle: crate::async_tasks::Handle,
        context: crate::async_tasks::Context,
    ) -> Result<crate::async_tasks::Snapshot, crate::async_tasks::Error> {
        if handle.context.0 != context.0 {
            return Err(crate::async_tasks::Error::WrongContext);
        }
        match self.decide(handle, crate::async_tasks::Event::Inspect) {
            Ok(decision) => Ok(decision.record),
            Err(error) => Err(error),
        }
    }

    /// Trusted host accounting can inspect terminal records without a capability.
    #[expect(
        tigerstyle::usize_in_public_api,
        reason = "Owner: noble-maintainers; slot is adapter-local platform indexing matching Handle.slot, not a portable guest scalar; the read-only retained table is bounded by MAX_SLOTS."
    )]
    pub fn snapshot(&self, slot: usize) -> Option<crate::async_tasks::Snapshot> {
        self.records.get(slot).copied()
    }

    #[expect(
        tigerstyle::platform_dependent_cast,
        reason = "Owner: noble-maintainers; u64::count_ones returns at most 64, which is representable as usize on every Rust target; this conversion never depends on the width of an arbitrary u32."
    )]
    pub fn observation(&self) -> crate::async_tasks::Observation {
        let mut observation = crate::async_tasks::Observation {
            pending: 0,
            ready: 0,
            delivered: 0,
            retiring: 0,
            retired: 0,
            outstanding_pins: 0,
            queued_wakeups: 0,
            reserved: crate::async_tasks::Footprint::empty(),
        };
        let mut index = 0;
        while index < self.records.len() {
            let record = self.records[index];
            match record.state {
                crate::async_tasks::State::Pending => observation.pending += 1,
                crate::async_tasks::State::Ready => observation.ready += 1,
                crate::async_tasks::State::Delivered => observation.delivered += 1,
                crate::async_tasks::State::Retiring => observation.retiring += 1,
                crate::async_tasks::State::Retired => observation.retired += 1,
            }
            observation.outstanding_pins += record.pins.count_ones() as usize;
            if record.wake_pending {
                observation.queued_wakeups += 1;
            }
            if !record.finalized {
                // Only admission installs a reservation, after validating it.
                let footprint = record.reservation.validated_footprint();
                observation.reserved = observation.reserved.add(footprint);
            }
            index += 1;
        }
        observation
    }
}
