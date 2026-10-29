/// Retains the upstream store/future. Dropping this value cancels execution.
pub(crate) struct PendingInvocation<'a> {
    request: &'a super::super::shell::ExecutionRequest<'a>,
    admitted: super::super::shell::AdmittedRequest,
    receipts: Vec<super::super::super::evidence::receipt::ExecutionRecord>,
    invocation: Option<hardened_wasmtime_shell::ResumableInvocation>,
    ledger: super::super::super::slicing::SliceLedger,
    continuation: super::super::super::slicing::ContinuationKey,
}

impl super::EngineAdapter {
    pub(crate) fn start_sliced<'a>(
        &self,
        request: &'a super::super::shell::ExecutionRequest<'a>,
        continuation: super::super::super::slicing::ContinuationKey,
        deadline_ticks: u64,
    ) -> Result<PendingInvocation<'a>, super::super::shell::ExecutionOutcome> {
        let early_denial = |denial| super::denied_outcome(request, denial);
        if !request.requested_profile.requests_time_slicing() {
            return Err(early_denial(super::super::super::model::Denial::classified(
                super::super::super::model::DenialClass::ProfileDenial,
                "a sliced execution needs a time-sliced execution request",
            )));
        }
        let mut admitted = super::admit(request).map_err(early_denial)?;
        let slicing = request
            .profile
            .slicing
            .as_ref()
            .ok_or_else(|| early_denial(resource_denial("missing slicing profile")))?;
        let ledger = super::super::super::slicing::SliceLedger::new(slicing.quantum_fuel, slicing.total_budget_fuel)
            .map_err(|issue| early_denial(resource_denial(format!("slice ledger denied: {issue:?}"))))?;
        let inspection =
            super::stage(request, &admitted, super::super::super::evidence::receipt::Stage::Inspection, Vec::new())
                .map_err(early_denial)?;
        let prepared = match self.prepare(request, deadline_ticks) {
            Ok(prepared) => prepared,
            Err(denial) => {
                return Err(super::denied(request, &admitted, denial, vec![inspection]).unwrap_or_else(early_denial));
            }
        };
        let prepared = std::sync::Arc::new(prepared);
        let mut invocation = match prepared.start_sliced(
            hardened_wasmtime_shell::SliceOptions {
                export: super::super::super::admission::COMPONENT_INVOKE_EXPORT.to_string(),
                input: std::mem::take(&mut admitted.input_bytes),
                quantum_fuel: slicing.quantum_fuel,
            },
            (),
            |_, _| Err(hardened_wasmtime_shell::HostDenied),
        ) {
            Ok(invocation) => invocation,
            Err(observation) => {
                return Err(super::denied(request, &admitted, super::observation_denial(&observation), vec![
                    inspection,
                ])
                .unwrap_or_else(early_denial));
            }
        };
        if let Err(denial) = initialize(&mut invocation, &ledger) {
            return Err(super::denied(request, &admitted, denial, vec![inspection]).unwrap_or_else(early_denial));
        }
        Ok(PendingInvocation {
            request,
            admitted,
            receipts: vec![inspection],
            invocation: Some(invocation),
            ledger,
            continuation,
        })
    }
}

fn resource_denial(detail: impl Into<String>) -> super::super::super::model::Denial {
    super::super::super::model::Denial::classified(super::super::super::model::DenialClass::ResourceDenial, detail)
}

// Match the local shell: finish bounded initialization before exposing callback polls.
fn initialize(
    invocation: &mut hardened_wasmtime_shell::ResumableInvocation,
    ledger: &super::super::super::slicing::SliceLedger,
) -> super::super::super::model::Result<()> {
    let polls = ledger
        .total_budget_fuel
        .checked_div(ledger.quantum_fuel)
        .and_then(|slices| slices.checked_add(1))
        .ok_or_else(|| resource_denial("the slice budget admits no instantiation polls"))?;
    for _ in 0..polls {
        match invocation
            .run_slice()
            .map_err(|issue| resource_denial(format!("retained initialization denied: {issue:?}")))?
        {
            hardened_wasmtime_shell::SliceStep::Yielded(observation)
                if observation.phase == hardened_wasmtime_runtime::outcomes::InvocationPhase::Instantiation => {}
            hardened_wasmtime_shell::SliceStep::Instantiated(_) => return Ok(()),
            hardened_wasmtime_shell::SliceStep::Completed(observation) => {
                return Err(super::observation_denial(&observation));
            }
            hardened_wasmtime_shell::SliceStep::Yielded(_) => {
                return Err(resource_denial("callback work preceded the instantiation checkpoint"));
            }
        }
    }
    Err(resource_denial("component instantiation did not complete within the slice budget"))
}

impl PendingInvocation<'_> {
    pub(crate) fn ledger(&self) -> &super::super::super::slicing::SliceLedger {
        &self.ledger
    }

    pub(crate) fn run_slice(&mut self) -> super::super::slicedshell::SliceProgress {
        let Some(invocation) = self.invocation.as_mut() else {
            return super::super::slicedshell::SliceProgress::Completed(super::denied_outcome(
                self.request,
                super::super::super::model::Denial::new("the sliced execution already completed"),
            ));
        };
        match invocation.run_slice() {
            Ok(hardened_wasmtime_shell::SliceStep::Yielded(observation))
                if observation.phase == hardened_wasmtime_runtime::outcomes::InvocationPhase::Invocation =>
            {
                // Configured quantum observations are not measured per-poll fuel.
                // Preserve the local ledger convention, with terminal fuel readback.
                match self.ledger.record_boundary(observation.quantum_fuel) {
                    Ok(boundary) => super::super::slicedshell::SliceProgress::Yielded {
                        boundary,
                        requeue: self.continuation.requeue_command(),
                    },
                    Err(issue) => self.fail(resource_denial(format!("slice boundary denied: {issue:?}"))),
                }
            }
            Ok(hardened_wasmtime_shell::SliceStep::Completed(observation)) => {
                self.invocation = None;
                if observation.class == hardened_wasmtime_runtime::outcomes::Class::Completed {
                    let final_slice = self
                        .ledger
                        .total_budget_fuel
                        .checked_sub(observation.fuel_remaining)
                        .and_then(|consumed| consumed.checked_sub(self.ledger.consumed_fuel));
                    let Some(final_slice) = final_slice else {
                        return self.fail(resource_denial("the store spent less fuel than the recorded slices"));
                    };
                    if let Err(issue) = self.ledger.record_boundary(final_slice) {
                        return self.fail(resource_denial(format!("final slice denied: {issue:?}")));
                    }
                }
                let outcome =
                    super::finish(self.request, &self.admitted, std::mem::take(&mut self.receipts), observation)
                        .unwrap_or_else(|denial| super::denied_outcome(self.request, denial));
                super::super::slicedshell::SliceProgress::Completed(outcome)
            }
            Ok(
                hardened_wasmtime_shell::SliceStep::Instantiated(_) | hardened_wasmtime_shell::SliceStep::Yielded(_),
            ) => self.fail(resource_denial("unexpected instantiation observation after the checkpoint")),
            Err(issue) => self.fail(resource_denial(format!("retained shared invocation denied: {issue:?}"))),
        }
    }

    fn fail(&mut self, denial: super::super::super::model::Denial) -> super::super::slicedshell::SliceProgress {
        self.invocation = None;
        let outcome = super::denied(self.request, &self.admitted, denial, std::mem::take(&mut self.receipts))
            .unwrap_or_else(|failure| super::denied_outcome(self.request, failure));
        super::super::slicedshell::SliceProgress::Completed(outcome)
    }
}
