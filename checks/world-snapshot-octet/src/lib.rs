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
    pub struct MoltenError(String);

    impl MoltenError {
        pub fn invalid_harness(message: impl Into<String>) -> Self {
            Self(message.into())
        }
    }

    impl fmt::Display for MoltenError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(&self.0)
        }
    }

    impl std::error::Error for MoltenError {}

    pub type Result<T> = core::result::Result<T, MoltenError>;
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
}

#[path = "chaoscontrol.rs"]
pub mod chaoscontrol;
#[path = "vm/mod.rs"]
pub mod vm;
