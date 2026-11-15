enum ParameterFrame<'a> {
    Value(&'a serde_json::Value, usize),
    Array(std::slice::Iter<'a, serde_json::Value>, usize),
    Record(serde_json::map::Iter<'a>, usize),
}

fn bound_job_parameters(value: &serde_json::Value, depth: usize, used: &mut usize) -> Result<()> {
    // Only one iterator per ancestor and the current value are retained. Even a
    // very wide untrusted array cannot expand the worklist before admission.
    let mut pending = Vec::with_capacity(34);
    pending.push(ParameterFrame::Value(value, depth));
    // Each admitted value and each container edge consumes at most three frames.
    for _ in 0..(3 * MAX_JOB_AUTHORING_BYTES) {
        let Some(frame) = pending.pop() else { return Ok(()); };
        match frame {
            ParameterFrame::Value(value, depth) => {
                ensure_count_at_most(depth, 32, "captured parameter depth")?;
                *used = checked_count_sum(*used, 1, MAX_JOB_AUTHORING_BYTES, "captured parameter work")?;
                match value {
                    serde_json::Value::String(value) => {
                        *used = checked_count_sum(*used, value.len().saturating_mul(6), MAX_JOB_AUTHORING_BYTES, "captured string bytes")?;
                    }
                    serde_json::Value::Array(values) => pending.push(ParameterFrame::Array(values.iter(), depth + 1)),
                    serde_json::Value::Object(values) => pending.push(ParameterFrame::Record(values.iter(), depth + 1)),
                    _ => { *used = checked_count_sum(*used, 24, MAX_JOB_AUTHORING_BYTES, "captured scalar bytes")?; }
                }
            }
            ParameterFrame::Array(mut values, depth) => {
                if let Some(value) = values.next() {
                    pending.push(ParameterFrame::Array(values, depth));
                    pending.push(ParameterFrame::Value(value, depth));
                }
            }
            ParameterFrame::Record(mut values, depth) => {
                if let Some((key, value)) = values.next() {
                    *used = checked_count_sum(*used, key.len().saturating_mul(6), MAX_JOB_AUTHORING_BYTES, "captured key bytes")?;
                    pending.push(ParameterFrame::Record(values, depth));
                    pending.push(ParameterFrame::Value(value, depth));
                }
            }
        }
    }
    Err(MoltenError::invalid_harness("captured parameter traversal exceeded its admitted work bound"))
}
