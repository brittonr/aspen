//! Bounded scanning and capture of a bracketed module definition body.

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
pub(super) fn parse<'a>(
    input_bytes: &[u8],
    limits: crate::Limits,
    cursor: super::Cursor<'a>,
    token: &crate::source::lexer::Token,
) -> Result<(super::Cursor<'a>, super::Member), crate::Diagnostic> {
    let span = cursor.span;
    let start = attempt!(crate::offset(token.span.start, span));
    let (cursor, name) = attempt!(cursor.ident());
    let (cursor, opening) = attempt!(open_definition(cursor));
    let (cursor, end) = attempt!(definition_end(cursor, limits));
    let end_span = attempt!(crate::index(end, span));
    Ok((
        cursor,
        super::Member::Definition {
            name,
            bytes: input_bytes[start..end].to_vec(),
            span: crate::Span {
                start: token.span.start,
                end: end_span,
            },
            body_span: crate::Span {
                start: opening.start,
                end: end_span,
            },
        },
    ))
}

fn open_definition(
    cursor: super::Cursor<'_>,
) -> Result<(super::Cursor<'_>, crate::Span), crate::Diagnostic> {
    let Some(token) = cursor.tokens.get(cursor.at) else {
        return Err(super::parse_diagnostic(
            cursor.span,
            "definition body must be bracketed",
        ));
    };
    if !matches!(&token.kind, crate::source::lexer::TokenKind::Open) {
        return Err(super::parse_diagnostic(
            cursor.span,
            "definition body must be bracketed",
        ));
    }
    Ok((cursor, token.span))
}
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the owned quotation cursor preserves the first definition_step error or checked closing offset under depth limits; reassess fallible-check density."
)]
fn definition_end<'a>(
    cursor: super::Cursor<'a>,
    limits: crate::Limits,
) -> Result<(super::Cursor<'a>, usize), crate::Diagnostic> {
    let mut cursor = cursor;
    let mut depth = 0u32;
    let mut end = None;
    let mut problem = None;
    while end.is_none() && problem.is_none() {
        let next = super::Cursor {
            tokens: cursor.tokens,
            at: cursor.at,
            span: cursor.span,
        };
        match definition_step(next, depth, limits) {
            Ok((next, next_depth, completed)) => {
                cursor = next;
                depth = next_depth;
                end = completed;
            }
            Err(error) => problem = Some(error),
        }
    }
    match problem {
        Some(error) => Err(error),
        None => match end {
            Some(end) => Ok((cursor, end)),
            None => Err(crate::internal(cursor.span)),
        },
    }
}
#[expect(
    tigerstyle::missing_const_fn,
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; slice.get, diagnostics, checked offset and ok_or_else are non-const on pinned Rust; missing token, nesting, depth and offset reject in typed order; reassess const APIs and fallible-check density."
)]
fn definition_step<'a>(
    mut cursor: super::Cursor<'a>,
    depth: u32,
    limits: crate::Limits,
) -> Result<(super::Cursor<'a>, u32, Option<usize>), crate::Diagnostic> {
    let Some(token) = cursor.tokens.get(cursor.at) else {
        return Err(super::parse_diagnostic(
            cursor.span,
            "unclosed module definition",
        ));
    };
    cursor.at += 1;
    let (depth, is_closed) = attempt!(super::super::quotation_step(depth, token)
        .ok_or_else(|| super::parse_diagnostic(cursor.span, "invalid module quotation nesting")));
    if depth > limits.depth {
        // A limit, not a malformed body: exhaustion never proves invalidity.
        return Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Exhausted,
            cursor.span,
            "module quotation depth exceeded",
        ));
    }
    let end = if is_closed {
        Some(attempt!(crate::offset(token.span.end, cursor.span)))
    } else {
        None
    };
    Ok((cursor, depth, end))
}
