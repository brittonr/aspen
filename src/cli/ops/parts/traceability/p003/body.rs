
fn collect_specs_under(
    path: &std::path::Path,
    changed: bool,
    sources: &mut impl Extend<molten::requirement_traceability::SpecSource>,
) -> Outcome<()> {
    let mut pending = vec![path.to_path_buf()];
    while let Some(next) = pending.pop() {
        if !next.exists() {
            continue;
        }
        if next.is_file() {
            if next.file_name().is_some_and(|name| name == std::ffi::OsStr::new("spec.md")) {
                let markdown = std::fs::read_to_string(&next).map_err(molten::error::MoltenError::from)?;
                sources.extend([molten::requirement_traceability::SpecSource {
                    source: next.display().to_string(),
                    markdown,
                    changed,
                    default_kind: "evidence".to_string(),
                }]);
            }
            continue;
        }
        let mut entries = std::fs::read_dir(&next)
            .map_err(molten::error::MoltenError::from)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(molten::error::MoltenError::from)?;
        entries.sort_by_key(|entry| entry.path());
        // Push in reverse so the stack visits entries in sorted order and keeps
        // the depth-first order of the earlier recursion.
        for entry in entries.into_iter().rev() {
            pending.push(entry.path());
        }
    }
    Ok(())
}

fn parse_coverage_inputs(
    root: &std::path::Path,
    coverage_items: Vec<String>,
    exemption_items: Vec<String>,
) -> Outcome<Vec<molten::requirement_traceability::CoverageInput>> {
    let mut coverage = std::collections::BTreeMap::<String, molten::requirement_traceability::CoverageInput>::new();
    for item in coverage_items {
        let fields = split_fields(&item, COVERAGE_FIELDS, "coverage")?;
        let evidence = evidence_from_fields(root, &fields)?;
        let entry =
            coverage
                .entry(fields[0].clone())
                .or_insert_with(|| molten::requirement_traceability::CoverageInput {
                    requirement_id: fields[0].clone(),
                    positive: Vec::new(),
                    negative: Vec::new(),
                    exemption: None,
                });
        match fields[1].as_str() {
            "positive" => entry.positive.push(evidence),
            "negative" => entry.negative.push(evidence),
            other => {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "coverage kind {other} must be positive or negative"
                )));
            }
        }
    }
    for item in exemption_items {
        let fields = split_fields(&item, EXEMPTION_FIELDS, "exemption")?;
        let entry =
            coverage
                .entry(fields[0].clone())
                .or_insert_with(|| molten::requirement_traceability::CoverageInput {
                    requirement_id: fields[0].clone(),
                    positive: Vec::new(),
                    negative: Vec::new(),
                    exemption: None,
                });
        entry.exemption = Some(molten::requirement_traceability::CoverageExemption {
            class: fields[1].clone(),
            evidence: fields[2].clone(),
        });
    }
    Ok(coverage.into_values().collect())
}

fn parse_receipt_inputs(
    root: &std::path::Path,
    receipt_paths: Vec<FilePath>,
) -> Outcome<Vec<molten::requirement_traceability::CoverageInput>> {
    let mut sources = Vec::with_capacity(receipt_paths.len());
    for path in receipt_paths {
        let text = std::fs::read_to_string(&path).map_err(molten::error::MoltenError::from)?;
        let value = molten::preserves_rail::parse_text(&text)?;
        let receipt = molten::requirement_traceability::parse_verification_run_receipt(&value)?;
        let is_target_exists = root.join(&receipt.target).exists();
        sources.push(molten::requirement_traceability::ReceiptCoverageSource {
            value,
            target_exists: is_target_exists,
        });
    }
    molten::requirement_traceability::coverage_from_verification_receipts(&sources)
}

fn evidence_from_fields(
    root: &std::path::Path,
    fields: &[String],
) -> Outcome<molten::requirement_traceability::VerificationEvidence> {
    let target = fields[2].clone();
    let artifact_ref = fields[4].clone();
    let is_target_exists = root.join(&target).exists();
    let is_artifact_present = molten::preserves_rail::validate_content_ref(&artifact_ref).is_ok();
    Ok(molten::requirement_traceability::VerificationEvidence {
        target,
        command: fields[3].clone(),
        artifact_refs: vec![artifact_ref.clone()],
        artifact_ref,
        target_exists: is_target_exists,
        artifact_present: is_artifact_present,
        source: "compatibility".to_string(),
        receipt_ref: None,
        expected_decision: "compatibility".to_string(),
    })
}

fn split_fields(item: &str, expected: usize, label: &str) -> Outcome<Vec<String>> {
    let fields = item.split('|').map(str::to_string).collect::<Vec<_>>();
    if fields.len() != expected {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "{label} entry must have {expected} pipe-delimited fields"
        )));
    }
    if fields.iter().any(|field| field.trim().is_empty()) {
        return Err(molten::error::MoltenError::invalid_harness(format!("{label} fields must not be empty")));
    }
    Ok(fields)
}

fn write_optional_preserves(path: Option<&FilePath>, value: &preserves::IOValue) -> Outcome<()> {
    let text = molten::preserves_rail::to_text(value)?;
    write_optional_text(path, &text)
}

fn write_optional_text(path: Option<&FilePath>, text: &str) -> Outcome<()> {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
        }
        std::fs::write(path, text).map_err(molten::error::MoltenError::from)?;
    } else {
        println!("{text}");
    }
    Ok(())
}
