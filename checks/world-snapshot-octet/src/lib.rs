#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

#[allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the focused scaffold mirrors Molten error names needed by the production adapter without importing the full product shell"
)]
pub mod error {
    use core::fmt;

    #[derive(Debug)]
    pub struct Failure(String);

    impl Failure {
            pub fn invalid_harness(message: impl Into<String>) -> Self {
                Self(message.into())
            }
        }

    impl fmt::Display for Failure {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

    impl std::error::Error for Failure {}

    pub type Result<T> = core::result::Result<T, Failure>;
}

pub mod preserves_rail {
    pub fn validate_content_ref(value: &str) -> core::result::Result<(), ()> {
        const PREFIX: &str = "blake3:";
        const HEX_BYTES: usize = 64;
        let Some(hex) = value.strip_prefix(PREFIX) else {
            return Err(());
        };
        if hex.len() == HEX_BYTES
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            Ok(())
        } else {
            Err(())
        }
    }

    #[must_use]
    pub fn string(value: impl AsRef<str>) -> preserves::IOValue {
        preserves::IOValue::new(value.as_ref().to_owned())
    }

    #[must_use]
    pub fn u64_value(value: u64) -> preserves::IOValue {
        preserves::IOValue::new(value)
    }

    #[must_use]
    pub fn sequence(values: Vec<preserves::IOValue>) -> preserves::IOValue {
        preserves::IOValue::new(values)
    }

    #[must_use]
    pub fn record(label: &'static str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
        preserves::IOValue::record(preserves::IOValue::symbol(label), fields)
    }

    /// Canonical packed Preserves bytes of `value`.
    ///
    /// # Errors
    ///
    /// Returns a failure when the value cannot be written as packed Preserves.
    pub fn canonical_bytes(value: &preserves::IOValue) -> crate::error::Result<Vec<u8>> {
        preserves::write_iovalue_packed(value, false)
            .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))
    }
}

pub mod chaoscontrol;
#[path = "vm/mod.rs"]
pub mod vm;
