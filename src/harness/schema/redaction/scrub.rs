use preserves::ValueImpl;

const MAX_REDACTION_TRANSFORM_NODES: usize = 100_000;

const MAX_REDACTION_CONTAINER_ITEMS: usize = 100_000;

const _: () = assert!(MAX_REDACTION_TRANSFORM_NODES <= 1_000_000);

const _: () = assert!(MAX_REDACTION_CONTAINER_ITEMS <= 1_000_000);

enum RedactionTraversalFrame {
    Enter {
        value: preserves::IOValue,
        path: String,
    },
    ExitRecord {
        original: preserves::IOValue,
        label: preserves::IOValue,
        field_count: usize,
    },
    ExitSequence {
        original: preserves::IOValue,
        item_count: usize,
    },
}

pub(super) fn ensure_redaction_bound(count: usize, limit: usize, context: &str) -> crate::error::Result<()> {
    if count > limit {
        return Err(crate::error::Failure::invalid_harness(format!(
            "{context} exceeds redaction transform bound {limit}"
        )));
    }
    Ok(())
}

struct RedactionFrameStack {
    frames: Vec<RedactionTraversalFrame>,
}

impl RedactionFrameStack {
    fn new() -> Self {
        Self {
            frames: Vec::with_capacity(1),
        }
    }

    fn push(&mut self, frame: RedactionTraversalFrame) -> crate::error::Result<()> {
        ensure_redaction_bound(self.frames.len() + 1, MAX_REDACTION_TRANSFORM_NODES, "redaction traversal stack")?;
        self.frames.push(frame);
        Ok(())
    }

    fn pop(&mut self) -> Option<RedactionTraversalFrame> {
        self.frames.pop()
    }

    fn push_children(&mut self, child_entries: Vec<(preserves::IOValue, String)>) -> crate::error::Result<()> {
        ensure_redaction_bound(
            self.frames.len() + child_entries.len(),
            MAX_REDACTION_TRANSFORM_NODES,
            "redaction traversal stack",
        )?;
        for (child_value, child_path) in child_entries.into_iter().rev() {
            self.push(RedactionTraversalFrame::Enter {
                value: child_value,
                path: child_path,
            })?;
        }
        Ok(())
    }
}

struct RedactionOutputStack {
    values: Vec<preserves::IOValue>,
}

impl RedactionOutputStack {
    fn new() -> Self {
        Self {
            values: Vec::with_capacity(1),
        }
    }

    fn push(&mut self, value: preserves::IOValue) -> crate::error::Result<()> {
        ensure_redaction_bound(self.values.len() + 1, MAX_REDACTION_TRANSFORM_NODES, "redaction traversal outputs")?;
        self.values.push(value);
        Ok(())
    }

    fn take(&mut self, count: usize) -> crate::error::Result<Vec<preserves::IOValue>> {
        if self.values.len() < count {
            return Err(crate::error::Failure::invalid_harness("redaction traversal stack underflow"));
        }
        Ok(self.values.split_off(self.values.len() - count))
    }

    fn finish(mut self) -> crate::error::Result<preserves::IOValue> {
        if self.values.len() != 1 {
            return Err(crate::error::Failure::invalid_harness("redaction traversal produced invalid output"));
        }
        self.values
            .pop()
            .ok_or_else(|| crate::error::Failure::invalid_harness("redaction traversal produced no output"))
    }
}

fn bounded_redaction_child_count(value: &preserves::IOValue, context: &str) -> crate::error::Result<usize> {
    let count = value.iter().count();
    ensure_redaction_bound(count, MAX_REDACTION_CONTAINER_ITEMS, context)?;
    Ok(count)
}

fn redaction_child_entries(
    value: &preserves::IOValue,
    path: &str,
    context: &str,
) -> crate::error::Result<Vec<(preserves::IOValue, String)>> {
    let child_count = bounded_redaction_child_count(value, context)?;
    let mut entries = Vec::with_capacity(child_count);
    for (index, child) in value.iter().enumerate() {
        entries.push((crate::preserves_rail::value_to_iovalue(&child), format!("{path}/{index}")));
    }
    Ok(entries)
}

pub(super) struct RedactionTraversal<'a> {
    stack: RedactionFrameStack,
    outputs: RedactionOutputStack,
    state: &'a mut super::masking::RedactionTransformState,
}

impl<'a> RedactionTraversal<'a> {
    pub(super) fn new(state: &'a mut super::masking::RedactionTransformState) -> Self {
        Self {
            stack: RedactionFrameStack::new(),
            outputs: RedactionOutputStack::new(),
            state,
        }
    }

    pub(super) fn run(mut self, value: &preserves::IOValue, path: &str) -> crate::error::Result<preserves::IOValue> {
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

    fn handle(&mut self, frame: RedactionTraversalFrame) -> crate::error::Result<()> {
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

    fn enter(&mut self, value: preserves::IOValue, path: String) -> crate::error::Result<()> {
        if let Some(label) = super::masking::record_label_string(&value)
            && super::disclosure::is_sensitive_record_label(&label)
        {
            let redacted = super::masking::transform_sensitive_record(&value, &label, &path, self.state)?;
            return self.outputs.push(redacted);
        }
        match value.value_class() {
            preserves::ValueClass::Atomic(_) | preserves::ValueClass::Embedded => self.outputs.push(value),
            preserves::ValueClass::Compound(preserves::CompoundClass::Record) => self.enter_record(value, path),
            preserves::ValueClass::Compound(preserves::CompoundClass::Sequence) => self.enter_sequence(value, path),
            preserves::ValueClass::Compound(preserves::CompoundClass::Set)
            | preserves::ValueClass::Compound(preserves::CompoundClass::Dictionary) => self.outputs.push(value),
        }
    }

    fn enter_record(&mut self, value: preserves::IOValue, path: String) -> crate::error::Result<()> {
        let label = crate::preserves_rail::value_to_iovalue(&value.label());
        let child_entries = redaction_child_entries(&value, &path, "redaction record fields")?;
        self.stack.push(RedactionTraversalFrame::ExitRecord {
            original: value,
            label,
            field_count: child_entries.len(),
        })?;
        self.stack.push_children(child_entries)
    }

    fn enter_sequence(&mut self, value: preserves::IOValue, path: String) -> crate::error::Result<()> {
        let child_entries = redaction_child_entries(&value, &path, "redaction sequence items")?;
        self.stack.push(RedactionTraversalFrame::ExitSequence {
            original: value,
            item_count: child_entries.len(),
        })?;
        self.stack.push_children(child_entries)
    }

    fn exit_record(
        &mut self,
        original: preserves::IOValue,
        label: preserves::IOValue,
        field_count: usize,
    ) -> crate::error::Result<()> {
        let fields = self.outputs.take(field_count)?;
        self.push_rebuilt(original, preserves::IOValue::record(label, fields))
    }

    fn exit_sequence(&mut self, original: preserves::IOValue, item_count: usize) -> crate::error::Result<()> {
        let values = self.outputs.take(item_count)?;
        self.push_rebuilt(original, crate::preserves_rail::sequence(values))
    }

    fn push_rebuilt(&mut self, original: preserves::IOValue, rebuilt: preserves::IOValue) -> crate::error::Result<()> {
        if rebuilt == original {
            self.outputs.push(original)
        } else {
            self.outputs.push(rebuilt)
        }
    }
}
