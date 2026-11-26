type IoValue = preserves::IOValue;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::Failure;
type Result<T> = crate::error::Result<T>;

/// The schema-id constants live in the `no_std` kernel; every `preserves_rail::*_SCHEMA` path
/// resolves through this re-export.
pub use molten_preserves_core::schema::*;
