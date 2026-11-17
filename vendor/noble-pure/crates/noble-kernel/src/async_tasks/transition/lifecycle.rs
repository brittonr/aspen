#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; transition validates the record and classifies retirement eligibility before this ownership-mask transformation; failure is recorded as an outcome, and assertion padding would add panic paths to the proved decision core."
)]
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; every Failure constructor must explicitly choose cancellation acknowledgment or failure recording; new failures must require review of their observable retirement action."
)]
pub(super) const fn retire(
    mut record: crate::async_tasks::Snapshot,
    reason: crate::async_tasks::Failure,
) -> crate::async_tasks::Decision {
    let mut accounting = crate::async_tasks::Accounting::empty();
    let inputs = match record.state {
        crate::async_tasks::State::Pending => {
            crate::async_tasks::bounds::mask(record.reservation.inputs)
        }
        crate::async_tasks::State::Ready => record.returned_inputs,
        crate::async_tasks::State::Delivered
        | crate::async_tasks::State::Retiring
        | crate::async_tasks::State::Retired => 0,
    };
    accounting.retired = crate::async_tasks::Obligations {
        inputs,
        results: record.results,
        buffers: record.buffers & 2,
    };
    accounting.wake_removed = record.wake_pending;
    record.retiring_inputs |= inputs;
    record.returned_inputs = 0;
    record.wake_pending = false;
    record.failure = Some(reason);
    record.state = crate::async_tasks::State::Retiring;
    if record.native_stopped {
        record.completion_closed = true;
    }
    let action = match reason {
        crate::async_tasks::Failure::Cancelled => {
            crate::async_tasks::Action::CancellationAcknowledged
        }
        crate::async_tasks::Failure::Trap
        | crate::async_tasks::Failure::Deadline
        | crate::async_tasks::Failure::Budget
        | crate::async_tasks::Failure::Internal => {
            crate::async_tasks::Action::FailureRecorded(reason)
        }
    };
    crate::async_tasks::Decision {
        record,
        action,
        accounting,
    }
}

pub(super) const fn deliver(
    mut record: crate::async_tasks::Snapshot,
) -> crate::async_tasks::Decision {
    let mut accounting = crate::async_tasks::Accounting::empty();
    accounting.delivered = crate::async_tasks::Obligations {
        inputs: record.returned_inputs,
        results: record.results,
        buffers: record.buffers & 2,
    };
    accounting.wake_removed = record.wake_pending;
    record.returned_inputs = 0;
    record.results = 0;
    record.buffers &= !2;
    record.wake_pending = false;
    record.state = crate::async_tasks::State::Delivered;
    crate::async_tasks::Decision {
        record,
        action: crate::async_tasks::Action::ResultDelivered,
        accounting,
    }
}

pub(super) const fn native_stop(
    mut record: crate::async_tasks::Snapshot,
) -> crate::async_tasks::Decision {
    if record.native_stopped {
        return crate::async_tasks::Decision::unchanged(
            record,
            crate::async_tasks::Action::Duplicate,
        );
    }
    record.native_stopped = true;
    match record.state {
        crate::async_tasks::State::Retiring | crate::async_tasks::State::Retired => {
            record.completion_closed = true
        }
        crate::async_tasks::State::Pending
        | crate::async_tasks::State::Ready
        | crate::async_tasks::State::Delivered => {}
    }
    crate::async_tasks::Decision::unchanged(record, crate::async_tasks::Action::NativeStopObserved)
}

pub(super) const fn wake(mut record: crate::async_tasks::Snapshot) -> crate::async_tasks::Decision {
    if record.wake_pending {
        return crate::async_tasks::Decision::unchanged(
            record,
            crate::async_tasks::Action::WakeCoalesced,
        );
    }
    if record.wakeups_remaining == 0 {
        return retire(record, crate::async_tasks::Failure::Budget);
    }
    record.wakeups_remaining -= 1;
    record.wake_pending = true;
    let mut decision =
        crate::async_tasks::Decision::unchanged(record, crate::async_tasks::Action::WakeQueued);
    decision.accounting.wake_queued = true;
    decision
}

pub(super) const fn take_wake(
    mut record: crate::async_tasks::Snapshot,
) -> Result<crate::async_tasks::Decision, crate::async_tasks::Error> {
    if !record.wake_pending {
        return Err(crate::async_tasks::Error::NoWakeup);
    }
    record.wake_pending = false;
    let mut decision =
        crate::async_tasks::Decision::unchanged(record, crate::async_tasks::Action::WakeTaken);
    decision.accounting.wake_removed = true;
    Ok(decision)
}
