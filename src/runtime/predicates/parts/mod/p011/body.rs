// The bounded record pattern form.
//
// A record pattern declares a record label and a set of declared field
// positions. The candidate is matched with the at-least rule: undeclared
// positions are ignored, a missing or mistyped declared position fails the
// match, and the bindings are produced in declared position order. A failed
// sub-pattern discards the bindings the visit produced so far.
//
// The form is admitted for routing only. It never computes an identity, a
// receipt hash, or a canonical value ref.

const RUNTIME_PATTERN_RECORD_LABEL: &str = "runtime-pattern-record-v1";
const PATTERN_RECORD_ARITY: usize = 2;
const PATTERN_RECORD_LABEL_INDEX: usize = 0;
const PATTERN_RECORD_ENTRIES_INDEX: usize = 1;
const PATTERN_RECORD_ENTRY_ARITY: usize = 2;
const PATTERN_RECORD_ENTRY_POSITION_INDEX: usize = 0;
const PATTERN_RECORD_ENTRY_PATTERN_INDEX: usize = 1;
const MAX_PATTERN_RECORD_ENTRIES: usize = 16;
const MAX_PATTERN_RECORD_DEPTH: usize = 4;

/// One declared field position of a record pattern.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecordPatternEntry {
    /// The declared field position of the candidate record.
    pub position: usize,
    /// The sub-pattern the declared position must match.
    pub pattern: Box<RuntimePattern>,
}

/// A record pattern read from its AST form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPattern {
    pub label: String,
    pub entries: Vec<RecordPatternEntry>,
}

/// Read the record pattern AST form.
pub fn record_pattern_from_ast(value: &IoValue) -> Result<Option<RecordPattern>> {
    let Some(fields) = value.collect_simple_record(RUNTIME_PATTERN_RECORD_LABEL, Some(PATTERN_RECORD_ARITY)) else {
        return Ok(None);
    };
    let label = crate::preserves_rail::required_string_field(&fields[PATTERN_RECORD_LABEL_INDEX], "record pattern label")?;
    if label.is_empty() {
        return Err(crate::error::Failure::invalid_harness(
            "runtime record pattern label must not be empty",
        ));
    }
    let entry_values = crate::preserves_rail::required_sequence_field(
        &fields[PATTERN_RECORD_ENTRIES_INDEX],
        "record pattern entries",
    )?;
    let mut entries = Vec::with_capacity(entry_values.len());
    for entry_value in entry_values.iter() {
        let entry_fields = entry_value
            .collect_simple_record("entry", Some(PATTERN_RECORD_ENTRY_ARITY))
            .ok_or_else(|| {
                crate::error::Failure::invalid_harness("runtime record pattern entry is not a declared entry")
            })?;
        let position = entry_fields[PATTERN_RECORD_ENTRY_POSITION_INDEX]
            .as_u64()
            .ok_or_else(|| {
                crate::error::Failure::invalid_harness("runtime record pattern entry position must be an unsigned integer")
            })?
            .map_err(|error| {
                crate::error::Failure::invalid_harness(format!(
                    "runtime record pattern entry position is out of range: {error}"
                ))
            })?;
        let position = usize::try_from(position).map_err(|_| {
            crate::error::Failure::invalid_harness("runtime record pattern entry position exceeds the platform bound")
        })?;
        let sub_pattern = RuntimePattern::from_ast_value(&crate::preserves_rail::value_to_iovalue(
            &entry_fields[PATTERN_RECORD_ENTRY_PATTERN_INDEX],
        ))?;
        entries.push(RecordPatternEntry {
            position,
            pattern: Box::new(sub_pattern),
        });
    }
    Ok(Some(RecordPattern { label, entries }))
}

/// The AST value of a record pattern.
pub fn record_pattern_to_value(label: &str, entries: &[RecordPatternEntry]) -> IoValue {
    let entry_values = entries
        .iter()
        .map(|entry| {
            crate::preserves_rail::record(
                "entry",
                vec![
                    crate::preserves_rail::u64_value(entry.position as u64),
                    entry.pattern.to_value(),
                ],
            )
        })
        .collect();
    crate::preserves_rail::record(
        RUNTIME_PATTERN_RECORD_LABEL,
        vec![crate::preserves_rail::string(label), crate::preserves_rail::sequence(entry_values)],
    )
}

/// Validate the admitted record pattern form and its bounds.
pub fn record_pattern_validate(label: &str, entries: &[RecordPatternEntry]) -> Result<()> {
    if label.is_empty() {
        return Err(crate::error::Failure::invalid_harness(
            "runtime record pattern label must not be empty",
        ));
    }
    if entries.is_empty() {
        return Err(crate::error::Failure::invalid_harness(
            "runtime record pattern must declare at least one position",
        ));
    }
    if entries.len() > MAX_PATTERN_RECORD_ENTRIES {
        return Err(crate::error::Failure::invalid_harness(format!(
            "runtime record pattern declares more than {MAX_PATTERN_RECORD_ENTRIES} positions"
        )));
    }
    let mut previous: Option<usize> = None;
    for entry in entries {
        if previous.is_some_and(|previous| entry.position <= previous) {
            return Err(crate::error::Failure::invalid_harness(
                "runtime record pattern positions must increase in Preserves order",
            ));
        }
        previous = Some(entry.position);
        entry.pattern.validate()?;
    }
    Ok(())
}

/// The nesting depth of a pattern, counting the record forms.
fn record_pattern_depth(pattern: &RuntimePattern) -> usize {
    match pattern {
        RuntimePattern::Exact(_) | RuntimePattern::Wildcard { .. } => 0,
        RuntimePattern::Record { entries, .. } => {
            1 + entries
                .iter()
                .map(|entry| record_pattern_depth(&entry.pattern))
                .max()
                .unwrap_or(0)
        }
    }
}

/// Match a candidate against a record pattern.
///
/// The candidate must be a record with the declared label. Each declared
/// position must be present and must match its sub-pattern. Positions the
/// pattern does not declare are ignored.
pub fn record_pattern_matches(
    label: &str,
    entries: &[RecordPatternEntry],
    value: &RuntimeValue,
) -> Result<(bool, Vec<(String, String)>)> {
    let Some(fields) = value.as_iovalue().collect_simple_record(label, None) else {
        return Ok((false, Vec::new()));
    };
    let mut bindings: Vec<(String, String)> = Vec::new();
    for entry in entries {
        let Some(field) = fields.fields_iter().nth(entry.position) else {
            return Ok((false, Vec::new()));
        };
        let field_value = RuntimeValue::new(crate::preserves_rail::value_to_iovalue(field))?;
        let (matched, mut entry_bindings) = entry.pattern.matches_value(&field_value)?;
        if !matched {
            return Ok((false, Vec::new()));
        }
        bindings.append(&mut entry_bindings);
    }
    Ok((true, bindings))
}
