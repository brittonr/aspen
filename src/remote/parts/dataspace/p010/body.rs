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
            .live_assertions()
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
    // Invalid typed carriers cannot establish introductions. Delivery parsing
    // reports their schema error; the pure derivation remains fail-closed.
    semantic_payload_references(value.as_iovalue()).unwrap_or_default()
}

/// The canonical content references named in canonical text.
pub fn references_in_text(text: &str) -> Vec<String> {
    const PREFIX: &str = "blake3:";
    const HEX_LENGTH: usize = 64;
    let bytes = text.as_bytes();
    let mut references: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut index = 0;
    while index + PREFIX.len() + HEX_LENGTH <= bytes.len() {
        if bytes[index..].starts_with(PREFIX.as_bytes()) {
            let start = index + PREFIX.len();
            let candidate = &bytes[start..start + HEX_LENGTH];
            if candidate.iter().all(u8::is_ascii_hexdigit) {
                // The matched prefix and hexadecimal suffix are ASCII, so both
                // offsets are UTF-8 boundaries even after non-ASCII text.
                references.insert(text[index..start + HEX_LENGTH].to_string());
                index = start + HEX_LENGTH;
                continue;
            }
        }
        index += 1;
    }
    references.into_iter().collect()
}


/// Inventory content-bearing fields, not the envelope's schema/identity hashes.
/// Known carriers are decoded by their owning parser before extracting refs.
pub fn semantic_payload_references(payload: &IoValue) -> Result<Vec<String>> {
    let mut refs = std::collections::BTreeSet::new();
    // A protocol message is unwrapped to its body before inventory; nesting
    // depth is bounded by the already-parsed finite payload value.
    let mut nested_body: Option<IoValue> = None;
    loop {
        let current = nested_body.as_ref().unwrap_or(payload);
        if current.collect_simple_record("protocol-message-v1", None).is_some() {
            let message = crate::protocol_session::parse_protocol_message(current)?;
            // The installed protocol/schema and evidence identify interpretation,
            // not transferred object references. Only body-or-ref is application data.
            nested_body = Some(message.body_or_ref);
            continue;
        }
        if current.collect_simple_record("job-worker-request-v1", None).is_some() {
            let request = crate::job_dag::parse_job_worker_request_value(current)?;
            // Workload operands are content references; admission/sync receipts and
            // authority/resource/bootstrap/evidence lists remain independent gates.
            refs.extend([request.job_ref, request.execution_request_ref]);
        } else {
            refs.extend(references_in_text(&crate::preserves_rail::to_text(current)?));
        }
        return Ok(refs.into_iter().collect());
    }
}

fn message_references(payload: &IoValue, content_refs: &[String]) -> Result<Vec<String>> {
    let mut refs: std::collections::BTreeSet<String> = semantic_payload_references(payload)?.into_iter().collect();
    refs.extend(content_refs.iter().cloned());
    Ok(refs.into_iter().collect())
}

include!("recorded.rs");
include!("recordedstore.rs");

#[cfg(test)]
mod recorded_session_tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/remote/parts/dataspace/tests/m001/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/remote/parts/dataspace/tests/m001/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/remote/parts/dataspace/tests/m001/p002/body.rs"));
}
