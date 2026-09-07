//! Reduced compiler-integration fixture, not a production domain or runtime proof.
#![cfg_attr(dylint_lib = "octet", feature(register_tool))]
#![cfg_attr(dylint_lib = "octet", register_tool(octet))]

#[cfg_attr(all(dylint_lib = "octet", not(disable_marker)), octet::sealed_enum)]
pub enum Classification {
    First,
    Second,
    #[cfg(future)]
    Future,
}

pub fn classify(value: Classification) -> u8 {
    match value {
        Classification::First => 1,
        Classification::Second => 2,
    }
}
