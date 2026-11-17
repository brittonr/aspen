pub(super) struct Parts {
    pub input: Option<u32>,
    pub output: Option<u32>,
    pub params: Option<u32>,
    pub program: Option<u32>,
    pub requires: Option<u32>,
    pub ensures: Option<u32>,
    pub kind: Option<u32>,
    pub definitions: alloc::vec::Vec<u32>,
    pub failure: Option<crate::Diagnostic>,
}

#[derive(Clone, Copy)]
#[octet::sealed_enum]
enum Field {
    Input,
    Output,
    Program,
    Params,
    Requires,
    Ensures,
    Kind,
    Definition,
    Unsupported,
    Unknown,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; field traversal preserves the first typed parse failure and never asserts over hostile input."
)]
pub(super) fn collect(
    container: super::Container<'_>,
    meter: &mut crate::Meter,
) -> Result<Parts, crate::Diagnostic> {
    let mut parsed = Ok(Parts {
        input: None,
        output: None,
        params: None,
        program: None,
        requires: None,
        ensures: None,
        kind: None,
        definitions: alloc::vec::Vec::new(),
        failure: None,
    });
    let mut at = 3usize;
    while at < container.children.len() {
        parsed = match parsed {
            Ok(parts) => part(container, at, parts, meter),
            Err(error) => Err(error),
        };
        if parsed.is_err() {
            break;
        }
        at += 1;
    }
    parsed
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; lookup, arity, and duplicate-field errors are fallible; logical failures deliberately retain ordinary acceptance."
)]
fn part(
    container: super::Container<'_>,
    at: usize,
    parts: Parts,
    meter: &mut crate::Meter,
) -> Result<Parts, crate::Diagnostic> {
    attempt!(meter.charge(1, container.span));
    let id = attempt!(crate::syntax::child(container.children, at, container.span));
    let at_span = attempt!(container.tree.node(id)).span;
    let items = attempt!(container.tree.round(id));
    let head = attempt!(container.tree.atom(
        container.source,
        attempt!(crate::syntax::child(items, 0, at_span))
    ));
    let field = classify(head);
    let (mut parts, result) = record(parts, field, id, at_span);
    if let Err(error) = result {
        // Duplicated ordinary interface/program fields are ambiguous;
        // logical-only failures may still retain ordinary acceptance.
        if matches!(field, Field::Input | Field::Output | Field::Program) {
            return Err(error);
        }
        if parts.failure.is_none() {
            parts.failure = Some(error);
        }
    }
    Ok(parts)
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; every Field must be reviewed when the grammar grows. Recording definitions allocates and duplicate or unsupported fields return diagnostics, not assertions."
)]
fn record(
    mut parts: Parts,
    field: Field,
    id: u32,
    span: crate::Span,
) -> (Parts, Result<(), crate::Diagnostic>) {
    let result = match field {
        Field::Input => {
            let (slot, result) = unique(parts.input, id, span);
            parts.input = slot;
            result
        }
        Field::Output => {
            let (slot, result) = unique(parts.output, id, span);
            parts.output = slot;
            result
        }
        Field::Program => {
            let (slot, result) = unique(parts.program, id, span);
            parts.program = slot;
            result
        }
        Field::Params => {
            let (slot, result) = unique(parts.params, id, span);
            parts.params = slot;
            result
        }
        Field::Requires => {
            let (slot, result) = unique(parts.requires, id, span);
            parts.requires = slot;
            result
        }
        Field::Ensures => {
            let (slot, result) = unique(parts.ensures, id, span);
            parts.ensures = slot;
            result
        }
        Field::Kind => {
            let (slot, result) = unique(parts.kind, id, span);
            parts.kind = slot;
            result
        }
        Field::Definition => { parts.definitions.push(id); Ok(()) }
        Field::Unsupported => Err(crate::Diagnostic::new(crate::DiagnosticKind::Unsupported, span, "this property or producer-supplied assumption is not supported by the fixed partial-correctness profile")),
        Field::Unknown => Err(crate::invalid(span, "unknown contract field")),
    };
    (parts, result)
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    tigerstyle::compound_condition,
    reason = "Owner: noble-maintainers; this total byte-token classifier intentionally groups the unsupported spellings. Slice equality is non-const and arbitrary input needs no assertion."
)]
fn classify(head: &[u8]) -> Field {
    if head == b"input" {
        return Field::Input;
    }
    if head == b"output" {
        return Field::Output;
    }
    if head == b"program" {
        return Field::Program;
    }
    if head == b"params" {
        return Field::Params;
    }
    if head == b"requires" {
        return Field::Requires;
    }
    if head == b"ensures" {
        return Field::Ensures;
    }
    if head == b"kind" {
        return Field::Kind;
    }
    if head == b"define" {
        return Field::Definition;
    }
    if head == b"invariant"
        || head == b"invariants"
        || head == b"effect-trace"
        || head == b"ownership"
        || head == b"effects"
        || head == b"assumptions"
    {
        return Field::Unsupported;
    }
    Field::Unknown
}

fn unique(
    slot: Option<u32>,
    id: u32,
    span: crate::Span,
) -> (Option<u32>, Result<(), crate::Diagnostic>) {
    if slot.is_some() {
        (
            slot,
            Err(crate::invalid(span, "contract field is repeated")),
        )
    } else {
        (Some(id), Ok(()))
    }
}
