//! Bounded mechanism DTOs. Decoding and profile admission never authorize effects.
//!
//! Text/collection bounds and closed-vocabulary selection are the pure
//! `molten_reliability_model_core::mechanism` rules; the DTOs stay here because they convert
//! into `molten-core` fabric types and hash definitions over canonical Preserves.

mod addressing;
mod clock;
mod durability;
mod mechanism;
mod wire;

pub use addressing::*;
pub use clock::*;
pub use durability::*;
pub use mechanism::*;
pub use wire::*;

/// `deserialize_with` targets for the DTO fields; the bounds are the pure
/// `molten_reliability_model_core::mechanism::bounded` rules.
mod bounded {
    pub(super) fn text<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
        molten_reliability_model_core::mechanism::bounded::text(deserializer)
    }

    pub(super) fn texts<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
        molten_reliability_model_core::mechanism::bounded::texts(deserializer)
    }

    pub(super) fn optional_text<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
        molten_reliability_model_core::mechanism::bounded::optional_text(deserializer)
    }
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

fn choice<T: Copy>(text: &str, choices: &[T], name: impl Fn(T) -> &'static str) -> crate::error::Result<T> {
    molten_reliability_model_core::mechanism::choice(text, choices, name).map_err(|issue| invalid(issue.to_string()))
}

fn choices<T: Copy>(
    texts: Vec<String>,
    values: &[T],
    name: impl Fn(T) -> &'static str,
) -> crate::error::Result<Vec<T>> {
    molten_reliability_model_core::mechanism::choices(texts, values, name).map_err(|issue| invalid(issue.to_string()))
}

#[cfg(test)]
mod tests;
