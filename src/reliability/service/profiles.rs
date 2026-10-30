//! Bounded mechanism DTOs. Decoding and profile admission never authorize effects.

mod addressing;
mod bounded;
mod clock;
mod durability;
mod mechanism;
mod wire;

pub use addressing::*;
pub use clock::*;
pub use durability::*;
pub use mechanism::*;
pub use wire::*;

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

fn choice<T: Copy>(text: &str, choices: &[T], name: impl Fn(T) -> &'static str) -> crate::error::Result<T> {
    choices
        .iter()
        .copied()
        .find(|value| name(*value) == text)
        .ok_or_else(|| invalid(format!("unsupported profile value: {text}")))
}

fn choices<T: Copy>(
    texts: Vec<String>,
    values: &[T],
    name: impl Fn(T) -> &'static str,
) -> crate::error::Result<Vec<T>> {
    texts.into_iter().map(|text| choice(&text, values, &name)).collect()
}

#[cfg(test)]
mod tests;
