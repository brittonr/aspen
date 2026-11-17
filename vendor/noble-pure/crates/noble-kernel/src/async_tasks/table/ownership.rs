impl crate::async_tasks::Table {
    /// Only this consumed opaque handle may transfer a Ready result to a guest.
    /// Host cleanup/native pins can remain after the receiver obtains its owners.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; delivery consumes the move-only task only after receiver and retained-state validation, commits to the exclusively owned table and returns the same task on rejection."
    )]
    pub fn deliver(
        &mut self,
        task: crate::async_tasks::Task,
        context: crate::async_tasks::Context,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Rejected<crate::async_tasks::Task>>
    {
        self.consume(task, context, crate::async_tasks::Event::Deliver)
    }

    /// Consume guest access; success acknowledges revocation, never native stop.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; cancellation consumes checked guest access in the exclusively owned table without releasing native debt and returns the unchanged move-only task on rejection."
    )]
    pub fn cancel(
        &mut self,
        task: crate::async_tasks::Task,
        context: crate::async_tasks::Context,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Rejected<crate::async_tasks::Task>>
    {
        self.consume(task, context, crate::async_tasks::Event::Cancel)
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; this private ownership boundary checks receiver identity before applying a pure decision to the exclusively owned table, preserving the original task on every failure."
    )]
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; consume commits through apply to the runtime Vec of retained records after validating receiver identity; Vec access and mutation are not const on the pinned compiler."
    )]
    fn consume(
        &mut self,
        task: crate::async_tasks::Task,
        context: crate::async_tasks::Context,
        event: crate::async_tasks::Event,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Rejected<crate::async_tasks::Task>>
    {
        if task.handle.context.0 != context.0 {
            return Err(crate::async_tasks::Rejected {
                error: crate::async_tasks::Error::WrongContext,
                input: task,
            });
        }
        match self.apply(task.handle, event) {
            Ok(decision) => Ok(decision),
            Err(error) => Err(crate::async_tasks::Rejected { error, input: task }),
        }
    }

    /// Trusted host abnormal cleanup. Raw claims can revoke but cannot deliver
    /// owners, mint Task, consume authorization, or access native storage.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; retirement applies a validated failure event only to the exclusively owned table, preserving the primary failure and all outstanding native and cleanup debt."
    )]
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; every failure must select its explicit retirement event so that adding a failure forces review of the lifecycle mapping rather than silently substituting another primary failure."
    )]
    pub fn retire(
        &mut self,
        handle: crate::async_tasks::Handle,
        failure: crate::async_tasks::Failure,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
        let event = match failure {
            crate::async_tasks::Failure::Cancelled => crate::async_tasks::Event::Cancel,
            crate::async_tasks::Failure::Trap => crate::async_tasks::Event::Trap,
            crate::async_tasks::Failure::Deadline => crate::async_tasks::Event::Deadline,
            crate::async_tasks::Failure::Budget => crate::async_tasks::Event::Budget,
            crate::async_tasks::Failure::Internal => crate::async_tasks::Event::InternalFailure,
        };
        self.apply(handle, event)
    }

    /// Acknowledge completed cleanup, not a request to run destructors.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; cleanup commits only the exact checked obligation mask after native stop and pin settlement; mutation is confined to the exclusively owned table."
    )]
    pub fn cleanup(
        &mut self,
        handle: crate::async_tasks::Handle,
        obligations: crate::async_tasks::Obligations,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
        self.apply(handle, crate::async_tasks::Event::Cleanup(obligations))
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; taking a queued wake commits one checked notification removal to the exclusively owned table without granting an owner or releasing native debt."
    )]
    pub fn take_wake(
        &mut self,
        handle: crate::async_tasks::Handle,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
        self.apply(handle, crate::async_tasks::Event::TakeWake)
    }

    /// Release reservations/reuse eligibility only after native stop, all pins,
    /// and every remaining task obligation have settled. Failure is conservative.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; finalization commits reservation release to the exclusively owned table only after the pure decision proves native stop and complete pin/cleanup settlement."
    )]
    pub fn finish(
        &mut self,
        handle: crate::async_tasks::Handle,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
        self.apply(handle, crate::async_tasks::Event::Finish)
    }
}
