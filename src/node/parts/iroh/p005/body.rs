
fn render_openmetrics(
    input: &MetricsSnapshotInput,
    diagnostics: &mut impl DiagnosticSink,
) -> crate::error::Result<String> {
    let mut output = String::new();
    let mut names = std::collections::BTreeSet::new();
    for sample in &input.samples {
        validate_metric_sample(sample, diagnostics)?;
        names.insert(sample.name.as_str());
        output.push_str("# TYPE ");
        output.push_str(&sample.name);
        output.push(' ');
        output.push_str(&sample.kind);
        output.push('\n');
        output.push_str(&sample.name);
        if !sample.labels.is_empty() {
            output.push('{');
            for (index, (key, value)) in sample.labels.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(key);
                output.push_str("=\"");
                output.push_str(value);
                output.push('"');
            }
            output.push('}');
        }
        output.push(' ');
        output.push_str(&sample.value.to_string());
        output.push('\n');
    }
    if names.is_empty() {
        push_diagnostic(diagnostics, "metrics snapshot requires at least one sample")?;
    }
    Ok(output)
}

fn validate_metric_sample(sample: &MetricSample, diagnostics: &mut impl DiagnosticSink) -> crate::error::Result<()> {
    validate_bounded_text(&sample.name, "metric name", MAX_METRIC_NAME_BYTES, diagnostics)?;
    if !sample.name.bytes().all(is_metric_name_byte) {
        push_diagnostic(diagnostics, "metric name contains unsupported characters")?;
    }
    validate_status(&sample.kind, &["counter", "gauge", "histogram"], "metric kind")?;
    validate_bounded_value_count(sample.labels.len(), MAX_METRIC_LABELS, "metric label")?;
    for (key, value) in &sample.labels {
        validate_bounded_text(key, "metric label key", MAX_METRIC_LABEL_BYTES, diagnostics)?;
        validate_bounded_text(value, "metric label value", MAX_METRIC_LABEL_BYTES, diagnostics)?;
        if label_leaks_sensitive_value(key, value) {
            push_diagnostic(diagnostics, format!("metric label {key} leaks sensitive or high-cardinality data"))?;
        }
    }
    Ok(())
}

fn is_metric_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':')
}

fn label_leaks_sensitive_value(key: &str, value: &str) -> bool {
    let key_lower = key.to_ascii_lowercase();
    let value_lower = value.to_ascii_lowercase();
    key_lower.contains("secret")
        || key_lower.contains("ticket")
        || key_lower.contains("path")
        || key_lower.contains("peer_id")
        || value_lower.contains("secret")
        || value_lower.contains("ticket")
        || value_lower.starts_with("/home/")
        || value_lower.starts_with("blake3:")
}

pub fn fixture_ref(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

pub fn default_limit_profile_ref() -> String {
    fixture_ref(DEFAULT_LIMIT_PROFILE)
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/iroh/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/iroh/tests/m000/p001/body.rs"));
}
