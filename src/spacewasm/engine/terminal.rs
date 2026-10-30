use super::super::continuation;
use super::super::receipt;

impl super::Engine {
    pub fn instructions(&self) -> u64 {
        self.fuel
    }

    /// Cancellation is an explicit scheduler action under the retained actor fence.
    pub fn cancel(&mut self, context: &continuation::ResumeContext, reason: String) -> continuation::Progress {
        if self.terminal {
            return continuation::Progress::ProofFailed(continuation::Failure::Harness(
                "terminal-continuation-reused".into(),
            ));
        }
        if context.current.current().as_ref() != Ok(&self.binding)
            || self.shared.current.current().as_ref() != Ok(&self.binding)
            || context.logical_turn < self.created
            || context.logical_turn - self.created > self.plan.profile.limits.retention_turns
        {
            return self.terminate(Some(continuation::Failure::Denied("stale continuation".into())));
        }
        let failure = if reason.is_empty() || reason.len() > 256 {
            continuation::Failure::Denied("invalid-cancellation-reason".into())
        } else {
            continuation::Failure::Cancelled(reason)
        };
        self.terminate(Some(failure))
    }

    pub(super) fn terminate(&mut self, failure: Option<continuation::Failure>) -> continuation::Progress {
        if let Some(error) = failure.as_ref().filter(|error| error.is_proof_error()) {
            return self.proof_failed(error.clone());
        }
        let final_state = match super::state::identity(&mut self.store) {
            Ok(reference) => reference,
            Err(error) => return self.proof_failed(error),
        };
        let resource_class = failure.as_ref().map_or("within-exact-profile", continuation::Failure::class).to_owned();
        let terminal = failure.map_or(receipt::Terminal::Finished, receipt::Terminal::Failed);
        let observation = receipt::Observation {
            terminal,
            output: std::mem::replace(&mut self.output, receipt::Output::Absent),
            hostcalls: std::mem::take(&mut *self.shared.calls.borrow_mut()),
            resource_class,
            final_state,
        };
        if let Err(error) = observation.validate() {
            return self.proof_failed(continuation::Failure::Proof(error));
        }
        let identity = match observation.identity() {
            Ok(identity) => identity,
            Err(error) => return self.proof_failed(continuation::Failure::Proof(error)),
        };
        if self.fuel == 0 {
            self.event("boundary", "no-instruction-execution");
        }
        self.event("resource", &observation.resource_class);
        self.event("terminal", &identity);
        self.terminal = true;
        self.release_guest();
        self.event("cleanup", "guest-code-stack-memory-released");
        continuation::Progress::Terminal(observation)
    }

    fn proof_failed(&mut self, error: continuation::Failure) -> continuation::Progress {
        self.event("resource", error.class());
        self.event("proof-failure", error.class());
        self.terminal = true;
        self.release_guest();
        self.event("cleanup", "guest-code-stack-memory-released");
        continuation::Progress::ProofFailed(error)
    }

    pub(super) fn release_guest(&mut self) {
        self.machine = None;
        while self.store.pop_module().is_some() {}
        while self.code.pop().is_some() {}
    }
}
