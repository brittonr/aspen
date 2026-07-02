
fn rewrite_plan_value(
    input: &RewritePlanInput,
    query: &RewriteQuery,
    diffs: &[RewriteDiff],
    impacted_refs: &[String],
) -> Result<IoValue> {
    Ok(record("rewrite-plan-v1", vec![
        string(crate::preserves_rail::REWRITE_PLAN_SCHEMA),
        record("planner", vec![string(&input.planner_ref), refs_sequence(&input.capability_refs)]),
        record("query", vec![query.query_value.clone(), string(&query.query_ref)]),
        replacement_value(&input.replacement)?,
        record("matches", vec![sequence(
            query.matches.iter().map(|rewrite_match| rewrite_match.value.clone()).collect(),
        )]),
        record("diffs", vec![sequence(diffs.iter().map(|diff| diff.value.clone()).collect())]),
        record("impact", vec![refs_sequence(impacted_refs)]),
        record("transcripts", vec![refs_sequence(&input.transcript_refs)]),
        record("schema-migrations", vec![refs_sequence(&input.schema_migration_recipe_refs)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&[
            "dry-run-preview",
            "artifact-creation-required",
            "no-in-place-mutation",
            "upgrade-session-hook-ready",
            "transcript-validation-hook",
            "schema-migration-hook",
        ]),
    ]))
}

fn rewrite_match_value(
    artifact_ref: &str,
    kind: &str,
    payload_ref: &str,
    bindings: &[RewriteBinding],
) -> Result<IoValue> {
    validate_ref(artifact_ref, "rewrite match artifact ref")?;
    validate_ref(payload_ref, "rewrite match payload ref")?;
    validate_non_empty(kind, "rewrite match kind")?;
    Ok(record("rewrite-match-v1", vec![
        string(crate::preserves_rail::REWRITE_MATCH_SCHEMA),
        record("artifact", vec![string(artifact_ref), string(kind), string(payload_ref)]),
        record("paths", vec![sequence(bindings.iter().map(|binding| string(&binding.path)).collect())]),
        record("bindings", vec![sequence(
            bindings
                .iter()
                .map(|binding| {
                    record("binding", vec![
                        string(&binding.path),
                        string(&binding.value_ref),
                        string(&binding.preview),
                    ])
                })
                .collect(),
        )]),
        checks_value(&["canonical-binding-ref", "bounded-path", "visible-result"]),
    ]))
}

struct RewriteDiffValueInput<'a> {
    artifact_ref: &'a str,
    kind: &'a str,
    old_payload_ref: &'a str,
    new_payload_ref: &'a str,
    paths: &'a [String],
    old_preview: &'a str,
    new_preview: &'a str,
}

struct RewriteReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    subject_ref: &'a str,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn rewrite_diff_value(input: &RewriteDiffValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.artifact_ref, "rewrite diff artifact ref")?;
    validate_ref(input.old_payload_ref, "rewrite diff old payload ref")?;
    validate_ref(input.new_payload_ref, "rewrite diff new payload ref")?;
    Ok(record("rewrite-diff-v1", vec![
        string(crate::preserves_rail::REWRITE_DIFF_SCHEMA),
        record("artifact", vec![string(input.artifact_ref), string(input.kind)]),
        record("payload", vec![string(input.old_payload_ref), string(input.new_payload_ref)]),
        record("paths", vec![sequence(input.paths.iter().map(string).collect())]),
        record("preview", vec![string(input.old_preview), string(input.new_preview)]),
        checks_value(&["structural-rewrite", "old-artifact-preserved", "canonical-new-payload"]),
    ]))
}

fn pattern_value(pattern: &RewritePattern) -> Result<IoValue> {
    let (kind, needle) = match pattern {
        RewritePattern::Any => ("any", ""),
        RewritePattern::ArtifactKind(value) => ("artifact-kind", value.as_str()),
        RewritePattern::RecordLabel(value) => ("record-label", value.as_str()),
        RewritePattern::StringEquals(value) => ("string-equals", value.as_str()),
        RewritePattern::StringContains(value) => ("string-contains", value.as_str()),
        RewritePattern::SchemaShapeKind(value) => ("schema-shape-kind", value.as_str()),
        RewritePattern::RefContains(value) => ("ref-contains", value.as_str()),
    };
    validate_pattern(pattern)?;
    Ok(record("pattern", vec![
        string(kind),
        string(needle),
        checks_value(&["bounded-preserves-pattern", "no-ambient-code"]),
    ]))
}

fn replacement_value(replacement: &RewriteReplacement) -> Result<IoValue> {
    match replacement {
        RewriteReplacement::StringValue { from, to } => {
            validate_non_empty(from, "rewrite replacement from string")?;
            Ok(record("replacement", vec![
                string("string-value"),
                record("from", vec![string(from)]),
                record("to", vec![string(to)]),
                checks_value(&["structural-value-replacement", "canonical-reparse-not-text-bypass"]),
            ]))
        }
    }
}

fn rewrite_receipt_value(input: &RewriteReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "rewrite receipt operation")?;
    if !matches!(input.decision, "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!("unsupported rewrite decision {}", input.decision)));
    }
    validate_ref(input.subject_ref, "rewrite receipt subject ref")?;
    validate_refs(input.refs, "rewrite receipt ref")?;
    let mut all_checks = vec![("canonical-receipt", "pass")];
    all_checks.extend_from_slice(input.checks);
    Ok(record("rewrite-receipt-v1", vec![
        string(crate::preserves_rail::REWRITE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("subject", vec![string(input.subject_ref)]),
        record("refs", vec![refs_sequence(&sorted_unique_refs(input.refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("tool", vec![string(TOOL_VERSION)]),
        checks_value_from_pairs(&all_checks),
    ]))
}

fn collect_bindings(
    value: &IoValue,
    pattern: &RewritePattern,
    path: &str,
    bindings: &mut impl crate::bounded::VecSink<RewriteBinding>,
) -> Result<()> {
    let mut pending = Vec::with_capacity(1);
    push_bounded(&mut pending, (value.clone(), path.to_string()), MAX_REWRITE_ITEMS, "rewrite scan values")?;
    while let Some((current, current_path)) = pending.pop() {
        if value_matches_pattern(&current, pattern) {
            push_bounded(
                bindings,
                RewriteBinding {
                    path: current_path.clone(),
                    value_ref: canonical_hash(&current)?,
                    preview: preview_text(&current)?,
                },
                MAX_REWRITE_ITEMS,
                "rewrite bindings",
            )?;
        }
        let mut children = Vec::new();
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => {}
            ValueClass::Compound(CompoundClass::Record) => {
                let label = record_label_name(&current);
                for (index, child) in current.iter().enumerate() {
                    push_bounded(
                        &mut children,
                        (value_to_iovalue(&child), format!("{current_path}/{label}/{index}")),
                        MAX_REWRITE_ITEMS,
                        "rewrite scan child values",
                    )?;
                }
            }
            ValueClass::Compound(CompoundClass::Sequence) | ValueClass::Compound(CompoundClass::Set) => {
                for (index, child) in current.iter().enumerate() {
                    push_bounded(
                        &mut children,
                        (value_to_iovalue(&child), format!("{current_path}/{index}")),
                        MAX_REWRITE_ITEMS,
                        "rewrite scan child values",
                    )?;
                }
            }
            ValueClass::Compound(CompoundClass::Dictionary) => {
                for (index, (key, child)) in current.entries().enumerate() {
                    push_bounded(
                        &mut children,
                        (value_to_iovalue(&key), format!("{current_path}/key/{index}")),
                        MAX_REWRITE_ITEMS,
                        "rewrite scan child values",
                    )?;
                    push_bounded(
                        &mut children,
                        (value_to_iovalue(&child), format!("{current_path}/value/{index}")),
                        MAX_REWRITE_ITEMS,
                        "rewrite scan child values",
                    )?;
                }
            }
        }
        for child in children.into_iter().rev() {
            push_bounded(&mut pending, child, MAX_REWRITE_ITEMS, "rewrite scan values")?;
        }
    }
    Ok(())
}

fn value_matches_pattern(value: &IoValue, pattern: &RewritePattern) -> bool {
    match pattern {
        RewritePattern::Any => true,
        RewritePattern::ArtifactKind(_) => false,
        RewritePattern::RecordLabel(expected) => {
            value.is_record() && value.label().as_symbol().is_some_and(|label| label.as_ref() == expected.as_str())
        }
        RewritePattern::StringEquals(expected) => {
            value.as_string().is_some_and(|text| text.as_ref() == expected.as_str())
        }
        RewritePattern::StringContains(needle) => value.as_string().is_some_and(|text| text.contains(needle.as_str())),
        RewritePattern::SchemaShapeKind(expected) => value
            .collect_simple_record("shape", None)
            .and_then(|fields| {
                if fields.len() == 0 {
                    None
                } else {
                    fields[0].as_string().map(|text| text.into_owned())
                }
            })
            .is_some_and(|kind| kind == expected.as_str()),
        RewritePattern::RefContains(needle) => to_text(value).is_ok_and(|text| text.contains(needle.as_str())),
    }
}

struct RewriteStringValuesInput<'a> {
    value: &'a IoValue,
    from: &'a str,
    to: &'a str,
    path: &'a str,
    changed_paths: &'a mut Vec<String>,
}

fn rewrite_string_values(input: RewriteStringValuesInput<'_>) -> Result<IoValue> {
    let mut traversal = TextTraversal::new(TextTraversalInput {
        value: input.value,
        from: input.from,
        to: input.to,
        path: input.path,
        changed_paths: input.changed_paths,
    })?;
    traversal.run()?;
    traversal.output()
}

enum TextFrame {
    Visit {
        value: IoValue,
        path: String,
    },
    FinishRecord {
        original: IoValue,
        label: IoValue,
        child_count: usize,
        changed_count_before: usize,
    },
    FinishSequence {
        original: IoValue,
        child_count: usize,
        changed_count_before: usize,
    },
}

struct TextTraversalInput<'a> {
    value: &'a IoValue,
    from: &'a str,
    to: &'a str,
    path: &'a str,
    changed_paths: &'a mut Vec<String>,
}

struct TextTraversal<'a> {
    from: &'a str,
    to: &'a str,
    changed_paths: &'a mut Vec<String>,
    frames: Vec<TextFrame>,
    outputs: Vec<IoValue>,
}
