//! The schema ids carried in Aspen's canonical records, as `&'static str` constants.
//!
//! Each id has the form `molten.<domain>.<name>.v<N>` and is unique across the table. The domain
//! submodules are private, so every constant is reachable as `schema::<NAME>`; the `molten` rail
//! re-exports them as `preserves_rail::<NAME>`. A constant that belongs to one domain module of the
//! `molten` crate rather than to the shared rail stays with that module.

/// Declares the schema-id constants of one domain and, under test, the table of their names and
/// ids that the uniqueness and well-formedness tests read.
macro_rules! schema_ids {
    (table $table:ident; $($name:ident = $id:literal;)+) => {
        $(pub const $name: &str = $id;)+

        #[cfg(test)]
        pub(crate) const $table: &[(&str, &str)] = &[$((stringify!($name), $name)),+];
    };
}

mod evidence;
mod node;
mod runtime;
mod services;
mod storage;

pub use evidence::*;
pub use node::*;
pub use runtime::*;
pub use services::*;
pub use storage::*;

/// Every domain table, for the tests.
#[cfg(test)]
const DOMAINS: &[&[(&str, &str)]] = &[
    EVIDENCE_ENTRIES,
    NODE_ENTRIES,
    RUNTIME_ENTRIES,
    SERVICES_ENTRIES,
    STORAGE_ENTRIES,
];

#[cfg(test)]
mod tests;
