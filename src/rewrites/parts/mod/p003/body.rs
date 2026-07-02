
impl<'a> TextTraversal<'a> {
    fn new(input: TextTraversalInput<'a>) -> Result<Self> {
        let mut traversal = Self {
            from: input.from,
            to: input.to,
            changed_paths: input.changed_paths,
            frames: Vec::with_capacity(1),
            outputs: Vec::with_capacity(1),
        };
        traversal.push_frame(TextFrame::Visit {
            value: input.value.clone(),
            path: input.path.to_string(),
        })?;
        Ok(traversal)
    }

    fn run(&mut self) -> Result<()> {
        while let Some(frame) = self.frames.pop() {
            match frame {
                TextFrame::Visit { value, path } => self.visit(value, path)?,
                TextFrame::FinishRecord {
                    original,
                    label,
                    child_count,
                    changed_count_before,
                } => self.finish_record(original, label, child_count, changed_count_before)?,
                TextFrame::FinishSequence {
                    original,
                    child_count,
                    changed_count_before,
                } => self.finish_sequence(original, child_count, changed_count_before)?,
            }
        }
        Ok(())
    }

    fn output(mut self) -> Result<IoValue> {
        let output_count = self.outputs.len();
        if output_count != 1 {
            return Err(MoltenError::invalid_harness(format!("rewrite traversal produced {output_count} outputs")));
        }
        self.outputs
            .pop()
            .ok_or_else(|| MoltenError::invalid_harness("rewrite traversal produced no output"))
    }

    fn visit(&mut self, current: IoValue, current_path: String) -> Result<()> {
        if current.as_string().is_some_and(|text| text.as_ref() == self.from) {
            push_bounded(&mut *self.changed_paths, current_path, MAX_REWRITE_ITEMS, "rewrite changed paths")?;
            self.push_output(string(self.to))?;
            return Ok(());
        }
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => self.push_output(current),
            ValueClass::Compound(CompoundClass::Record) => self.visit_record(current, current_path),
            ValueClass::Compound(CompoundClass::Sequence) => self.visit_sequence(current, current_path),
            ValueClass::Compound(CompoundClass::Set) | ValueClass::Compound(CompoundClass::Dictionary) => {
                self.push_output(current)
            }
        }
    }

    fn visit_record(&mut self, current: IoValue, current_path: String) -> Result<()> {
        let label = value_to_iovalue(&current.label());
        let label_name = record_label_name(&current);
        let changed_count_before = self.changed_paths.len();
        let mut children = Vec::new();
        for (index, child) in current.iter().enumerate() {
            push_bounded(
                &mut children,
                TextFrame::Visit {
                    value: value_to_iovalue(&child),
                    path: format!("{current_path}/{label_name}/{index}"),
                },
                MAX_REWRITE_ITEMS,
                "rewrite traversal child frames",
            )?;
        }
        self.push_frame(TextFrame::FinishRecord {
            original: current,
            label,
            child_count: children.len(),
            changed_count_before,
        })?;
        self.push_children(children)
    }

    fn visit_sequence(&mut self, current: IoValue, current_path: String) -> Result<()> {
        let changed_count_before = self.changed_paths.len();
        let mut children = Vec::new();
        for (index, child) in current.iter().enumerate() {
            push_bounded(
                &mut children,
                TextFrame::Visit {
                    value: value_to_iovalue(&child),
                    path: format!("{current_path}/{index}"),
                },
                MAX_REWRITE_ITEMS,
                "rewrite traversal child frames",
            )?;
        }
        self.push_frame(TextFrame::FinishSequence {
            original: current,
            child_count: children.len(),
            changed_count_before,
        })?;
        self.push_children(children)
    }

    fn finish_record(
        &mut self,
        original: IoValue,
        label: IoValue,
        child_count: usize,
        changed_count_before: usize,
    ) -> Result<()> {
        let fields = self.take_child_outputs(child_count, "rewrite record output count underflow")?;
        let rewritten = if self.changed_paths.len() == changed_count_before {
            original
        } else {
            IoValue::record(label, fields)
        };
        self.push_output(rewritten)
    }

    fn finish_sequence(&mut self, original: IoValue, child_count: usize, changed_count_before: usize) -> Result<()> {
        let items = self.take_child_outputs(child_count, "rewrite sequence output count underflow")?;
        let rewritten = if self.changed_paths.len() == changed_count_before {
            original
        } else {
            sequence(items)
        };
        self.push_output(rewritten)
    }

    fn take_child_outputs(&mut self, child_count: usize, label: &str) -> Result<Vec<IoValue>> {
        let start = self.outputs.len().checked_sub(child_count).ok_or_else(|| MoltenError::invalid_harness(label))?;
        Ok(self.outputs.split_off(start))
    }

    fn push_children(&mut self, children: Vec<TextFrame>) -> Result<()> {
        for child in children.into_iter().rev() {
            self.push_frame(child)?;
        }
        Ok(())
    }

    fn push_frame(&mut self, frame: TextFrame) -> Result<()> {
        push_bounded(&mut self.frames, frame, MAX_REWRITE_ITEMS, "rewrite traversal frames")
    }

    fn push_output(&mut self, output: IoValue) -> Result<()> {
        push_bounded(&mut self.outputs, output, MAX_REWRITE_ITEMS, "rewrite traversal outputs")
    }
}

fn scoped_refs(root: &Path, roots: &[String], include_dependencies: bool) -> Result<OrderedSet<String>> {
    validate_refs(roots, "rewrite scope root ref")?;
    let mut scoped = OrderedSet::new();
    let mut stack = roots.to_vec();
    while let Some(current) = stack.pop() {
        if !scoped.insert(current.clone()) || !include_dependencies {
            continue;
        }
        for dependency in crate::artifacts::direct_dependencies(root, &current)? {
            stack.push(dependency);
        }
    }
    Ok(scoped)
}

fn impacted_refs(root: &Path, diffs: &[RewriteDiff]) -> Result<Vec<String>> {
    let mut impacted = OrderedSet::new();
    for diff in diffs {
        for reference in crate::artifacts::impact_refs(root, std::slice::from_ref(&diff.artifact_ref))? {
            impacted.insert(reference);
        }
    }
    Ok(impacted.into_iter().collect())
}

fn preview_text(value: &IoValue) -> Result<String> {
    let text = to_text(value)?;
    const LIMIT: usize = 240;
    if text.chars().count() > LIMIT {
        let mut truncated = text.chars().take(LIMIT).collect::<String>();
        truncated.push('…');
        Ok(truncated)
    } else {
        Ok(text)
    }
}

fn record_label_name(value: &IoValue) -> String {
    value.label().as_symbol().map(|label| label.into_owned()).unwrap_or_else(|| "record".to_string())
}

fn validate_query_input(input: &RewriteQueryInput) -> Result<()> {
    validate_refs(&input.root_refs, "rewrite query root ref")?;
    validate_refs(&input.policy_refs, "rewrite query policy ref")?;
    validate_refs(&input.capability_refs, "rewrite query capability ref")?;
    validate_refs(&input.hidden_refs, "rewrite query hidden ref")?;
    for kind in &input.artifact_kinds {
        validate_non_empty(kind, "rewrite query artifact kind")?;
    }
    validate_pattern(&input.pattern)
}

fn validate_plan_input(input: &RewritePlanInput) -> Result<()> {
    validate_query_input(&input.query)?;
    validate_ref(&input.planner_ref, "rewrite planner ref")?;
    validate_refs(&input.policy_refs, "rewrite plan policy ref")?;
    validate_refs(&input.capability_refs, "rewrite plan capability ref")?;
    validate_refs(&input.transcript_refs, "rewrite plan transcript ref")?;
    validate_refs(&input.schema_migration_recipe_refs, "rewrite plan schema migration recipe ref")?;
    if input.policy_refs.is_empty() {
        return Err(MoltenError::invalid_harness("rewrite plan requires explicit policy refs"));
    }
    if input.capability_refs.is_empty() {
        return Err(MoltenError::invalid_harness("rewrite plan requires explicit capability refs"));
    }
    match &input.replacement {
        RewriteReplacement::StringValue { from, .. } => validate_non_empty(from, "rewrite replacement from string"),
    }
}

fn validate_pattern(pattern: &RewritePattern) -> Result<()> {
    match pattern {
        RewritePattern::Any => Ok(()),
        RewritePattern::ArtifactKind(value)
        | RewritePattern::RecordLabel(value)
        | RewritePattern::StringEquals(value)
        | RewritePattern::StringContains(value)
        | RewritePattern::SchemaShapeKind(value) => validate_non_empty(value, "rewrite pattern"),
        RewritePattern::RefContains(value) => validate_ref(value, "rewrite ref pattern"),
    }
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "rewrite checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(&item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "rewrite check name")?;
        let status = required_string(&check[1], "rewrite check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("rewrite check {name} has status {status}")));
        }
        push_bounded(&mut parsed, name, MAX_REWRITE_ITEMS, "rewrite checks")?;
    }
    Ok(parsed)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}
