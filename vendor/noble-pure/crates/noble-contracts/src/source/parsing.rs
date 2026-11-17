#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; parsing mutates only its fresh scanner, syntax arena/quotation frames, and preparation-owned meter under byte/node/depth/work bounds; declaration extraction moves an owned quotation body, while borrowed source bytes and the committed namespace remain unchanged."
)]

struct Frame {
    body: alloc::vec::Vec<u32>,
    start: u32,
}

struct State {
    frames: alloc::vec::Vec<Frame>,
    nodes: alloc::vec::Vec<super::Node>,
}

pub(super) fn parse(
    source_bytes: &[u8],
    meter: &mut crate::Meter,
) -> Result<(super::Tree, Option<alloc::string::String>), crate::Diagnostic> {
    parse_impl(source_bytes, meter, false)
}

pub(super) fn parse_declared(
    source_bytes: &[u8],
    meter: &mut crate::Meter,
) -> Result<(super::Tree, Option<alloc::string::String>), crate::Diagnostic> {
    parse_impl(source_bytes, meter, true)
}

fn parse_impl(
    source_bytes: &[u8],
    meter: &mut crate::Meter,
    declared: bool,
) -> Result<(super::Tree, Option<alloc::string::String>), crate::Diagnostic> {
    let mut lexer = if declared {
        attempt!(super::lexer::Scanner::new_declared(source_bytes, meter))
    } else {
        attempt!(super::lexer::Scanner::new(source_bytes, meter))
    };
    let span = crate::Span {
        start: 0,
        end: attempt!(crate::index(
            source_bytes.len(),
            crate::Span { start: 0, end: 0 }
        )),
    };
    let (name, first) = attempt!(opening(&mut lexer, span, meter));
    scan_tokens(first, &mut lexer, span, meter, name)
}

fn scan_tokens(
    first: Option<super::lexer::Token>,
    lexer: &mut super::lexer::Scanner<'_>,
    span: crate::Span,
    meter: &mut crate::Meter,
    name: Option<alloc::string::String>,
) -> Result<(super::Tree, Option<alloc::string::String>), crate::Diagnostic> {
    attempt!(meter.node(span));
    let mut state = State {
        frames: alloc::vec::Vec::with_capacity(1),
        nodes: alloc::vec::Vec::new(),
    };
    state.frames.push(Frame {
        body: alloc::vec::Vec::new(),
        start: 0,
    });
    let mut next = Ok(first);
    let mut failure = None;
    while let Some(token) = match next {
        Ok(token) => token,
        Err(problem) => {
            failure = Some(problem);
            None
        }
    } {
        next = advance(token, &mut state, lexer, meter);
    }
    if let Some(problem) = failure {
        return Err(problem);
    }
    if state.frames.len() != 1 {
        return Err(crate::invalid(span, "unclosed quotation"));
    }
    let frame = match state.frames.pop() {
        Some(frame) => frame,
        None => return Err(crate::internal(span)),
    };
    // The initial root frame is never closed or replaced by quotation steps.
    debug_assert_eq!(frame.start, 0);
    let body = if name.is_some() {
        attempt!(declaration_body(frame, &mut state.nodes, span))
    } else {
        frame.body
    };
    debug_assert!(body.iter().all(|index| usize::try_from(*index)
        .ok()
        .is_some_and(|node| node < state.nodes.len())));
    Ok((
        super::Tree {
            nodes: state.nodes,
            body,
            span,
        },
        name,
    ))
}

fn advance(
    token: super::lexer::Token,
    state: &mut State,
    lexer: &mut super::lexer::Scanner<'_>,
    meter: &mut crate::Meter,
) -> Result<Option<super::lexer::Token>, crate::Diagnostic> {
    attempt!(step(token, state, meter));
    lexer.next(meter)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; declaration opening requires an actual word name followed by one quotation opener, with checked UTF-8 conversion and metered token reads rather than assertions."
)]
fn opening(
    lexer: &mut super::lexer::Scanner<'_>,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(Option<alloc::string::String>, Option<super::lexer::Token>), crate::Diagnostic> {
    let first = attempt!(lexer.next(meter));
    match first {
        Some(super::lexer::Token {
            kind: super::lexer::TokenKind::Def,
            ..
        }) => {}
        first => return Ok((None, first)),
    }
    let word = match attempt!(lexer.next(meter)) {
        Some(super::lexer::Token {
            kind: super::lexer::TokenKind::Word(word),
            ..
        }) => word,
        _ => {
            return Err(crate::invalid(
                span,
                "definition requires a nonreserved word name",
            ))
        }
    };
    let name = match alloc::string::String::from_utf8(word) {
        Ok(name) => name,
        Err(_) => return Err(crate::internal(span)),
    };
    let first = match attempt!(lexer.next(meter)) {
        Some(
            token @ super::lexer::Token {
                kind: super::lexer::TokenKind::Open,
                ..
            },
        ) => Some(token),
        _ => {
            return Err(crate::invalid(
                span,
                "definition body must be one quotation",
            ))
        }
    };
    Ok((Some(name), first))
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; declaration extraction validates arity and the arena node before moving the owned quotation body; malformed producer state returns a diagnostic rather than asserting over it."
)]
fn declaration_body(
    frame: Frame,
    nodes: &mut [super::Node],
    span: crate::Span,
) -> Result<alloc::vec::Vec<u32>, crate::Diagnostic> {
    if frame.body.len() != 1 {
        return Err(crate::invalid(
            span,
            "definition submission contains trailing expressions",
        ));
    }
    let id = match frame.body.first() {
        Some(id) => *id,
        None => return Err(crate::internal(span)),
    };
    match nodes.get_mut(attempt!(crate::offset(id, span))) {
        Some(super::Node {
            kind: super::Kind::Quotation(body),
            ..
        }) => Ok(core::mem::take(body)),
        Some(super::Node {
            kind:
                super::Kind::Literal(_)
                | super::Kind::Text(_)
                | super::Kind::Word(_)
                | super::Kind::Call(_)
                | super::Kind::EditorHole,
            ..
        })
        | None => Err(crate::internal(span)),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; token transitions charge work, check nesting limits and frame presence, reject misplaced declarations or unmatched closers, and append nodes through checked metered IDs."
)]
fn step(
    token: super::lexer::Token,
    state: &mut State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, token.span));
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; every closed token kind must define a parser transition, and newly added syntax must fail compilation until handled explicitly."
    )]
    let kind = match token.kind {
        super::lexer::TokenKind::Open => {
            attempt!(meter.depth(
                attempt!(crate::index(state.frames.len(), token.span)),
                token.span
            ));
            attempt!(meter.node(token.span));
            state.frames.push(Frame {
                body: alloc::vec::Vec::new(),
                start: token.span.start,
            });
            return Ok(());
        }
        super::lexer::TokenKind::Close => {
            if state.frames.len() <= 1 {
                return Err(crate::invalid(
                    token.span,
                    "unmatched quotation closing bracket",
                ));
            }
            let frame = match state.frames.pop() {
                Some(frame) => frame,
                None => return Err(crate::internal(token.span)),
            };
            return append(
                super::Node {
                    kind: super::Kind::Quotation(frame.body),
                    span: crate::Span {
                        start: frame.start,
                        end: token.span.end,
                    },
                },
                state,
                meter,
            );
        }
        super::lexer::TokenKind::Def => {
            return Err(crate::invalid(
                token.span,
                "def is a declaration, not an expression",
            ));
        }
        super::lexer::TokenKind::Literal(lit) => super::Kind::Literal(lit),
        super::lexer::TokenKind::Text(bytes) => super::Kind::Text(bytes),
        super::lexer::TokenKind::Word(word) => super::Kind::Word(word),
        super::lexer::TokenKind::ProofColon | super::lexer::TokenKind::LogicalBody(_) => {
            return Err(crate::invalid(
                token.span,
                "logical token outside a proof declaration",
            ));
        }
    };
    append(
        super::Node {
            kind,
            span: token.span,
        },
        state,
        meter,
    )
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; appending a metered syntax node grows owned vectors after checked indexing and frame lookup, whose failures allocate diagnostics."
)]
fn append(
    node: super::Node,
    state: &mut State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.node(node.span));
    let id = attempt!(crate::index(state.nodes.len(), node.span));
    let frame = match state.frames.last_mut() {
        Some(frame) => frame,
        None => return Err(crate::internal(node.span)),
    };
    frame.body.push(id);
    state.nodes.push(node);
    Ok(())
}
