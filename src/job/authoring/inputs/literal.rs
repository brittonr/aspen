enum LiteralFrame<'a> {
    Value(&'a serde_json::Value, usize),
    Array(std::slice::Iter<'a, serde_json::Value>, usize, bool),
    Record(serde_json::map::Iter<'a>, usize, bool),
}

struct Buffer { text: String }

impl Buffer {
    fn append(&mut self, text: &str) -> Result<()> {
        checked_count_sum(self.text.len(), text.len(), MAX_JOB_AUTHORING_BYTES, "captured parameter bytes")?;
        self.text.push_str(text);
        Ok(())
    }
}

fn nickel_quoted(value: &str) -> Result<String> {
    if value.contains("%{") { return Err(MoltenError::invalid_harness("captured parameter interpolation is unsupported")); }
    serde_json::to_string(value).map_err(|error| MoltenError::invalid_harness(error.to_string()))
}

fn job_nickel_literal(value: &serde_json::Value, depth: usize) -> Result<String> {
    let mut budget = 0;
    bound_job_parameters(value, depth, &mut budget)?;
    let mut output = Buffer { text: String::with_capacity(budget) };
    // Admission bounds depth, so a current value plus its ancestor iterators
    // requires at most 34 slots, independent of array/record width.
    let mut pending = Vec::with_capacity(34);
    pending.push(LiteralFrame::Value(value, depth));
    // Each admitted value and each container edge consumes at most three frames.
    for _ in 0..(3 * MAX_JOB_AUTHORING_BYTES) {
        let Some(frame) = pending.pop() else { return Ok(output.text); };
        match frame {
            LiteralFrame::Value(value, depth) => {
                ensure_count_at_most(depth, 32, "captured parameter depth")?;
                match value {
                    serde_json::Value::Null => output.append("null")?,
                    serde_json::Value::Bool(value) => output.append(if *value { "true" } else { "false" })?,
                    serde_json::Value::Number(value) => {
                        let value = value.as_u64().ok_or_else(|| MoltenError::invalid_harness("captured numbers must be lossless u64"))?;
                        output.append(&value.to_string())?;
                    }
                    serde_json::Value::String(value) => output.append(&nickel_quoted(value)?)?,
                    serde_json::Value::Array(values) => {
                        ensure_count_at_most(values.len(), MAX_JOB_STAGE_VALUES, "captured parameter array")?;
                        output.append("[")?;
                        pending.push(LiteralFrame::Array(values.iter(), depth + 1, true));
                    }
                    serde_json::Value::Object(values) => {
                        ensure_count_at_most(values.len(), MAX_JOB_NODES, "captured parameter record")?;
                        output.append("{")?;
                        pending.push(LiteralFrame::Record(values.iter(), depth + 1, true));
                    }
                }
            }
            LiteralFrame::Array(mut values, depth, is_first) => {
                if let Some(value) = values.next() {
                    if !is_first { output.append(",")?; }
                    pending.push(LiteralFrame::Array(values, depth, false));
                    pending.push(LiteralFrame::Value(value, depth));
                } else { output.append("]")?; }
            }
            LiteralFrame::Record(mut values, depth, is_first) => {
                if let Some((key, value)) = values.next() {
                    ensure_count_at_most(depth, 32, "captured parameter depth")?;
                    if !is_first { output.append(",")?; }
                    output.append(&nickel_quoted(key)?)?;
                    output.append(" = ")?;
                    pending.push(LiteralFrame::Record(values, depth, false));
                    pending.push(LiteralFrame::Value(value, depth));
                } else { output.append("}")?; }
            }
        }
    }
    Err(MoltenError::invalid_harness("captured literal traversal exceeded its admitted work bound"))
}
