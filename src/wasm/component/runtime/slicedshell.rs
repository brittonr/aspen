// The time-sliced component execution shell.
//
// A sliced execution passes the same admission boundaries and produces the
// same inspection, instantiation, and execution receipts as a synchronous
// execution. Between slices the caller's admitted scheduler runs other work:
// each yield returns the command that re-enqueues the continuation. The turn
// publishes nothing until the guest returns, and publishes exactly once.

/// What one slice of a sliced execution did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceProgress {
    /// The guest consumed a quantum; re-enqueue the continuation with `requeue`.
    Yielded {
        boundary: super::super::slicing::SliceBoundary,
        requeue: molten_core::fabric_time::SchedulerCommand,
    },
    /// The execution ended with its final outcome.
    Completed(super::shell::ExecutionOutcome),
}

/// A time-sliced component execution in progress.
pub struct SlicedExecution<'a> {
    request: &'a super::shell::ExecutionRequest<'a>,
    admitted: super::shell::AdmittedRequest,
    receipts: Vec<super::super::evidence::receipt::ExecutionRecord>,
    invocation: Option<super::sliced::Invocation>,
    final_ledger: Option<super::super::slicing::SliceLedger>,
    continuation: super::super::slicing::ContinuationKey,
}

/// Admit and instantiate a time-sliced execution without running guest code.
///
/// A request that does not ask for time slicing, or a profile that does not
/// admit it, is denied with the same outcome a synchronous execution reports.
// r[impl molten.wasm_component.execution_quantum]
pub fn start_sliced_execution<'a>(
    request: &'a super::shell::ExecutionRequest<'a>,
    continuation: super::super::slicing::ContinuationKey,
) -> Result<SlicedExecution<'a>, super::shell::ExecutionOutcome> {
    if !request.requested_profile.requests_time_slicing() {
        let denial = super::super::model::Denial::classified(
            super::super::model::DenialClass::ProfileDenial,
            "a sliced execution needs a time-sliced execution request",
        );
        return Err(super::denial::denied_outcome(request, denial));
    }
    let admitted = super::shell::admit_request(request, super::provenance::Backend::Local)
        .map_err(|denial| super::denial::denied_outcome(request, denial))?;
    let receipt = |stage, parent_refs| {
        super::shell::plan_receipt(request, super::shell::PlanReceiptInput {
            plan: &admitted.plan,
            stage,
            input_ref: None,
            output_ref: None,
            fuel_limit: None,
            fuel_remaining: None,
            parent_refs,
        })
    };
    let inspection = receipt(super::super::evidence::receipt::Stage::Inspection, Vec::new())
        .map_err(|denial| super::denial::denied_outcome(request, denial))?;
    let invocation = match super::sliced::instantiate(
        request.profile,
        request.source.component_bytes(),
        request.facts,
        &admitted.input_bytes,
    ) {
        Ok(invocation) => invocation,
        Err(denial) => return Err(denied(request, &admitted, denial, vec![inspection])),
    };
    let instantiation =
        receipt(super::super::evidence::receipt::Stage::Instantiation, vec![inspection.receipt_ref.clone()])
            .map_err(|denial| super::denial::denied_outcome(request, denial))?;
    Ok(SlicedExecution {
        request,
        admitted,
        receipts: vec![inspection, instantiation],
        invocation: Some(invocation),
        final_ledger: None,
        continuation,
    })
}

fn denied(
    request: &super::shell::ExecutionRequest<'_>,
    admitted: &super::shell::AdmittedRequest,
    denial: super::super::model::Denial,
    receipts: Vec<super::super::evidence::receipt::ExecutionRecord>,
) -> super::shell::ExecutionOutcome {
    super::denial::plan_denied_outcome(request, &admitted.plan, denial, receipts, Some(admitted.input_ref.clone()))
        .unwrap_or_else(|failure| super::denial::denied_outcome(request, failure))
}

impl SlicedExecution<'_> {
    /// The quantum boundaries recorded so far.
    pub fn ledger(&self) -> Option<&super::super::slicing::SliceLedger> {
        self.invocation.as_ref().map(super::sliced::Invocation::ledger).or(self.final_ledger.as_ref())
    }

    /// Run one slice of the execution.
    ///
    /// A yield returns the requeue command for the admitted scheduler. The
    /// guest's return completes the turn exactly once; a budget exhaustion at
    /// any slice classifies as the existing fuel-exhaustion denial.
    // r[impl molten.wasm_component.yield_continuation]
    pub fn run_slice(&mut self) -> SliceProgress {
        let Some(invocation) = self.invocation.as_mut() else {
            let denial = super::super::model::Denial::new("the sliced execution already completed");
            return SliceProgress::Completed(super::denial::denied_outcome(self.request, denial));
        };
        let step = invocation.run_slice();
        if matches!(step, Ok(super::sliced::SliceStep::Completed(_)) | Err(_)) {
            self.final_ledger = Some(invocation.ledger().clone());
        }
        match step {
            Ok(super::sliced::SliceStep::Yielded(boundary)) => SliceProgress::Yielded {
                boundary,
                requeue: self.continuation.requeue_command(),
            },
            Ok(super::sliced::SliceStep::Completed(result)) => {
                self.invocation = None;
                SliceProgress::Completed(self.complete(result))
            }
            Err(denial) => {
                self.invocation = None;
                SliceProgress::Completed(denied(self.request, &self.admitted, denial, self.receipts.clone()))
            }
        }
    }

    fn complete(&self, result: super::super::model::Result<super::Execution>) -> super::shell::ExecutionOutcome {
        let request = self.request;
        let decoded = result.and_then(|execution| {
            super::shell::decode_output(request.profile, &execution.output_bytes).map(|decoded| (decoded, execution))
        });
        let ((output, output_ref), execution) = match decoded {
            Ok(decoded) => decoded,
            Err(denial) => return denied(request, &self.admitted, denial, self.receipts.clone()),
        };
        let parent_refs = self.receipts.last().map(|receipt| receipt.receipt_ref.clone()).into_iter().collect();
        let receipt = super::shell::plan_receipt(request, super::shell::PlanReceiptInput {
            plan: &self.admitted.plan,
            stage: super::super::evidence::receipt::Stage::Execution,
            input_ref: Some(self.admitted.input_ref.clone()),
            output_ref: Some(output_ref),
            fuel_limit: Some(request.profile.resources.fuel),
            fuel_remaining: Some(execution.fuel_remaining),
            parent_refs,
        });
        let receipt = match receipt {
            Ok(receipt) => receipt,
            Err(denial) => return denied(request, &self.admitted, denial, self.receipts.clone()),
        };
        let mut receipts = self.receipts.clone();
        receipts.push(receipt);
        let turn = super::super::slicing::PendingTurn::new(super::shell::ExecutionOutcome {
            decision: super::super::evidence::receipt::Decision::Pass,
            output: Some(output),
            receipts,
            diagnostics: Vec::new(),
        });
        match turn.publish() {
            Ok(outcome) => outcome,
            Err(issue) => {
                let denial = super::super::model::Denial::new(format!("the completed turn did not publish: {issue:?}"));
                denied(request, &self.admitted, denial, self.receipts.clone())
            }
        }
    }
}
