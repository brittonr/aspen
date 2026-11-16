use alloc::string::String;
use core::fmt;

use serde::{Deserialize, Serialize};

/// Current checkpoint and cohort protocol profile.
pub const VM_COHORT_PROFILE: &str = "vm-cohort-v1";
/// Maximum text bytes in one core identifier.
pub const MAX_TEXT_BYTES: usize = 256;
/// Maximum workers admitted by the public core.
pub const MAX_WORKERS: u32 = 256;
/// Maximum planned effects retained in one cohort.
pub const MAX_EFFECTS: usize = 4_096;
/// Maximum lifecycle observations retained in one reduction.
pub const MAX_OBSERVATIONS: usize = 8_192;
/// Maximum cleanup obligations retained in one cohort.
pub const MAX_CLEANUP_OBLIGATIONS: usize = 4_096;
/// Required receipt claim boundaries.
pub const RECEIPT_NON_CLAIMS: &[&str] = &[
    "checkpoint admission does not prove guest image trust",
    "cohort planning does not grant workload or execution authority",
    "KVM observations do not prove sandbox correctness or determinism",
    "cleanup observations do not prove physical resource erasure",
    "receipts do not grant retention, dependency, or release authority",
];

/// Error returned by a typed BLAKE3 reference constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReferenceError;

impl fmt::Display for ReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid lowercase BLAKE3 reference")
    }
}

macro_rules! digest_reference {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// Constructs a validated lowercase BLAKE3 reference.
            ///
            /// # Errors
            ///
            /// Returns [`ReferenceError`] when the value is not one canonical
            /// lowercase `blake3:` reference.
            pub fn new(value: impl Into<String>) -> Result<Self, ReferenceError> {
                let value = value.into();
                if valid_reference(&value) {
                    Ok(Self(value))
                } else {
                    Err(ReferenceError)
                }
            }

            /// Returns the canonical textual reference.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = ReferenceError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

digest_reference!(
    CheckpointRef,
    "Identity of an admitted immutable checkpoint."
);
digest_reference!(CohortRef, "Identity of one complete cohort plan.");
digest_reference!(CloneRef, "Identity of one clone in a cohort.");
digest_reference!(
    OperationRef,
    "Identity of one effect or transition operation."
);
digest_reference!(
    ProfileRef,
    "Identity of one compatibility or resource profile."
);
digest_reference!(
    ResourceRef,
    "Identity of one immutable or mutable resource."
);
digest_reference!(ReceiptRef, "Identity of one bounded receipt.");

fn valid_reference(value: &str) -> bool {
    const PREFIX: &str = "blake3:";
    const HEX_BYTES: usize = 64;
    value.strip_prefix(PREFIX).is_some_and(|hex| {
        hex.len() == HEX_BYTES
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}
