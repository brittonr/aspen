impl crate::async_tasks::Table {
    /// Validates the complete native binding before consuming result metadata.
    /// Errors and `Action::Duplicate` accept no supplied payload obligations.
    /// The shell keeps its original callback ledger: rejection never authorizes
    /// dropping a purported duplicate resource or releasing a native pin twice.
    /// A matching terminal result alone does not establish native stop.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; this exclusive table mutation applies the authenticated completion decision without accepting payload obligations on rejection or duplicate callbacks."
    )]
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; every native outcome must select its explicit completion event; adding an outcome must force this callback mapping to be reviewed rather than silently treated as success or domain failure."
    )]
    pub fn complete(
        &mut self,
        callback: crate::async_tasks::Callback,
        outcome: crate::async_tasks::Outcome,
        completion: crate::async_tasks::Completion,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
        let event = match outcome {
            crate::async_tasks::Outcome::Success => crate::async_tasks::Event::CompleteSuccess {
                native: callback.native,
                completion,
            },
            crate::async_tasks::Outcome::DomainError => {
                crate::async_tasks::Event::CompleteDomainError {
                    native: callback.native,
                    completion,
                }
            }
        };
        self.apply(callback.task, event)
    }

    /// Host observation that this operation no longer accesses native storage.
    /// During retirement this also closes result intake if no result arrived.
    /// No native pin is settled by this acknowledgement alone.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; the authenticated callback binding selects one retained native-stop transition in the exclusively owned table without settling pins or cleanup obligations."
    )]
    pub fn native_stopped(
        &mut self,
        callback: crate::async_tasks::Callback,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
        self.apply(
            callback.task,
            crate::async_tasks::Event::NativeStopped {
                native: callback.native,
            },
        )
    }

    /// Acknowledge exact pin identities, only after matching native stop.
    /// Any overlap with already-settled pins rejects the entire event.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; settlement validates callback identity, native stop and the full pin mask before committing the exact release to the exclusively owned table."
    )]
    pub fn settle_pins(
        &mut self,
        callback: crate::async_tasks::Callback,
        pins: u64,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
        self.apply(
            callback.task,
            crate::async_tasks::Event::SettlePins {
                native: callback.native,
                pins,
            },
        )
    }

    /// One queued notification per task; repeated notifications coalesce.
    /// Exhausting the admitted lifetime wake budget retires with Budget failure.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; notification commits the checked bounded wake transition to the exclusively owned table, preserving native pins and owners when exhaustion retires the task."
    )]
    pub fn wake(
        &mut self,
        callback: crate::async_tasks::Callback,
    ) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
        self.apply(
            callback.task,
            crate::async_tasks::Event::Wake {
                native: callback.native,
            },
        )
    }
}
