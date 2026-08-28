//! Focused strict-Octet compilation surface for the pure world operator core.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

macro_rules! reference_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: String) -> Result<Self, &'static str> {
                if value.is_empty() {
                    Err("reference must not be empty")
                } else {
                    Ok(Self(value))
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

pub mod world_commit {
    reference_type!(WorldCommitRef);
}

pub mod world_head {
    reference_type!(WorldBranchId);
    reference_type!(WorldHeadPolicyRef);
}

pub mod world_operator;
