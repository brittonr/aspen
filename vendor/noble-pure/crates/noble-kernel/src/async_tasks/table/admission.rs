impl crate::async_tasks::Table {
    /// Validate bounded capacity without consuming any owner or authorization.
    // r[impl RA-ASYNC-02]
    // r[impl RA-ASYNC-05]
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; preparation validates without changing the exclusively owned table, and its retained exclusive borrow prevents capacity or generation changes until commit or abandonment."
    )]
    pub fn prepare(
        &mut self,
        request: crate::async_tasks::Request,
    ) -> Result<super::Admission<'_>, crate::async_tasks::Error> {
        let footprint = attempt!(request.footprint());
        let slot = attempt!(self.free_slot());
        attempt!(self.observation().reserved.add(footprint).fits(self.limits));
        if self.generation >= self.limits.generations {
            return Err(crate::async_tasks::Error::GenerationExhausted);
        }
        let generation = match self.generation.checked_add(1) {
            Some(generation) => generation,
            None => return Err(crate::async_tasks::Error::GenerationExhausted),
        };
        let handle = crate::async_tasks::Handle {
            table: self.id,
            slot,
            generation,
            context: request.context,
        };
        let mut buffers = 0;
        if request.input_bytes != 0 {
            buffers |= 1;
        }
        if request.parked_bytes != 0 {
            buffers |= 4;
        }
        let record = crate::async_tasks::Snapshot {
            handle,
            native: request.native,
            state: crate::async_tasks::State::Pending,
            reservation: request,
            completion: None,
            outcome: None,
            failure: None,
            completion_closed: false,
            native_stopped: false,
            pins: crate::async_tasks::bounds::mask(request.pins),
            returned_inputs: 0,
            retiring_inputs: 0,
            results: 0,
            buffers,
            wake_pending: false,
            wakeups_remaining: request.wakeups,
            finalized: false,
        };
        Ok(super::Admission {
            table: self,
            record,
        })
    }

    /// Convenience admission; all supplied obligations are returned unchanged on
    /// rejection. This does not authenticate, inspect or consume an authority.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; admission commits only a fully preflighted request to the exclusively owned table and returns every supplied move-only obligation unchanged on rejection."
    )]
    pub fn admit<T>(
        &mut self,
        request: crate::async_tasks::Request,
        inputs: T,
    ) -> Result<crate::async_tasks::Admitted<T>, crate::async_tasks::Rejected<T>> {
        match self.prepare(request) {
            Ok(admission) => Ok(admission.commit(inputs)),
            Err(error) => Err(crate::async_tasks::Rejected {
                error,
                input: inputs,
            }),
        }
    }

    fn free_slot(&self) -> Result<usize, crate::async_tasks::Error> {
        let mut index = 0;
        while index < self.records.len() {
            if self.records[index].finalized {
                return Ok(index);
            }
            index += 1;
        }
        if index < self.limits.tasks {
            Ok(index)
        } else {
            Err(crate::async_tasks::Error::TaskCapacity)
        }
    }
}

impl super::Admission<'_> {
    /// Infallible local consumption after all independent host preflights.
    /// Native work must start only after the host commits the other obligations
    /// at this same serialized boundary. Dropping this value instead aborts.
    pub fn commit<T>(self, inputs: T) -> crate::async_tasks::Admitted<T> {
        let slot = self.record.handle.slot;
        if slot == self.table.records.len() {
            self.table.records.push(self.record);
        } else {
            self.table.records[slot] = self.record;
        }
        self.table.generation = self.record.handle.generation;
        let mut decision = crate::async_tasks::Decision::unchanged(
            self.record,
            crate::async_tasks::Action::Admitted,
        );
        decision.accounting.acquired.inputs =
            crate::async_tasks::bounds::mask(self.record.reservation.inputs);
        decision.accounting.acquired.buffers = self.record.buffers;
        decision.accounting.pins_acquired = self.record.pins;
        crate::async_tasks::Admitted {
            task: crate::async_tasks::Task {
                handle: self.record.handle,
            },
            callback: crate::async_tasks::Callback {
                task: self.record.handle,
                native: self.record.native,
            },
            inputs,
            decision,
        }
    }
}
