mod coverage;
pub use coverage::{validate_coverage, CoverageError, CoverageRow};

/// Stable resolved schema identities for schema-bound coverage admission.
pub const STATE_SCHEMA: &str = concat!(
    "noble-kernel::async_tasks::domain::State{",
    "Pending;Ready;Delivered;Retiring;Retired}"
);
pub const EVENT_SCHEMA: &str = concat!(
    "noble-kernel::async_tasks::domain::Event{",
    "Inspect;CompleteSuccess{native:NativeId,completion:Completion};",
    "CompleteDomainError{native:NativeId,completion:Completion};Deliver;Cancel;",
    "Trap;Deadline;Budget;InternalFailure;NativeStopped{native:NativeId};",
    "SettlePins{native:NativeId,pins:u64};Cleanup(Obligations);",
    "Wake{native:NativeId};TakeWake;Finish};",
    "noble-kernel::async_tasks::domain::NativeId(u64);",
    "noble-kernel::async_tasks::domain::Completion{inputs:Disposition,produced:u64,bytes:usize};",
    "noble-kernel::async_tasks::domain::Disposition{returned:u64,consumed:u64,retired:u64};",
    "noble-kernel::async_tasks::domain::Obligations{inputs:u64,results:u64,buffers:u8}"
);
pub const STATE_CONSTRUCTORS: [super::State; 5] = [
    super::State::Pending,
    super::State::Ready,
    super::State::Delivered,
    super::State::Retiring,
    super::State::Retired,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Inspect,
    CompleteSuccess,
    CompleteDomainError,
    Deliver,
    Cancel,
    Trap,
    Deadline,
    Budget,
    InternalFailure,
    NativeStopped,
    SettlePins,
    Cleanup,
    Wake,
    TakeWake,
    Finish,
}

pub const EVENT_CONSTRUCTORS: [EventKind; 15] = [
    EventKind::Inspect,
    EventKind::CompleteSuccess,
    EventKind::CompleteDomainError,
    EventKind::Deliver,
    EventKind::Cancel,
    EventKind::Trap,
    EventKind::Deadline,
    EventKind::Budget,
    EventKind::InternalFailure,
    EventKind::NativeStopped,
    EventKind::SettlePins,
    EventKind::Cleanup,
    EventKind::Wake,
    EventKind::TakeWake,
    EventKind::Finish,
];

impl super::Event {
    pub const fn kind(self) -> EventKind {
        match self {
            Self::Inspect => EventKind::Inspect,
            Self::CompleteSuccess {
                native: _,
                completion: _,
            } => EventKind::CompleteSuccess,
            Self::CompleteDomainError {
                native: _,
                completion: _,
            } => EventKind::CompleteDomainError,
            Self::Deliver => EventKind::Deliver,
            Self::Cancel => EventKind::Cancel,
            Self::Trap => EventKind::Trap,
            Self::Deadline => EventKind::Deadline,
            Self::Budget => EventKind::Budget,
            Self::InternalFailure => EventKind::InternalFailure,
            Self::NativeStopped { native: _ } => EventKind::NativeStopped,
            Self::SettlePins { native: _, pins: _ } => EventKind::SettlePins,
            Self::Cleanup(_) => EventKind::Cleanup,
            Self::Wake { native: _ } => EventKind::Wake,
            Self::TakeWake => EventKind::TakeWake,
            Self::Finish => EventKind::Finish,
        }
    }
}

/// Classifier output is descriptive, never an independently applicable permit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Inspect,
    Complete {
        outcome: super::Outcome,
        completion: super::Completion,
    },
    Deliver,
    Retire(super::Failure),
    ObserveStop,
    SettlePins(u64),
    Cleanup(super::Obligations),
    Wake,
    TakeWake,
    Finish,
    Duplicate,
}

/// Full 5 x 15 production matrix. Data checks follow classification, not guards
/// standing in for uncovered constructor pairs. Invalid pairs retain all owners.
// r[impl RA-ASYNC-03]
// r[impl DX-PROTOCOL-03]
pub const fn classify(state: super::State, event: super::Event) -> Result<Rule, super::Error> {
    match (state, event) {
        (_, super::Event::Inspect) => Ok(Rule::Inspect),
        (
            _,
            super::Event::CompleteSuccess {
                native: _,
                completion,
            },
        ) => Ok(completion_rule(state, super::Outcome::Success, completion)),
        (
            _,
            super::Event::CompleteDomainError {
                native: _,
                completion,
            },
        ) => Ok(completion_rule(
            state,
            super::Outcome::DomainError,
            completion,
        )),
        (_, super::Event::Deliver) => delivery_rule(state),
        (_, super::Event::Cancel) => Ok(retirement_rule(state, super::Failure::Cancelled)),
        (_, super::Event::Trap) => Ok(retirement_rule(state, super::Failure::Trap)),
        (_, super::Event::Deadline) => Ok(retirement_rule(state, super::Failure::Deadline)),
        (_, super::Event::Budget) => Ok(retirement_rule(state, super::Failure::Budget)),
        (_, super::Event::InternalFailure) => Ok(retirement_rule(state, super::Failure::Internal)),
        (
            super::State::Pending
            | super::State::Ready
            | super::State::Delivered
            | super::State::Retiring,
            super::Event::NativeStopped { native: _ },
        ) => Ok(Rule::ObserveStop),
        (super::State::Retired, super::Event::NativeStopped { native: _ }) => Ok(Rule::Duplicate),
        (
            super::State::Pending
            | super::State::Ready
            | super::State::Delivered
            | super::State::Retiring,
            super::Event::SettlePins { native: _, pins },
        ) => Ok(Rule::SettlePins(pins)),
        (super::State::Retired, super::Event::SettlePins { native: _, pins: _ }) => {
            Err(super::Error::Retired)
        }
        (super::State::Pending, super::Event::Cleanup(_)) => Err(super::Error::Pending),
        (
            super::State::Ready | super::State::Delivered | super::State::Retiring,
            super::Event::Cleanup(obligations),
        ) => Ok(Rule::Cleanup(obligations)),
        (super::State::Retired, super::Event::Cleanup(_)) => Err(super::Error::Retired),
        (super::State::Pending | super::State::Ready, super::Event::Wake { native: _ }) => {
            Ok(Rule::Wake)
        }
        (super::State::Delivered, super::Event::Wake { native: _ }) => Err(super::Error::Delivered),
        (super::State::Retiring, super::Event::Wake { native: _ }) => Err(super::Error::Retiring),
        (super::State::Retired, super::Event::Wake { native: _ }) => Err(super::Error::Retired),
        (super::State::Pending | super::State::Ready, super::Event::TakeWake) => Ok(Rule::TakeWake),
        (super::State::Delivered, super::Event::TakeWake) => Err(super::Error::Delivered),
        (super::State::Retiring, super::Event::TakeWake) => Err(super::Error::Retiring),
        (super::State::Retired, super::Event::TakeWake) => Err(super::Error::Retired),
        (super::State::Pending, super::Event::Finish) => Err(super::Error::Pending),
        (super::State::Ready, super::Event::Finish) => Err(super::Error::Ready),
        (super::State::Delivered | super::State::Retiring, super::Event::Finish) => {
            Ok(Rule::Finish)
        }
        (super::State::Retired, super::Event::Finish) => Ok(Rule::Duplicate),
    }
}

const fn delivery_rule(state: super::State) -> Result<Rule, super::Error> {
    match state {
        super::State::Ready => Ok(Rule::Deliver),
        super::State::Pending => Err(super::Error::Pending),
        super::State::Delivered => Err(super::Error::Delivered),
        super::State::Retiring => Err(super::Error::Retiring),
        super::State::Retired => Err(super::Error::Retired),
    }
}

const fn completion_rule(
    state: super::State,
    outcome: super::Outcome,
    completion: super::Completion,
) -> Rule {
    match state {
        super::State::Pending | super::State::Retiring => Rule::Complete {
            outcome,
            completion,
        },
        super::State::Ready | super::State::Delivered | super::State::Retired => Rule::Duplicate,
    }
}

const fn retirement_rule(state: super::State, reason: super::Failure) -> Rule {
    match state {
        super::State::Pending | super::State::Ready => Rule::Retire(reason),
        super::State::Delivered | super::State::Retiring | super::State::Retired => Rule::Duplicate,
    }
}
