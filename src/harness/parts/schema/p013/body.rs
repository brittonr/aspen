
struct RedactionOutputStack {
    values: Vec<IoValue>,
}

impl RedactionOutputStack {
    fn new() -> Self {
        Self {
            values: Vec::with_capacity(1),
        }
    }

    fn push(&mut self, value: IoValue) -> Result<()> {
        ensure_redaction_bound(self.values.len() + 1, MAX_REDACTION_TRANSFORM_NODES, "redaction traversal outputs")?;
        self.values.push(value);
        Ok(())
    }

    fn take(&mut self, count: usize) -> Result<Vec<IoValue>> {
        if self.values.len() < count {
            return Err(MoltenError::invalid_harness("redaction traversal stack underflow"));
        }
        Ok(self.values.split_off(self.values.len() - count))
    }

    fn finish(mut self) -> Result<IoValue> {
        if self.values.len() != 1 {
            return Err(MoltenError::invalid_harness("redaction traversal produced invalid output"));
        }
        self.values
            .pop()
            .ok_or_else(|| MoltenError::invalid_harness("redaction traversal produced no output"))
    }
}

fn bounded_redaction_child_count(value: &IoValue, context: &str) -> Result<usize> {
    let count = value.iter().count();
    ensure_redaction_bound(count, MAX_REDACTION_CONTAINER_ITEMS, context)?;
    Ok(count)
}

fn redaction_child_entries(value: &IoValue, path: &str, context: &str) -> Result<Vec<(IoValue, String)>> {
    let child_count = bounded_redaction_child_count(value, context)?;
    let mut entries = Vec::with_capacity(child_count);
    for (index, child) in value.iter().enumerate() {
        entries.push((value_to_iovalue(&child), format!("{path}/{index}")));
    }
    Ok(entries)
}

struct RedactionTraversal<'a> {
    stack: RedactionFrameStack,
    outputs: RedactionOutputStack,
    state: &'a mut RedactionTransformState,
}

impl<'a> RedactionTraversal<'a> {
    fn new(state: &'a mut RedactionTransformState) -> Self {
        Self {
            stack: RedactionFrameStack::new(),
            outputs: RedactionOutputStack::new(),
            state,
        }
    }

    fn run(mut self, value: &IoValue, path: &str) -> Result<IoValue> {
        self.stack.push(RedactionTraversalFrame::Enter {
            value: value.clone(),
            path: path.to_string(),
        })?;
        let mut visited_nodes = 0usize;
        while let Some(frame) = self.stack.pop() {
            visited_nodes += 1;
            ensure_redaction_bound(visited_nodes, MAX_REDACTION_TRANSFORM_NODES, "redaction traversal visited nodes")?;
            self.handle(frame)?;
        }
        self.outputs.finish()
    }

    fn handle(&mut self, frame: RedactionTraversalFrame) -> Result<()> {
        match frame {
            RedactionTraversalFrame::Enter { value, path } => self.enter(value, path),
            RedactionTraversalFrame::ExitRecord {
                original,
                label,
                field_count,
            } => self.exit_record(original, label, field_count),
            RedactionTraversalFrame::ExitSequence { original, item_count } => self.exit_sequence(original, item_count),
        }
    }

    fn enter(&mut self, value: IoValue, path: String) -> Result<()> {
        if let Some(label) = record_label_string(&value)
            && is_sensitive_record_label(&label)
        {
            let redacted = transform_sensitive_record(&value, &label, &path, self.state)?;
            return self.outputs.push(redacted);
        }
        match value.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => self.outputs.push(value),
            ValueClass::Compound(CompoundClass::Record) => self.enter_record(value, path),
            ValueClass::Compound(CompoundClass::Sequence) => self.enter_sequence(value, path),
            ValueClass::Compound(CompoundClass::Set) | ValueClass::Compound(CompoundClass::Dictionary) => {
                self.outputs.push(value)
            }
        }
    }

    fn enter_record(&mut self, value: IoValue, path: String) -> Result<()> {
        let label = value_to_iovalue(&value.label());
        let child_entries = redaction_child_entries(&value, &path, "redaction record fields")?;
        self.stack.push(RedactionTraversalFrame::ExitRecord {
            original: value,
            label,
            field_count: child_entries.len(),
        })?;
        self.stack.push_children(child_entries)
    }

    fn enter_sequence(&mut self, value: IoValue, path: String) -> Result<()> {
        let child_entries = redaction_child_entries(&value, &path, "redaction sequence items")?;
        self.stack.push(RedactionTraversalFrame::ExitSequence {
            original: value,
            item_count: child_entries.len(),
        })?;
        self.stack.push_children(child_entries)
    }

    fn exit_record(&mut self, original: IoValue, label: IoValue, field_count: usize) -> Result<()> {
        let fields = self.outputs.take(field_count)?;
        self.push_rebuilt(original, IoValue::record(label, fields))
    }

    fn exit_sequence(&mut self, original: IoValue, item_count: usize) -> Result<()> {
        let values = self.outputs.take(item_count)?;
        self.push_rebuilt(original, sequence(values))
    }

    fn push_rebuilt(&mut self, original: IoValue, rebuilt: IoValue) -> Result<()> {
        if rebuilt == original {
            self.outputs.push(original)
        } else {
            self.outputs.push(rebuilt)
        }
    }
}

fn transform_sensitive_value(value: &IoValue, path: &str, state: &mut RedactionTransformState) -> Result<IoValue> {
    RedactionTraversal::new(state).run(value, path)
}

fn transform_sensitive_record(
    value: &IoValue,
    label: &str,
    path: &str,
    state: &mut RedactionTransformState,
) -> Result<IoValue> {
    if label == "encrypted-ref" {
        return Err(MoltenError::invalid_harness(
            "malformed encrypted-ref marker cannot be accepted into a repro export profile",
        ));
    }
    if label == "encrypted-ref-v1" {
        let encrypted = crate::secrets::parse_encrypted_ref(value)?;
        if state.profile != ReproExportProfile::EncryptedPrivate {
            return redaction_marker_for_value(value, label, path, state);
        }
        state.encrypted_refs.push(encrypted.encrypted_ref);
        return Ok(value.clone());
    }
    match state.profile {
        ReproExportProfile::DenySensitive => Err(MoltenError::invalid_harness(format!(
            "redaction preflight found sensitive marker {label}; sealed pass repro bundles require explicit redaction before export"
        ))),
        ReproExportProfile::RedactedDiagnostic => redaction_marker_for_value(value, label, path, state),
        ReproExportProfile::EncryptedPrivate => encrypted_ref_for_value(value, label, path, state),
    }
}

fn redaction_marker_for_value(
    value: &IoValue,
    label: &str,
    path: &str,
    state: &mut RedactionTransformState,
) -> Result<IoValue> {
    let commitment_ref = canonical_hash(value)?;
    let path_ref = canonical_hash(&string(path))?;
    let receipt_ref = canonical_hash(&record("redaction-marker-seed", vec![
        string(&commitment_ref),
        string(&path_ref),
        string(&state.policy_ref),
    ]))?;
    let marker_value = crate::secrets::redaction_marker_value(&crate::secrets::RedactionMarkerInput {
        reason: label.to_string(),
        commitment_ref: commitment_ref.clone(),
        schema_ref: canonical_hash(&string(label))?,
        path_ref,
        policy_refs: vec![state.policy_ref.clone()],
        receipt_ref,
    })?;
    let marker = crate::secrets::parse_redaction_marker(&marker_value)?;
    state.marker_refs.push(marker.marker_ref.clone());
    state.marker_entries.push(RedactionManifestEntry {
        path: path.to_string(),
        reason: label.to_string(),
        commitment_ref,
        marker_ref: Some(marker.marker_ref),
        encrypted_ref: None,
    });
    Ok(marker_value)
}

fn encrypted_ref_for_value(
    value: &IoValue,
    label: &str,
    path: &str,
    state: &mut RedactionTransformState,
) -> Result<IoValue> {
    let commitment_ref = canonical_hash(value)?;
    let ciphertext_ref =
        canonical_hash(&record("encrypted-redaction-ciphertext", vec![string(&commitment_ref), string(path)]))?;
    let encrypted_value = crate::secrets::encrypted_ref_value(&crate::secrets::EncryptedRefInput {
        ciphertext_ref,
        commitment_ref: commitment_ref.clone(),
        encryption_ref: canonical_hash(&repro_export_profile_value(state.profile))?,
        schema_ref: canonical_hash(&string(label))?,
        policy_refs: vec![state.policy_ref.clone()],
        evidence_refs: vec![canonical_hash(&string(path))?],
    })?;
    let encrypted = crate::secrets::parse_encrypted_ref(&encrypted_value)?;
    state.encrypted_refs.push(encrypted.encrypted_ref.clone());
    state.marker_entries.push(RedactionManifestEntry {
        path: path.to_string(),
        reason: label.to_string(),
        commitment_ref,
        marker_ref: None,
        encrypted_ref: Some(encrypted.encrypted_ref),
    });
    Ok(encrypted_value)
}

fn record_label_string(value: &IoValue) -> Option<String> {
    if !value.is_record() {
        return None;
    }
    value.label().as_symbol().map(std::borrow::Cow::into_owned)
}

pub fn failure_repro_bundle_value(failure_value: &IoValue) -> Result<IoValue> {
    failure_repro_bundle_value_with_command(failure_value, &default_failure_bundle_command())
}

pub fn failure_repro_bundle_value_with_command(failure_value: &IoValue, command: &[String]) -> Result<IoValue> {
    let failure = parse_failure(failure_value)?;
    Ok(record("harness-repro-bundle-v1", vec![
        string(crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA),
        record("bundle-kind", vec![string("failure")]),
        tool_value(),
        command_value(command),
        replay_instructions_value(&[
            &["molten", "test", "report", "show", "failure.preserves"][..],
            &["molten", "test", "gate", "check", "failure.preserves"][..],
        ]),
        artifact_refs_value(&[("failure", failure.failure_ref.as_str())]),
        string(failure.failure_ref),
        failure_value.clone(),
    ]))
}
