pub(super) const fn pins(
    mut record: crate::async_tasks::Snapshot,
    pins: u64,
) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
    if !record.native_stopped {
        return Err(crate::async_tasks::Error::NativeStillRunning);
    }
    if pins == 0 || pins & !record.pins != 0 {
        return Err(crate::async_tasks::Error::InvalidPins);
    }
    record.pins &= !pins;
    let mut decision =
        crate::async_tasks::Decision::unchanged(record, crate::async_tasks::Action::PinsSettled);
    decision.accounting.pins_released = pins;
    Ok(decision)
}

/// Cleanup acknowledges named completed work. Native access must have ended
/// first; resource-bearing Ready results remain ineligible until cancellation.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; settle checks native stop, outstanding pins and exact cleanup eligibility in order, returning typed refusals before mutating the copied record; hostile cleanup claims must not panic."
)]
pub(super) const fn settle(
    mut record: crate::async_tasks::Snapshot,
    settled: crate::async_tasks::Obligations,
) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
    if !record.native_stopped {
        return Err(crate::async_tasks::Error::NativeStillRunning);
    }
    if record.pins != 0 {
        return Err(crate::async_tasks::Error::PinsOutstanding);
    }
    let available = match record.state {
        crate::async_tasks::State::Ready => crate::async_tasks::Obligations {
            inputs: record.retiring_inputs,
            results: 0,
            buffers: record.buffers & !2,
        },
        crate::async_tasks::State::Delivered | crate::async_tasks::State::Retiring => {
            crate::async_tasks::Obligations {
                inputs: record.retiring_inputs,
                results: record.results,
                buffers: record.buffers,
            }
        }
        crate::async_tasks::State::Pending | crate::async_tasks::State::Retired => {
            return Err(crate::async_tasks::Error::InvalidCleanup)
        }
    };
    if settled.inputs == 0 && settled.results == 0 && settled.buffers == 0 {
        return Err(crate::async_tasks::Error::InvalidCleanup);
    }
    if settled.inputs & !available.inputs != 0
        || settled.results & !available.results != 0
        || settled.buffers & !available.buffers != 0
    {
        return Err(crate::async_tasks::Error::InvalidCleanup);
    }
    record.retiring_inputs &= !settled.inputs;
    record.results &= !settled.results;
    record.buffers &= !settled.buffers;
    let mut decision =
        crate::async_tasks::Decision::unchanged(record, crate::async_tasks::Action::CleanupSettled);
    decision.accounting.cleaned = settled;
    Ok(decision)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; finish preserves duplicate completion and native-stop, pin and cleanup refusal precedence before releasing the reservation; unsettled tasks are typed outcomes rather than assertion failures."
)]
pub(super) const fn finish(
    mut record: crate::async_tasks::Snapshot,
) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
    if record.finalized {
        return Ok(crate::async_tasks::Decision::unchanged(
            record,
            crate::async_tasks::Action::Duplicate,
        ));
    }
    if !record.native_stopped {
        return Err(crate::async_tasks::Error::NativeStillRunning);
    }
    if record.pins != 0 {
        return Err(crate::async_tasks::Error::PinsOutstanding);
    }
    if !super::validation::settled(record) {
        return Err(crate::async_tasks::Error::CleanupOutstanding);
    }
    let action = match record.state {
        crate::async_tasks::State::Delivered => crate::async_tasks::Action::DeliverySettled,
        crate::async_tasks::State::Retiring => {
            record.state = crate::async_tasks::State::Retired;
            crate::async_tasks::Action::RetirementCompleted
        }
        crate::async_tasks::State::Pending
        | crate::async_tasks::State::Ready
        | crate::async_tasks::State::Retired => {
            return Err(crate::async_tasks::Error::InvalidRecord)
        }
    };
    record.finalized = true;
    let mut decision = crate::async_tasks::Decision::unchanged(record, action);
    decision.accounting.reservation_released = true;
    Ok(decision)
}
