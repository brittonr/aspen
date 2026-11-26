//! Bounded serde vocabulary shared by the experimental mechanism profile DTOs.
//!
//! The DTOs themselves stay in the `molten` shell (`src/reliability/service/profiles/`): they
//! convert into `molten-core` fabric transport, time, and durability types and hash definitions
//! over canonical Preserves. This module owns the pure, dependency-free part: text and collection
//! bounds enforced during deserialization and closed-vocabulary selection.

pub mod bounded;

/// A profile string that names no member of the closed vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyIssue {
    pub value: alloc::string::String,
}

impl core::fmt::Display for VocabularyIssue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "unsupported profile value: {}", self.value)
    }
}

impl core::error::Error for VocabularyIssue {}

/// Selects the single vocabulary member whose canonical name equals `text`; no default applies.
pub fn choice<T: Copy>(text: &str, choices: &[T], name: impl Fn(T) -> &'static str) -> Result<T, VocabularyIssue> {
    choices
        .iter()
        .copied()
        .find(|value| name(*value) == text)
        .ok_or_else(|| VocabularyIssue { value: text.into() })
}

/// Selects every member named by `texts`, in order, failing on the first unknown name.
pub fn choices<T: Copy>(
    texts: alloc::vec::Vec<alloc::string::String>,
    values: &[T],
    name: impl Fn(T) -> &'static str,
) -> Result<alloc::vec::Vec<T>, VocabularyIssue> {
    texts.into_iter().map(|text| choice(&text, values, &name)).collect()
}
