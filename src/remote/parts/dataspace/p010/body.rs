// The reference introduction rule.
//
// A message must not embed a reference the peer has not been introduced to.
// The introduced set for a session is the session's bootstrap references plus
// the references its live assertions name, so withdrawing the last assertion
// that introduced a reference withdraws the introduction.

/// The references a session has been introduced to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntroducedReferences {
    references: std::collections::BTreeSet<String>,
}

impl IntroducedReferences {
    /// Derive the introduced set from the session bootstrap references and the
    /// session's live assertions.
    pub fn derive(
        state: &crate::runtime::RuntimeState,
        session_owner: &str,
        bootstrap_references: &[String],
    ) -> Self {
        let mut references: std::collections::BTreeSet<String> = bootstrap_references.iter().cloned().collect();
        for assertion in state
            .snapshot()
            .assertions
            .iter()
            .filter(|assertion| assertion.actor == session_owner)
        {
            for reference in references_in_value(&assertion.value) {
                references.insert(reference);
            }
        }
        Self { references }
    }

    /// Whether the peer knows the reference.
    pub fn contains(&self, reference: &str) -> bool {
        self.references.contains(reference)
    }

    /// The carried references the peer does not know yet.
    pub fn unknown(&self, references: &[String]) -> Vec<String> {
        references
            .iter()
            .filter(|reference| !self.references.contains(*reference))
            .cloned()
            .collect()
    }

    /// The known references, in a stable order.
    pub fn references(&self) -> Vec<String> {
        self.references.iter().cloned().collect()
    }
}

/// The canonical content references named by a value.
pub fn references_in_value(value: &crate::runtime::RuntimeValue) -> Vec<String> {
    let text = crate::preserves_rail::to_text(value.as_iovalue()).unwrap_or_default();
    references_in_text(&text)
}

/// The canonical content references named in canonical text.
pub fn references_in_text(text: &str) -> Vec<String> {
    const PREFIX: &str = "blake3:";
    const HEX_LENGTH: usize = 64;
    let bytes = text.as_bytes();
    let mut references: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut index = 0;
    while index + PREFIX.len() + HEX_LENGTH <= bytes.len() {
        if text[index..].starts_with(PREFIX) {
            let start = index + PREFIX.len();
            let candidate = &text[start..start + HEX_LENGTH];
            if candidate.chars().all(|character| character.is_ascii_hexdigit()) {
                references.insert(text[index..start + HEX_LENGTH].to_string());
                index = start + HEX_LENGTH;
                continue;
            }
        }
        index += 1;
    }
    references.into_iter().collect()
}

/// Refuse to build a message that carries an unintroduced reference.
pub fn refuse_unintroduced_references(
    carried: &[String],
    introduced: &IntroducedReferences,
) -> std::result::Result<(), Vec<String>> {
    let unknown = introduced.unknown(carried);
    if unknown.is_empty() {
        return Ok(());
    }
    Err(unknown)
}
