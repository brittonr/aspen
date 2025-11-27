//! Generic configuration loader utilities
//!
//! Provides reusable helpers to eliminate boilerplate in configuration structs.

/// Helper trait for parsing environment variables with defaults
pub trait EnvLoader: Sized + Default {
    /// Parse a value from environment variable with fallback
    fn from_env_or_default(key: &str, default: Self) -> Self {
        std::env::var(key)
            .ok()
            .and_then(|v| Self::parse_str(&v))
            .unwrap_or(default)
    }

    /// Parse a value from environment variable (optional)
    fn from_env(key: &str) -> Option<Self> {
        std::env::var(key).ok().and_then(|v| Self::parse_str(&v))
    }

    /// Parse from string
    fn parse_str(s: &str) -> Option<Self>;
}

impl EnvLoader for u64 {
    fn parse_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl EnvLoader for u32 {
    fn parse_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl EnvLoader for bool {
    fn parse_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Macro to reduce boilerplate in config loading
///
/// Generates `load()` and `apply_env_overrides()` methods.
#[macro_export]
macro_rules! impl_config_loader {
    (
        $struct_name:ident {
            $($field:ident: $env_var:literal),* $(,)?
        }
    ) => {
        impl $struct_name {
            /// Load configuration from environment variables
            pub fn load() -> Result<Self, $crate::config::error::ConfigError> {
                let defaults = Self::default();
                Ok(Self {
                    $(
                        $field: $crate::config::loader::EnvLoader::from_env_or_default(
                            $env_var,
                            defaults.$field
                        ),
                    )*
                })
            }

            /// Apply environment variable overrides to existing configuration
            pub fn apply_env_overrides(&mut self) -> Result<(), $crate::config::error::ConfigError> {
                $(
                    if let Some(val) = $crate::config::loader::EnvLoader::from_env($env_var) {
                        self.$field = val;
                    }
                )*
                Ok(())
            }
        }
    };
}
