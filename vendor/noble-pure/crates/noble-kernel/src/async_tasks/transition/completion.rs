/// The shell checks metadata here before storing result bytes. An oversized
/// payload is never admitted: the returned byte count remains the shell's
/// rejection obligation, while bounded result owners enter task retirement.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; complete preserves duplicate precedence, rejects invalid ownership dispositions and retires oversized results through explicit accounting; arbitrary callback metadata must not introduce assertion panics."
)]
pub(super) const fn complete(
    mut record: crate::async_tasks::Snapshot,
    outcome: crate::async_tasks::Outcome,
    completion: crate::async_tasks::Completion,
) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
    if record.completion_closed {
        return Ok(crate::async_tasks::Decision::unchanged(
            record,
            crate::async_tasks::Action::Duplicate,
        ));
    }
    attempt!(super::validation::disposition(record, completion));
    let prior_retiring = record.retiring_inputs;
    let is_oversized = exceeds_limit(completion, record.reservation.result_bytes);
    let was_retiring = match record.state {
        crate::async_tasks::State::Retiring => true,
        crate::async_tasks::State::Pending
        | crate::async_tasks::State::Ready
        | crate::async_tasks::State::Delivered
        | crate::async_tasks::State::Retired => false,
    };
    let mut accounting = crate::async_tasks::Accounting::empty();
    accounting.acquired.results = completion.produced;
    accounting.completion_accepted = true;
    accounting.consumed_inputs = completion.inputs.consumed;
    accounting.retirement_withdrawn_inputs = prior_retiring & completion.inputs.consumed;
    record.completion = Some(completion);
    record.outcome = Some(outcome);
    record.completion_closed = true;
    record.results = completion.produced;
    record.retiring_inputs = completion.inputs.retired;
    if completion.bytes != 0 && !is_oversized {
        record.buffers |= 2;
        accounting.acquired.buffers = 2;
    }
    let action = if was_retiring || is_oversized {
        record.retiring_inputs |= completion.inputs.returned;
        record.returned_inputs = 0;
        record.state = crate::async_tasks::State::Retiring;
        accounting.retired.results = completion.produced;
        accounting.retired.buffers = record.buffers & 2;
        accounting.wake_removed = record.wake_pending;
        record.wake_pending = false;
        if is_oversized {
            if record.failure.is_none() {
                record.failure = Some(crate::async_tasks::Failure::Budget);
            }
            accounting.rejected_result_bytes = completion.bytes;
            crate::async_tasks::Action::OversizedResult
        } else {
            crate::async_tasks::Action::CompletionRetired(outcome)
        }
    } else {
        record.state = crate::async_tasks::State::Ready;
        record.returned_inputs = completion.inputs.returned;
        crate::async_tasks::Action::ResultReady(outcome)
    };
    accounting.retired.inputs = record.retiring_inputs & !prior_retiring;
    // A result callback does not establish native stop and releases no pin.
    Ok(crate::async_tasks::Decision {
        record,
        action,
        accounting,
    })
}

// Keep the extracted predicate explicitly Bool-valued across both uses. The
// pinned translator otherwise infers a Lean Prop let without its Decidable.
const fn exceeds_limit(completion: crate::async_tasks::Completion, result_bytes: usize) -> bool {
    completion.bytes > result_bytes
}
