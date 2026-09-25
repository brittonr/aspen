
struct StructuralScanState {
    visited_nodes: usize,
    limits: StructuralInspectionLimits,
}

// r[impl molten.preserves_value_inspection.structural_scan]
pub fn find_structural_match<F>(
    value: &IoValue,
    scope: StructuralInspectionScope,
    predicate: F,
) -> Result<Option<StructuralMatch>>
where
    F: FnMut(StructuralTokenKind, &str) -> bool,
{
    find_structural_match_with_limits(value, scope, StructuralInspectionLimits::default(), predicate)
}

pub fn find_structural_match_with_limits<F>(
    value: &IoValue,
    scope: StructuralInspectionScope,
    limits: StructuralInspectionLimits,
    mut predicate: F,
) -> Result<Option<StructuralMatch>>
where
    F: FnMut(StructuralTokenKind, &str) -> bool,
{
    let mut state = StructuralScanState {
        visited_nodes: 0,
        limits,
    };
    visit_structural_value(value, scope, &mut predicate, &mut state)
}

pub fn find_named_structural_marker(value: &IoValue, markers: &[&str]) -> Result<Option<StructuralMatch>> {
    find_structural_match(value, StructuralInspectionScope::structural_markers(), |kind, token| {
        matches!(kind, StructuralTokenKind::RecordLabel | StructuralTokenKind::Symbol) && markers.contains(&token)
    })
}

// r[impl molten.preserves_value_inspection.marker_detection]
pub fn find_sensitive_structural_marker(value: &IoValue) -> Result<Option<StructuralMatch>> {
    find_named_structural_marker(value, SENSITIVE_STRUCTURAL_MARKERS)
}

// r[impl molten.preserves_value_inspection.ambient_token_denial]
pub fn find_ambient_job_token(value: &IoValue) -> Result<Option<StructuralMatch>> {
    find_named_structural_marker(value, AMBIENT_JOB_TOKENS)
}

// r[impl molten.preserves_value_inspection.ref_retention]
pub fn find_structural_content_ref(value: &IoValue, target_ref: &str) -> Result<Option<StructuralMatch>> {
    let target = ContentRef::parse(target_ref)?;
    find_structural_match(value, StructuralInspectionScope::content_refs(), |kind, token| {
        kind == StructuralTokenKind::ContentRef && token == target.as_str()
    })
}

pub fn contains_structural_content_ref(value: &IoValue, target_ref: &str) -> Result<bool> {
    Ok(find_structural_content_ref(value, target_ref)?.is_some())
}

// The scan keeps an explicit stack of open containers instead of recursing. Each frame holds the not yet visited
// children of one container on the current path, so the walk keeps the preorder, the first match, and the first
// bound failure of a recursive walk. The stack never exceeds the admitted depth.
fn visit_structural_value<F>(
    value: &IoValue,
    scope: StructuralInspectionScope,
    predicate: &mut F,
    state: &mut StructuralScanState,
) -> Result<Option<StructuralMatch>>
where
    F: FnMut(StructuralTokenKind, &str) -> bool,
{
    let mut path = vec!["$".to_string()];
    let mut frames = Vec::new();
    match inspect_structural_node(value, scope, predicate, state, &path)? {
        StructuralNodeOutcome::Matched(found) => return Ok(Some(found)),
        StructuralNodeOutcome::Children(children) => open_structural_frame(&mut frames, children, state)?,
    }
    while let Some(frame) = frames.last_mut() {
        let Some((segment, child)) = frame.next() else {
            frames.pop();
            path.pop();
            continue;
        };
        path.push(segment);
        match inspect_structural_node(&child, scope, predicate, state, &path)? {
            StructuralNodeOutcome::Matched(found) => return Ok(Some(found)),
            StructuralNodeOutcome::Children(children) if children.is_empty() => {
                path.pop();
            }
            StructuralNodeOutcome::Children(children) => open_structural_frame(&mut frames, children, state)?,
        }
    }
    Ok(None)
}

enum StructuralNodeOutcome {
    Matched(StructuralMatch),
    Children(Vec<(String, IoValue)>),
}

fn open_structural_frame(
    frames: &mut impl crate::bounded::VecSink<std::vec::IntoIter<(String, IoValue)>>,
    children: Vec<(String, IoValue)>,
    state: &StructuralScanState,
) -> Result<()> {
    if children.is_empty() {
        return Ok(());
    }
    crate::bounded::push_bounded(
        frames,
        children.into_iter(),
        state.limits.max_depth,
        "structural Preserves scan open containers",
    )
}

// Counts and bounds one node, applies the scope predicates, and returns its children in preorder. Children past the
// remaining node budget are not materialized, because reaching any of them would first exceed `max_nodes`.
fn inspect_structural_node<F>(
    value: &IoValue,
    scope: StructuralInspectionScope,
    predicate: &mut F,
    state: &mut StructuralScanState,
    path: &[String],
) -> Result<StructuralNodeOutcome>
where
    F: FnMut(StructuralTokenKind, &str) -> bool,
{
    state.visited_nodes = state
        .visited_nodes
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("structural Preserves scan node count overflow"))?;
    if state.visited_nodes > state.limits.max_nodes {
        return Err(MoltenError::invalid_harness(format!(
            "structural Preserves scan exceeded {} nodes",
            state.limits.max_nodes
        )));
    }
    if path.len() > state.limits.max_depth {
        return Err(MoltenError::invalid_harness(format!(
            "structural Preserves scan exceeded depth {}",
            state.limits.max_depth
        )));
    }
    let child_budget = state.limits.max_nodes.saturating_sub(state.visited_nodes).saturating_add(1);
    let matched = |kind, token: &str| Ok(StructuralNodeOutcome::Matched(structural_match(kind, token, path)));

    if value.is_record() {
        let label = value.label();
        if let Some(name) = label.as_symbol()
            && scope.record_labels
            && predicate(StructuralTokenKind::RecordLabel, name.as_ref())
        {
            return matched(StructuralTokenKind::RecordLabel, name.as_ref());
        }
        let fields =
            value.iter().enumerate().map(|(index, child)| (format!("field[{index}]"), value_to_iovalue(&child)));
        let children = std::iter::once(("label".to_string(), value_to_iovalue(&label))).chain(fields);
        return Ok(StructuralNodeOutcome::Children(children.take(child_budget).collect()));
    }

    if let Some(symbol) = value.as_symbol()
        && scope.symbols
        && predicate(StructuralTokenKind::Symbol, symbol.as_ref())
    {
        return matched(StructuralTokenKind::Symbol, symbol.as_ref());
    }
    if let Some(text) = value.as_string() {
        if scope.strings && predicate(StructuralTokenKind::String, text.as_ref()) {
            return matched(StructuralTokenKind::String, text.as_ref());
        }
        if scope.content_refs && ContentRef::parse(text.as_ref()).is_ok()
            && predicate(StructuralTokenKind::ContentRef, text.as_ref())
        {
            return matched(StructuralTokenKind::ContentRef, text.as_ref());
        }
    }
    if let Some(bytes) = value.as_bytestring()
        && scope.byte_strings
    {
        let token = content_ref_from_bytes(bytes.as_ref());
        if predicate(StructuralTokenKind::ByteString, &token) {
            return matched(StructuralTokenKind::ByteString, &token);
        }
    }

    Ok(StructuralNodeOutcome::Children(container_children(value, child_budget)))
}

/// The labelled items of a sequence or set, or the labelled keys and values of a dictionary, within the budget.
fn container_children(value: &IoValue, child_budget: usize) -> Vec<(String, IoValue)> {
    if value.is_sequence() || value.is_set() {
        value
            .iter()
            .enumerate()
            .map(|(index, child)| (format!("item[{index}]"), value_to_iovalue(&child)))
            .take(child_budget)
            .collect()
    } else if value.is_dictionary() {
        value
            .entries()
            .enumerate()
            .flat_map(|(index, (key, child))| {
                [
                    (format!("entry[{index}].key"), value_to_iovalue(&key)),
                    (format!("entry[{index}].value"), value_to_iovalue(&child)),
                ]
            })
            .take(child_budget)
            .collect()
    } else {
        Vec::new()
    }
}

fn structural_match(kind: StructuralTokenKind, token: &str, path: &[String]) -> StructuralMatch {
    StructuralMatch {
        kind,
        token: token.to_string(),
        path: path.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/tests/m000/p002/body.rs"));
}
