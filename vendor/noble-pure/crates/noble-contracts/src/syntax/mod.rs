#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; syntax preparation mutates only fresh parser frames/nodes, type work/value stacks, checked I64 accumulators, and preparation-owned work accounting; source-byte/node/depth limits, the 256-constructor cap, and charged parsed-child scheduling bound this scratch; borrowed source and completed syntax trees remain unchanged."
)]

mod constructors;
mod identifiers;
mod integer;
mod parsing;
mod typing;

pub(crate) enum Form {
    Atom,
    Round(alloc::vec::Vec<u32>),
    Square(alloc::vec::Vec<u32>),
}

pub(crate) struct Node {
    pub form: Form,
    pub span: crate::Span,
}

pub(crate) struct Tree {
    pub nodes: alloc::vec::Vec<Node>,
    pub root: u32,
}

impl Tree {
    pub fn node(&self, id: u32) -> Result<&Node, crate::Diagnostic> {
        let span = crate::Span { start: 0, end: 0 };
        match self.nodes.get(attempt!(crate::offset(id, span))) {
            Some(node) => Ok(node),
            None => Err(crate::internal(span)),
        }
    }

    pub fn round(&self, id: u32) -> Result<&[u32], crate::Diagnostic> {
        let node = attempt!(self.node(id));
        match &node.form {
            Form::Round(children) => Ok(children),
            _ => Err(crate::invalid(node.span, "expected a parenthesized form")),
        }
    }

    pub fn square(&self, id: u32) -> Result<&[u32], crate::Diagnostic> {
        let node = attempt!(self.node(id));
        match &node.form {
            Form::Square(children) => Ok(children),
            _ => Err(crate::invalid(
                node.span,
                "expected a bracketed program body",
            )),
        }
    }

    pub fn atom<'a>(&self, source: &'a [u8], id: u32) -> Result<&'a [u8], crate::Diagnostic> {
        let node = attempt!(self.node(id));
        if !matches!(node.form, Form::Atom) {
            return Err(crate::invalid(node.span, "expected an atom"));
        }
        let start = attempt!(crate::offset(node.span.start, node.span));
        let end = attempt!(crate::offset(node.span.end, node.span));
        match source.get(start..end) {
            Some(bytes) => Ok(bytes),
            None => Err(crate::internal(node.span)),
        }
    }
}

pub(crate) fn child(
    children: &[u32],
    at: usize,
    span: crate::Span,
) -> Result<u32, crate::Diagnostic> {
    match children.get(at) {
        Some(id) => Ok(*id),
        None => Err(crate::invalid(span, "missing form argument")),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; byte, encoding, syntax, and work limits are fallible validation of hostile source, not panic preconditions."
)]
pub(crate) fn parse(source: &[u8], meter: &mut crate::Meter) -> Result<Tree, crate::Diagnostic> {
    let full = crate::Span {
        start: 0,
        end: attempt!(crate::index(source.len(), crate::Span { start: 0, end: 0 })),
    };
    if full.end > meter.limits.bytes {
        return Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Exhausted,
            full,
            "source byte limit exceeded",
        ));
    }
    attempt!(source_encoding(source, full, meter));
    let mut state = parsing::State::new();
    let mut failure = None;
    while state.position < source.len() {
        if let Err(error) = parsing::step(source, full, &mut state, meter) {
            failure = Some(error);
            break;
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    parsing::finish(state, full)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; encoding validation charges runtime work and builds an owned diagnostic on invalid UTF-8."
)]
fn source_encoding(
    source: &[u8],
    full: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    // Validate the bounded whole source before the parser can skip comments.
    attempt!(meter.charge(full.end, full));
    match core::str::from_utf8(source) {
        Ok(_) => Ok(()),
        Err(error) => {
            let start = attempt!(crate::index(error.valid_up_to(), full));
            let end = match error.error_len() {
                Some(length) => start.saturating_add(attempt!(crate::index(length, full))),
                None => full.end,
            };
            Err(crate::invalid(
                crate::Span { start, end },
                "invalid UTF-8 source encoding",
            ))
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; empty names, each identifier byte, and UTF-8 decoding have explicit diagnostic paths."
)]
pub(crate) fn identifier(
    source: &[u8],
    tree: &Tree,
    id: u32,
    meter: &mut crate::Meter,
) -> Result<alloc::string::String, crate::Diagnostic> {
    let span = attempt!(tree.node(id)).span;
    let bytes = attempt!(tree.atom(source, id));
    if bytes.is_empty() {
        return Err(crate::invalid(span, "empty identifier"));
    }
    let mut at = 0usize;
    let mut failure = None;
    while at < bytes.len() {
        if let Err(error) = identifiers::byte(bytes, at, span, meter) {
            failure = Some(error);
            break;
        }
        at += 1;
    }
    if let Some(error) = failure {
        return Err(error);
    }
    match core::str::from_utf8(bytes) {
        Ok(text) => Ok(alloc::string::String::from(text)),
        Err(_) => Err(crate::invalid(span, "invalid identifier encoding")),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; decimal classification and every checked digit transition distinguish non-literals from malformed or out-of-range I64 values."
)]
pub(crate) fn integer(
    bytes: &[u8],
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<Option<i64>, crate::Diagnostic> {
    let first = match bytes.first() {
        Some(first) => *first,
        None => return Ok(None),
    };
    let is_negative = first == b'-';
    let mut at = 0usize;
    if is_negative {
        at = 1;
    }
    if at >= bytes.len() {
        return Ok(None);
    }
    let first = match bytes.get(at) {
        Some(first) => *first,
        None => return Err(crate::internal(span)),
    };
    if !first.is_ascii_digit() {
        return Ok(None);
    }
    let mut value = integer::Accumulator::new(is_negative);
    let mut failure = None;
    while at < bytes.len() {
        if let Err(error) = value.digit(bytes, at, span, meter) {
            failure = Some(error);
            break;
        }
        at += 1;
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(Some(value.value)),
    }
}

pub(crate) const TYPE_CAP: u32 = 256;

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; type worklist failures and the single-result invariant are explicit diagnostics, preserving the fallible frontend contract."
)]
pub(crate) fn ty(
    source: &[u8],
    tree: &Tree,
    root: u32,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::types::Ty, crate::Diagnostic> {
    let span = attempt!(tree.node(root)).span;
    let context = typing::Context { source, tree };
    let mut outcome = Ok(typing::State::new(root));
    while let Ok(mut state) = outcome {
        match state.steps.pop() {
            Some(step) => outcome = typing::step(context, step, span, state, meter),
            None => {
                outcome = Ok(state);
                break;
            }
        }
    }
    let mut state = attempt!(outcome);
    if state.values.len() != 1 {
        return Err(crate::internal(span));
    }
    match state.values.pop() {
        Some(value) => Ok(value),
        None => Err(crate::internal(span)),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the parsed list bounds traversal and every child type is checked fallibly with metered work."
)]
pub(crate) fn types(
    source: &[u8],
    tree: &Tree,
    root: u32,
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<noble_kernel::types::Ty>, crate::Diagnostic> {
    let span = attempt!(tree.node(root)).span;
    let children = attempt!(tree.round(root));
    let context = typing::Context { source, tree };
    // The parsed list fixes the maximum number of result types.
    let mut values = alloc::vec::Vec::with_capacity(children.len());
    let mut at = 0usize;
    let mut failure = None;
    while at < children.len() {
        match typing::item(context, children, at, span, meter) {
            Ok(ty) => values.push(ty),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
        at += 1;
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(values),
    }
}
