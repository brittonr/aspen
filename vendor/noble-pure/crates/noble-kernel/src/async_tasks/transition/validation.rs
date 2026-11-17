pub(super) const fn handle(
    retained: crate::async_tasks::Handle,
    claim: crate::async_tasks::Handle,
) -> Result<(), crate::async_tasks::Error> {
    if retained.table.0 != claim.table.0 || retained.slot != claim.slot {
        return Err(crate::async_tasks::Error::InvalidHandle);
    }
    if retained.context.0 != claim.context.0 {
        return Err(crate::async_tasks::Error::WrongContext);
    }
    if retained.generation != claim.generation {
        return Err(crate::async_tasks::Error::WrongGeneration);
    }
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; native explicitly classifies all event constructors and returns WrongNative for mismatched callback identities; untrusted identities must reject rather than trigger assertions."
)]
pub(super) const fn native(
    retained: crate::async_tasks::NativeId,
    event: crate::async_tasks::Event,
) -> Result<(), crate::async_tasks::Error> {
    let supplied = match event {
        crate::async_tasks::Event::CompleteSuccess {
            native,
            completion: _,
        }
        | crate::async_tasks::Event::CompleteDomainError {
            native,
            completion: _,
        }
        | crate::async_tasks::Event::NativeStopped { native }
        | crate::async_tasks::Event::SettlePins { native, pins: _ }
        | crate::async_tasks::Event::Wake { native } => Some(native),
        crate::async_tasks::Event::Inspect
        | crate::async_tasks::Event::Deliver
        | crate::async_tasks::Event::Cancel
        | crate::async_tasks::Event::Trap
        | crate::async_tasks::Event::Deadline
        | crate::async_tasks::Event::Budget
        | crate::async_tasks::Event::InternalFailure
        | crate::async_tasks::Event::Cleanup(_)
        | crate::async_tasks::Event::TakeWake
        | crate::async_tasks::Event::Finish => None,
    };
    match supplied {
        Some(supplied) => {
            if supplied.0 != retained.0 {
                return Err(crate::async_tasks::Error::WrongNative);
            }
            Ok(())
        }
        None => Ok(()),
    }
}

pub(super) const fn disposition(
    record: crate::async_tasks::Snapshot,
    completion: crate::async_tasks::Completion,
) -> Result<(), crate::async_tasks::Error> {
    let inputs = completion.inputs;
    if inputs.returned & inputs.consumed != 0
        || inputs.returned & inputs.retired != 0
        || inputs.consumed & inputs.retired != 0
    {
        return Err(crate::async_tasks::Error::InvalidDisposition);
    }
    let all = crate::async_tasks::bounds::mask(record.reservation.inputs);
    if inputs.returned | inputs.consumed | inputs.retired != all {
        return Err(crate::async_tasks::Error::InvalidDisposition);
    }
    let results = crate::async_tasks::bounds::mask(record.reservation.results);
    if completion.produced & !results != 0 {
        return Err(crate::async_tasks::Error::ResultCapacity);
    }
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; record validates reservation, identity, ownership and lifecycle invariants in order and returns InvalidRecord for fabricated snapshots; asserting these external preconditions would change the total rejection contract."
)]
pub(super) const fn record(
    record: crate::async_tasks::Snapshot,
) -> Result<(), crate::async_tasks::Error> {
    if record.reservation.footprint().is_err() {
        return Err(crate::async_tasks::Error::InvalidRecord);
    }
    if record.handle.generation == 0 || record.handle.slot >= crate::async_tasks::MAX_SLOTS {
        return Err(crate::async_tasks::Error::InvalidRecord);
    }
    if record.native.0 != record.reservation.native.0
        || record.handle.context.0 != record.reservation.context.0
    {
        return Err(crate::async_tasks::Error::InvalidRecord);
    }
    if record.pins & !crate::async_tasks::bounds::mask(record.reservation.pins) != 0
        || record.wakeups_remaining > record.reservation.wakeups
        || record.buffers & !7 != 0
    {
        return Err(crate::async_tasks::Error::InvalidRecord);
    }
    if record.buffers & 1 != 0 && record.reservation.input_bytes == 0 {
        return Err(crate::async_tasks::Error::InvalidRecord);
    }
    if record.buffers & 4 != 0 && record.reservation.parked_bytes == 0 {
        return Err(crate::async_tasks::Error::InvalidRecord);
    }
    if record.returned_inputs & record.retiring_inputs != 0 {
        return Err(crate::async_tasks::Error::InvalidRecord);
    }
    if !owners_valid(record) || !state_valid(record) {
        return Err(crate::async_tasks::Error::InvalidRecord);
    }
    if record.finalized && (!settled(record) || record.wake_pending) {
        return Err(crate::async_tasks::Error::InvalidRecord);
    }
    Ok(())
}

const fn owners_valid(record: crate::async_tasks::Snapshot) -> bool {
    match record.completion {
        Some(completion) => {
            if disposition(record, completion).is_err()
                || !record.completion_closed
                || record.outcome.is_none()
            {
                return false;
            }
            if record.returned_inputs & !completion.inputs.returned != 0
                || record.retiring_inputs
                    & !(completion.inputs.retired | completion.inputs.returned)
                    != 0
                || record.results & !completion.produced != 0
            {
                return false;
            }
            if record.buffers & 2 != 0
                && (completion.bytes == 0 || completion.bytes > record.reservation.result_bytes)
            {
                return false;
            }
            true
        }
        None => {
            record.outcome.is_none()
                && record.returned_inputs == 0
                && record.results == 0
                && record.buffers & 2 == 0
                && record.retiring_inputs
                    & !crate::async_tasks::bounds::mask(record.reservation.inputs)
                    == 0
                && (!record.completion_closed || record.native_stopped)
        }
    }
}

const fn state_valid(record: crate::async_tasks::Snapshot) -> bool {
    match record.state {
        crate::async_tasks::State::Pending => {
            record.failure.is_none()
                && record.completion.is_none()
                && !record.completion_closed
                && !record.finalized
                && record.retiring_inputs == 0
        }
        crate::async_tasks::State::Ready => {
            let completion = match record.completion {
                Some(value) => value,
                None => return false,
            };
            record.failure.is_none()
                && !record.finalized
                && completion.bytes <= record.reservation.result_bytes
                && record.returned_inputs == completion.inputs.returned
                && record.results == completion.produced
                && record.retiring_inputs & !completion.inputs.retired == 0
                && (record.buffers & 2 != 0) == (completion.bytes != 0)
        }
        crate::async_tasks::State::Delivered => {
            record.failure.is_none()
                && record.completion.is_some()
                && record.returned_inputs == 0
                && record.results == 0
                && record.buffers & 2 == 0
        }
        crate::async_tasks::State::Retiring => {
            record.failure.is_some()
                && !record.finalized
                && record.returned_inputs == 0
                && !record.wake_pending
        }
        crate::async_tasks::State::Retired => {
            record.failure.is_some()
                && record.finalized
                && record.completion_closed
                && settled(record)
                && !record.wake_pending
        }
    }
}

pub(super) const fn settled(record: crate::async_tasks::Snapshot) -> bool {
    record.native_stopped
        && record.pins == 0
        && record.retiring_inputs == 0
        && record.returned_inputs == 0
        && record.results == 0
        && record.buffers == 0
}
