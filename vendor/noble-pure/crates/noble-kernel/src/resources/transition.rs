/// A total, allocation-free transition over one semantic record.
///
/// The table calls this with its retained snapshot and commits only successful
/// decisions. Calling it on a fabricated snapshot does not create an owner.
/// Every state/event branch is explicit, including duplicate terminal events.
// r[impl RA-STATE-01]
// r[impl RA-STATE-02]
// r[impl RA-STATE-03]
// r[impl RA-CLEAN-02]
// r[impl RA-CLEAN-03]
#[expect(
    tigerstyle::path_segment_repetition,
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the proof binds this exact production transition path; every untrusted record, identity and event is checked through typed errors, so assertion padding would change the total rejection contract."
)]
pub const fn transition(
    record: super::Snapshot,
    claim: super::Handle,
    event: super::Event,
) -> Result<super::Decision, super::Error> {
    if !valid_record(record) {
        return Err(super::Error::InvalidRecord);
    }
    match validate_handle(record.handle, claim) {
        Ok(()) => {}
        Err(error) => return Err(error),
    }
    match event {
        super::Event::Inspect(required) => inspect(record, required),
        super::Event::Begin { required, serial } => begin(record, required, serial),
        super::Event::Access { serial } => access(record, serial),
        super::Event::Complete { serial, result } => complete(record, serial, result),
        super::Event::Revoke { serial, reason } => revoke(record, serial, reason),
        super::Event::Retire(reason) => Ok(retire(record, reason)),
        super::Event::Release(required) => release(record, required),
        super::Event::Transfer {
            required,
            receiver,
            generation,
        } => transfer(record, required, receiver, generation),
    }
}

const fn valid_record(record: super::Snapshot) -> bool {
    if record.handle.generation == 0 {
        return false;
    }
    if matches!(record.last_scope, Some(0)) {
        return false;
    }
    match record.state {
        super::State::Live => record.retirement.is_none(),
        super::State::Busy(serial) => {
            require_scope(record, serial).is_ok() && record.retirement.is_none()
        }
        super::State::Retiring(serial) => {
            require_scope(record, serial).is_ok() && record.retirement.is_some()
        }
        super::State::Retired => true,
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; every retained/claimed handle field is independently rejected with a typed error; assertions would turn hostile input into a panic instead of total validation."
)]
const fn validate_handle(
    retained: super::Handle,
    claim: super::Handle,
) -> Result<(), super::Error> {
    if retained.table.0 != claim.table.0 {
        return Err(super::Error::InvalidHandle);
    }
    if retained.slot != claim.slot {
        return Err(super::Error::InvalidHandle);
    }
    if retained.context.0 != claim.context.0 {
        return Err(super::Error::WrongContext);
    }
    if retained.kind.0 != claim.kind.0 {
        return Err(super::Error::WrongKind);
    }
    if retained.generation != claim.generation {
        return Err(super::Error::WrongGeneration);
    }
    if retained.rights.0 != claim.rights.0 {
        return Err(super::Error::WrongRights);
    }
    Ok(())
}

const fn require_live(
    record: super::Snapshot,
    required: super::Requirement,
) -> Result<(), super::Error> {
    if record.handle.context.0 != required.context.0 {
        return Err(super::Error::WrongContext);
    }
    if record.handle.kind.0 != required.kind.0 {
        return Err(super::Error::WrongKind);
    }
    if record.handle.rights.0 & required.rights.0 != required.rights.0 {
        return Err(super::Error::WrongRights);
    }
    match record.state {
        super::State::Live => Ok(()),
        super::State::Busy(_) => Err(super::Error::Busy),
        super::State::Retiring(_) => Err(super::Error::Retiring),
        super::State::Retired => Err(super::Error::Retired),
    }
}

const fn inspect(
    record: super::Snapshot,
    required: super::Requirement,
) -> Result<super::Decision, super::Error> {
    match require_live(record, required) {
        Ok(()) => Ok(super::Decision {
            record,
            action: super::Action::Unchanged,
        }),
        Err(error) => Err(error),
    }
}

const fn begin(
    mut record: super::Snapshot,
    required: super::Requirement,
    serial: u64,
) -> Result<super::Decision, super::Error> {
    match require_live(record, required) {
        Ok(()) => {}
        Err(error) => return Err(error),
    }
    if serial == 0 {
        return Err(super::Error::WrongScope);
    }
    match record.last_scope {
        Some(previous) if serial <= previous => return Err(super::Error::WrongScope),
        Some(_) | None => {}
    }
    record.state = super::State::Busy(serial);
    record.last_scope = Some(serial);
    Ok(super::Decision {
        record,
        action: super::Action::BorrowAdmitted,
    })
}

const fn require_scope(record: super::Snapshot, serial: u64) -> Result<(), super::Error> {
    if serial == 0 {
        return Err(super::Error::WrongScope);
    }
    match record.last_scope {
        Some(previous) if previous == serial => Ok(()),
        Some(_) | None => Err(super::Error::WrongScope),
    }
}

const fn access(record: super::Snapshot, serial: u64) -> Result<super::Decision, super::Error> {
    match require_scope(record, serial) {
        Ok(()) => {}
        Err(error) => return Err(error),
    }
    match record.state {
        super::State::Live => Err(super::Error::WrongScope),
        super::State::Busy(_) => Ok(super::Decision {
            record,
            action: super::Action::Unchanged,
        }),
        super::State::Retiring(_) => Err(super::Error::Retiring),
        super::State::Retired => Err(super::Error::Retired),
    }
}

const fn complete(
    mut record: super::Snapshot,
    serial: u64,
    result: super::Completion,
) -> Result<super::Decision, super::Error> {
    match require_scope(record, serial) {
        Ok(()) => {}
        Err(error) => return Err(error),
    }
    let action = match record.state {
        super::State::Live => super::Action::Unchanged,
        super::State::Busy(_) => {
            record.state = super::State::Live;
            super::Action::OwnerReturned(result)
        }
        super::State::Retiring(_) => {
            record.state = super::State::Retired;
            super::Action::RetirementCompleted
        }
        super::State::Retired => super::Action::Unchanged,
    };
    Ok(super::Decision { record, action })
}

const fn revoke(
    record: super::Snapshot,
    serial: u64,
    reason: super::Retirement,
) -> Result<super::Decision, super::Error> {
    match require_scope(record, serial) {
        Ok(()) => {}
        Err(error) => return Err(error),
    }
    match record.state {
        super::State::Live | super::State::Retired => Ok(super::Decision {
            record,
            action: super::Action::Unchanged,
        }),
        super::State::Busy(_) | super::State::Retiring(_) => Ok(retire(record, reason)),
    }
}

const fn retire(mut record: super::Snapshot, reason: super::Retirement) -> super::Decision {
    let action = match record.state {
        super::State::Live => {
            record.state = super::State::Retired;
            record.retirement = Some(reason);
            super::Action::LocalRelease
        }
        super::State::Busy(serial) => {
            record.state = super::State::Retiring(serial);
            record.retirement = Some(reason);
            super::Action::AccessRevoked
        }
        super::State::Retiring(_) => super::Action::Unchanged,
        super::State::Retired => super::Action::Unchanged,
    };
    super::Decision { record, action }
}

const fn release(
    mut record: super::Snapshot,
    required: super::Requirement,
) -> Result<super::Decision, super::Error> {
    match require_live(record, required) {
        Ok(()) => {}
        Err(error) => return Err(error),
    }
    record.state = super::State::Retired;
    Ok(super::Decision {
        record,
        action: super::Action::LocalRelease,
    })
}

const fn transfer(
    mut record: super::Snapshot,
    required: super::Requirement,
    receiver: super::Context,
    generation: u64,
) -> Result<super::Decision, super::Error> {
    match require_live(record, required) {
        Ok(()) => {}
        Err(error) => return Err(error),
    }
    if generation <= record.handle.generation {
        return Err(super::Error::WrongGeneration);
    }
    record.handle.context = receiver;
    record.handle.generation = generation;
    record.last_scope = None;
    Ok(super::Decision {
        record,
        action: super::Action::OwnerTransferred,
    })
}
