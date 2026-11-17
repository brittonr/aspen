//! Bounded synchronous resource ownership, independent of native storage.
//!
//! The shell owns native objects, serializes access to a `Table`, and executes
//! each returned accounting decision once. A local release decision is not a
//! claim that a native destructor ran or that a remote object was released.
//! Dropping an `Owner` or `Borrow` never discharges the table's obligation:
//! abnormal invocation cleanup must call `retire_context`, then retain the
//! table and native pins until matching completions arrive.
//!
//! `Handle`, `Scope`, `Snapshot`, and `Decision` are descriptive, untrusted
//! data. Only the table's private retained state can mint an opaque owner.
//! The public single-record `transition` is the same algebra used by the
//! table, not a separate proof model. Its decisions alone grant no authority.

mod table;
mod transition;

pub use table::Table;
pub use transition::transition;

/// Hard allocation and iteration ceiling, including transfer preflight.
pub const MAX_SLOTS: usize = 256;

/// Host-assigned table namespace. Never reuse it while old handles can exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableId(pub u64);

/// Exact component/invocation owner context, assigned by the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context(pub u64);

/// Granted rights; required rights must be a subset of these bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rights(pub u64);

/// An untrusted boundary handle, not an owning capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    pub table: TableId,
    pub slot: usize,
    pub generation: u64,
    pub context: Context,
    pub kind: crate::types::ResourceKind,
    pub rights: Rights,
}

/// Independent operation requirements, selected by the approved adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirement {
    pub context: Context,
    pub kind: crate::types::ResourceKind,
    pub rights: Rights,
}

/// Full callback binding. A copied claim cannot complete a scope twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope {
    pub handle: Handle,
    pub serial: u64,
}

/// One record's complete guest-availability and native-pin state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum State {
    Live,
    Busy(u64),
    Retiring(u64),
    Retired,
}

/// First abnormal outcome; subsequent cleanup does not replace it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retirement {
    Cancelled,
    Trap,
    UnexpectedSuspension,
    HostCleanup,
}

/// Both normal result alternatives obey the same owner-return contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
    Success,
    DomainError,
}

/// Public semantic record, never accepted as a replacement for retained state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub handle: Handle,
    pub state: State,
    pub last_scope: Option<u64>,
    pub retirement: Option<Retirement>,
}

/// Exhaustive single-record input vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum Event {
    Inspect(Requirement),
    Begin {
        required: Requirement,
        serial: u64,
    },
    Access {
        serial: u64,
    },
    Complete {
        serial: u64,
        result: Completion,
    },
    Revoke {
        serial: u64,
        reason: Retirement,
    },
    Retire(Retirement),
    Release(Requirement),
    Transfer {
        required: Requirement,
        receiver: Context,
        generation: u64,
    },
}

/// Local accounting action, not a physical/native outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tigerstyle::state_machine_enum]
pub enum Action {
    Unchanged,
    BorrowAdmitted,
    OwnerReturned(Completion),
    AccessRevoked,
    LocalRelease,
    RetirementCompleted,
    OwnerTransferred,
}

/// Actual single-record decision used by the production table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub record: Snapshot,
    pub action: Action,
}

/// Delta to apply exactly once at the shell boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accounting {
    pub pins_acquired: usize,
    pub pins_released: usize,
    pub owners_returned: usize,
    pub owners_transferred: usize,
    pub local_releases: usize,
}

impl Decision {
    /// Derive accounting rather than storing redundant mutable counters.
    pub fn accounting(&self) -> Accounting {
        let mut delta = Accounting {
            pins_acquired: 0,
            pins_released: 0,
            owners_returned: 0,
            owners_transferred: 0,
            local_releases: 0,
        };
        match self.action {
            Action::Unchanged | Action::AccessRevoked => {}
            Action::BorrowAdmitted => delta.pins_acquired = 1,
            Action::OwnerReturned(_) => {
                delta.pins_released = 1;
                delta.owners_returned = 1;
            }
            Action::LocalRelease => delta.local_releases = 1,
            Action::RetirementCompleted => {
                delta.pins_released = 1;
                delta.local_releases = 1;
            }
            Action::OwnerTransferred => delta.owners_transferred = 1,
        }
        delta
    }
}

/// Explicit bounded failure. Rejection never starts protected work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Capacity,
    PinCapacity,
    ContextCapacity,
    InvalidHandle,
    WrongContext,
    WrongKind,
    WrongGeneration,
    WrongRights,
    Busy,
    Retiring,
    Retired,
    WrongScope,
    GenerationExhausted,
    ScopeExhausted,
    ArgumentCount,
    DuplicateOwner,
    InvalidRecord,
}

/// An opaque move-only guest owner. No public raw-handle constructor exists.
#[derive(Debug)]
pub struct Owner {
    handle: Handle,
}

impl Owner {
    /// Inspect boundary identity without duplicating an owner.
    pub fn handle(&self) -> Handle {
        self.handle
    }
}

/// Adapter-local scope token. It carries no native pointer or guest value.
#[derive(Debug)]
pub struct Borrow {
    scope: Scope,
}

/// Committed admission and its one native pin, before the shell starts work.
#[derive(Debug)]
pub struct Admitted {
    pub borrow: Borrow,
    pub decision: Decision,
}

impl Borrow {
    /// Callback data remains untrusted and is checked against retained state.
    pub fn scope(&self) -> Scope {
        self.scope
    }
}

/// Failed preflight returns every input obligation unchanged.
#[derive(Debug)]
pub struct Rejected<T> {
    pub error: Error,
    pub input: T,
}

/// Completion can publish an owner only through a matching retained scope.
#[derive(Debug)]
pub struct Completed {
    pub owner: Option<Owner>,
    pub decision: Decision,
}

/// Committed receiver obligations, in original argument order.
#[derive(Debug)]
pub struct Transferred {
    pub owners: alloc::vec::Vec<Owner>,
    pub decisions: alloc::vec::Vec<Decision>,
}

/// Retained table bounds. Counter limits may be lowered to bound lifetimes;
/// reaching either ceiling fails closed rather than recycling an identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub slots: usize,
    pub pins: usize,
    pub owners_per_context: usize,
    pub generations: u64,
    pub scopes: u64,
}

/// Counts of retained obligations, including invisible retiring resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {
    pub live: usize,
    pub busy: usize,
    pub retiring: usize,
    pub retired: usize,
    pub native_pins: usize,
}
