//! vCard 4.0 parser and serializer (RFC 6350).

pub mod parse;
pub mod serialize;

pub use parse::parse_vcard;
pub use parse::parse_vcards;
pub use serialize::serialize_vcard;
pub use serialize::serialize_vcards;
