//! Bounded, serialized facet-owned service dataspace for the M7 synchronous profile.
//!
//! The trusted host, never wire data or a guest, supplies facet identities and
//! grants. This table owns only immutable typed assertions and interests; it does
//! not share guest memory or grant a native resource. Calls are serialized by an
//! exclusive borrow. The host must retire the facet after a component trap or
//! normal termination, before admitting another call from that instance.

pub const MAX_WIRE_BYTES: usize = 64;
pub const MAX_WIRE_DEPTH: u32 = 4;
pub const MAX_NAME_BYTES: usize = 32;
pub const MAX_FACETS: usize = 8;
pub const MAX_ASSERTIONS: usize = 8;
pub const MAX_INTERESTS: usize = 8;
pub const MAX_EVENTS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    ByteLimit,
    DepthLimit,
    Schema,
    InvalidName,
    InvalidFacet,
    Denied,
    Capacity,
}

mod table;
mod wire;
pub use wire::{decode_service, encode_service, valid_name, Packet, ServiceRef};

/// The selected component profile never installs a common mutable guest
/// memory into two stores. Refuse such a request before opening a facet.
pub const fn admit_shared_memory(requested: bool) -> Result<(), Error> {
    if requested {
        Err(Error::Denied)
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rights {
    pub publish: bool,
    pub observe: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publication {
    Unchanged,
    Added,
    Replaced,
}

/// Pure, allocation-free decision root; the shell separately checks queue
/// reservation and commits the corresponding table mutation.
pub const fn decide_publication(current: Option<bool>, requested: bool) -> Publication {
    match current {
        Some(value) if value == requested => Publication::Unchanged,
        Some(_) => Publication::Replaced,
        None => Publication::Added,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum Operation {
    Publish,
    Observe,
}

/// An effect is statically selected by WIT; only this independent runtime
/// right, supplied by the host, can authorize its execution.
pub const fn permits(rights: Rights, operation: Operation) -> bool {
    match operation {
        Operation::Publish => rights.publish,
        Operation::Observe => rights.observe,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facet(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub facets: usize,
    pub assertions: usize,
    pub interests: usize,
    pub events: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Service {
    pub name: alloc::string::String,
    pub ready: bool,
}

impl Service {
    fn from_ref(value: ServiceRef<'_>) -> Self {
        Self {
            name: alloc::string::String::from(value.name),
            ready: value.ready,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Added,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub observer: Facet,
    pub service: Service,
    pub change: Change,
}

struct Scope {
    id: Facet,
    parent: Option<Facet>,
    rights: Rights,
    live: bool,
}

struct Assertion {
    owner: Facet,
    service: Service,
}

struct Interest {
    owner: Facet,
    service: Service,
}

pub struct Table {
    limits: Limits,
    next: u64,
    scopes: alloc::vec::Vec<Scope>,
    assertions: alloc::vec::Vec<Assertion>,
    interests: alloc::vec::Vec<Interest>,
    events: alloc::vec::Vec<Event>,
}
