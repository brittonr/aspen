pub(super) fn parse(
    input_bytes: &[u8],
    limits: crate::Limits,
    cursor: super::Cursor<'_>,
    name: alloc::string::String,
    version: u32,
) -> Result<super::ParsedUnit, crate::Diagnostic> {
    let span = cursor.span;
    let cursor = attempt!(open_body(cursor));
    let (cursor, members) = attempt!(collect_body(input_bytes, limits, cursor));
    if cursor.at != cursor.tokens.len() {
        return Err(super::parse_diagnostic(
            span,
            "module contains trailing expressions",
        ));
    }
    Ok(super::ParsedUnit::Module {
        name,
        version,
        members,
    })
}

fn open_body(mut cursor: super::Cursor<'_>) -> Result<super::Cursor<'_>, crate::Diagnostic> {
    if !super::open_at(cursor.tokens, cursor.at) {
        return Err(super::parse_diagnostic(
            cursor.span,
            "module body must be one bracketed declaration block",
        ));
    }
    cursor.at += 1;
    Ok(cursor)
}
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; bounded member collection stops at Close or the first parse_member Diagnostic and reports unclosed bodies without panicking; reassess fallible-check density."
)]
fn collect_body<'a>(
    input_bytes: &[u8],
    limits: crate::Limits,
    cursor: super::Cursor<'a>,
) -> Result<(super::Cursor<'a>, alloc::vec::Vec<super::Member>), crate::Diagnostic> {
    let span = cursor.span;
    let max_members = attempt!(usize::try_from(limits.nodes)
        .map_err(|_| super::parse_diagnostic(span, "member limit exceeds the host address space")));
    let member_count = max_members.min(cursor.tokens.len());
    let mut members = alloc::vec::Vec::with_capacity(member_count);
    let tokens = cursor.tokens;
    let mut cursor = cursor;
    let mut is_closed = false;
    let mut problem = None;
    while cursor.at < tokens.len() && !is_closed && problem.is_none() {
        let token = &tokens[cursor.at];
        if let crate::source::lexer::TokenKind::Close = &token.kind {
            cursor.at += 1;
            is_closed = true;
        } else {
            let next = super::Cursor {
                tokens,
                at: cursor.at,
                span: cursor.span,
            };
            match parse_member(input_bytes, limits, next, token) {
                Ok((next, member)) => {
                    cursor = next;
                    members.push(member);
                }
                Err(error) => problem = Some(error),
            }
        }
    }
    match problem {
        Some(problem) => Err(problem),
        None if is_closed => Ok((cursor, members)),
        None => Err(super::parse_diagnostic(span, "unclosed module body")),
    }
}

fn parse_member<'a>(
    input_bytes: &[u8],
    limits: crate::Limits,
    mut cursor: super::Cursor<'a>,
    token: &crate::source::lexer::Token,
) -> Result<(super::Cursor<'a>, super::Member), crate::Diagnostic> {
    cursor.at += 1;
    if let crate::source::lexer::TokenKind::Word(bytes) = &token.kind {
        return parse_word_member(cursor, bytes, token.span);
    }
    if let crate::source::lexer::TokenKind::Def = &token.kind {
        return super::body::parse(input_bytes, limits, cursor, token);
    }
    Err(super::parse_diagnostic(
        token.span,
        "unknown module declaration or initializer",
    ))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn parse_word_member<'a>(
    cursor: super::Cursor<'a>,
    spelling: &[u8],
    span: crate::Span,
) -> Result<(super::Cursor<'a>, super::Member), crate::Diagnostic> {
    if spelling.eq(b"opaque") {
        return opaque(cursor);
    }
    if spelling.eq(b"variant") {
        return variant(cursor);
    }
    if spelling.eq(b"require") {
        return require(cursor);
    }
    if spelling.eq(b"signature") {
        return signature(cursor);
    }
    if spelling.eq(b"export") {
        return export(cursor);
    }
    if spelling.eq(b"contract") {
        return contract(cursor, span);
    }
    if spelling.eq(b"proof") {
        return proof(cursor, span);
    }
    Err(super::parse_diagnostic(
        span,
        "unknown module declaration or initializer",
    ))
}

fn revision(mut cursor: super::Cursor<'_>) -> Result<(super::Cursor<'_>, u32), crate::Diagnostic> {
    let Some(token) = cursor.tokens.get(cursor.at) else {
        return Err(super::parse_diagnostic(
            cursor.span,
            "missing logical declaration revision",
        ));
    };
    let version = attempt!(super::logical_revision(token).ok_or_else(|| {
        super::parse_diagnostic(token.span, "expected logical declaration revision 1 or 2")
    }));
    cursor.at += 1;
    Ok((cursor, version))
}

fn logical(
    mut cursor: super::Cursor<'_>,
) -> Result<(super::Cursor<'_>, crate::intrinsic::Form), crate::Diagnostic> {
    let Some(token) = cursor.tokens.get(cursor.at) else {
        return Err(super::parse_diagnostic(
            cursor.span,
            "missing bracketed logical form",
        ));
    };
    let crate::source::lexer::TokenKind::LogicalBody(tokens) = &token.kind else {
        return Err(super::parse_diagnostic(
            token.span,
            "expected bracketed logical form",
        ));
    };
    let form = attempt!(super::logic::parse(tokens, token.span));
    cursor.at += 1;
    Ok((cursor, form))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn binding_block(
    mut cursor: super::Cursor<'_>,
) -> Result<(super::Cursor<'_>, alloc::vec::Vec<super::Binding>), crate::Diagnostic> {
    if !super::open_at(cursor.tokens, cursor.at) {
        return Err(super::parse_diagnostic(
            cursor.span,
            "expected bracketed stack bindings",
        ));
    }
    cursor.at += 1;
    let mut bindings = alloc::vec::Vec::new();
    loop {
        let Some(token) = cursor.tokens.get(cursor.at) else {
            return Err(super::parse_diagnostic(
                cursor.span,
                "unclosed stack bindings",
            ));
        };
        if matches!(&token.kind, crate::source::lexer::TokenKind::Close) {
            cursor.at += 1;
            return Ok((cursor, bindings));
        }
        let span = token.span;
        let (next, name) = attempt!(cursor.ident());
        let (next, ty) = attempt!(next.next());
        if bindings
            .iter()
            .any(|known: &super::Binding| known.name == name)
        {
            return Err(super::parse_diagnostic(span, "duplicate stack binding"));
        }
        // Each binding consumes two tokens, so this bound is never reached
        // before the duplicate or unclosed-block diagnostics.
        if bindings.len() >= next.tokens.len() {
            return Err(super::parse_diagnostic(
                span,
                "stack bindings exceed their token block",
            ));
        }
        bindings.push(super::Binding {
            name,
            ty: alloc::string::String::from(ty),
            span,
        });
        cursor = next;
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn reference(text: &str, logical_target: bool) -> bool {
    let target_name = |name| {
        if logical_target {
            super::logical_name(name)
        } else {
            super::name(name)
        }
    };
    let Some((scope, local)) = text.split_once('.') else {
        return target_name(text);
    };
    if !target_name(local) || local.contains('.') {
        return false;
    }
    if let Some((module, version)) = scope.split_once('@') {
        super::name(module)
            && !version.is_empty()
            && version.as_bytes()[0] != b'0'
            && version.bytes().all(|byte| byte.is_ascii_digit())
            && version.parse::<u32>().is_ok()
    } else {
        super::name(scope)
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn contract(
    cursor: super::Cursor<'_>,
    span: crate::Span,
) -> Result<(super::Cursor<'_>, super::Member), crate::Diagnostic> {
    let (cursor, revision) = attempt!(revision(cursor));
    let (mut cursor, name) = attempt!(cursor.logical_ident());
    if !super::open_at(cursor.tokens, cursor.at) {
        return Err(super::parse_diagnostic(
            span,
            "contract requires one bracketed declaration",
        ));
    }
    cursor.at += 1;
    let cursor = attempt!(cursor.expected("subject"));
    let Some(subject_span) = cursor.tokens.get(cursor.at).map(|token| token.span) else {
        return Err(super::parse_diagnostic(span, "missing contract subject"));
    };
    let (cursor, subject) = attempt!(cursor.next());
    if !reference(subject, false) {
        return Err(super::parse_diagnostic(
            subject_span,
            "invalid qualified contract subject",
        ));
    }
    let cursor = attempt!(cursor.expected("input"));
    let (cursor, inputs) = attempt!(binding_block(cursor));
    let cursor = attempt!(cursor.expected("output"));
    let (cursor, outputs) = attempt!(binding_block(cursor));
    let cursor = attempt!(cursor.expected("requires"));
    let (cursor, requires) = attempt!(logical(cursor));
    let cursor = attempt!(cursor.expected("ensures"));
    let (mut cursor, ensures) = attempt!(logical(cursor));
    let Some(close) = cursor.tokens.get(cursor.at) else {
        return Err(super::parse_diagnostic(
            span,
            "unclosed contract declaration",
        ));
    };
    if !matches!(&close.kind, crate::source::lexer::TokenKind::Close) {
        return Err(super::parse_diagnostic(
            close.span,
            "contract contains trailing fields",
        ));
    }
    cursor.at += 1;
    Ok((
        cursor,
        super::Member::Contract(super::Contract {
            revision,
            name,
            subject: alloc::string::String::from(subject),
            subject_span,
            inputs,
            outputs,
            requires,
            ensures,
            span: crate::Span {
                start: span.start,
                end: close.span.end,
            },
        }),
    ))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn proof(
    cursor: super::Cursor<'_>,
    span: crate::Span,
) -> Result<(super::Cursor<'_>, super::Member), crate::Diagnostic> {
    let (cursor, revision) = attempt!(revision(cursor));
    let (mut cursor, name) = attempt!(cursor.logical_ident());
    let Some(head) = cursor.tokens.get(cursor.at) else {
        return Err(super::parse_diagnostic(span, "missing proof claim"));
    };
    let claim = match &head.kind {
        crate::source::lexer::TokenKind::ProofColon => {
            if revision != 1 {
                return Err(super::parse_diagnostic(
                    head.span,
                    "proof revision 2 requires a contract claim",
                ));
            }
            cursor.at += 1;
            let (next, proposition) = attempt!(logical(cursor));
            cursor = next;
            super::ProofClaim::Pure(proposition)
        }
        crate::source::lexer::TokenKind::Word(bytes) if bytes == b"for" => {
            cursor.at += 1;
            let Some(target_span) = cursor.tokens.get(cursor.at).map(|token| token.span) else {
                return Err(super::parse_diagnostic(span, "missing proof contract"));
            };
            let (next, target) = attempt!(cursor.next());
            if !reference(target, true) {
                return Err(super::parse_diagnostic(
                    target_span,
                    "invalid qualified proof contract",
                ));
            }
            cursor = next;
            super::ProofClaim::For(alloc::string::String::from(target), target_span)
        }
        _ => {
            return Err(super::parse_diagnostic(
                head.span,
                "expected proof for or : claim",
            ))
        }
    };
    let (cursor, term) = attempt!(logical(cursor));
    let Some(last) = cursor
        .at
        .checked_sub(1)
        .and_then(|index| cursor.tokens.get(index))
    else {
        return Err(super::parse_diagnostic(
            span,
            "proof term has no closing token",
        ));
    };
    let end = last.span.end;
    Ok((
        cursor,
        super::Member::Proof(super::Proof {
            revision,
            name,
            claim,
            term,
            span: crate::Span {
                start: span.start,
                end,
            },
        }),
    ))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn signature(
    cursor: super::Cursor<'_>,
) -> Result<(super::Cursor<'_>, super::Member), crate::Diagnostic> {
    let (cursor, name) = attempt!(cursor.ident());
    let (mut cursor, binders) = attempt!(cursor.next());
    if !binders.starts_with("forall<") || !binders.ends_with('>') {
        return Err(super::parse_diagnostic(
            cursor.span,
            "invalid signature binder list",
        ));
    }
    if !super::open_at(cursor.tokens, cursor.at) {
        return Err(super::parse_diagnostic(
            cursor.span,
            "signature requires bracketed interface",
        ));
    }
    let interface_start = cursor.tokens[cursor.at].span.start;
    cursor.at += 1;
    let mut words = alloc::vec::Vec::new();
    while cursor.at < cursor.tokens.len() {
        if let crate::source::lexer::TokenKind::Close = &cursor.tokens[cursor.at].kind {
            let end = cursor.tokens[cursor.at].span.end;
            cursor.at += 1;
            return Ok((
                cursor,
                super::Member::Signature {
                    name,
                    binders: alloc::string::String::from(binders),
                    words,
                    span: crate::Span {
                        start: interface_start,
                        end,
                    },
                },
            ));
        }
        let (next, word) = attempt!(cursor.next());
        // Each word consumes one token of this bracketed interface.
        if words.len() >= next.tokens.len() {
            return Err(super::parse_diagnostic(
                next.span,
                "signature interface exceeds its token block",
            ));
        }
        words.push(alloc::string::String::from(word));
        cursor = next;
    }
    Err(super::parse_diagnostic(
        cursor.span,
        "unclosed signature interface",
    ))
}

fn opaque(
    cursor: super::Cursor<'_>,
) -> Result<(super::Cursor<'_>, super::Member), crate::Diagnostic> {
    let (cursor, name) = attempt!(cursor.ident());
    let (cursor, base) = attempt!(cursor.next());
    let base = alloc::string::String::from(base);
    let (cursor, public) = attempt!(cursor.public());
    Ok((cursor, super::Member::Opaque { name, base, public }))
}

fn variant(
    cursor: super::Cursor<'_>,
) -> Result<(super::Cursor<'_>, super::Member), crate::Diagnostic> {
    let (cursor, spelling) = attempt!(cursor.next());
    let (name, parameters) = attempt!(variant_name(spelling, cursor.span));
    let (cursor, left, left_type, left_public) = attempt!(variant_arm(cursor));
    let (cursor, right, right_type, right_public) = attempt!(variant_arm(cursor));
    Ok((
        cursor,
        super::Member::Variant {
            name,
            parameters,
            left,
            left_type,
            left_public,
            right,
            right_type,
            right_public,
        },
    ))
}

#[expect(
    tigerstyle::compound_condition,
    reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn variant_name(
    spelling: &str,
    span: crate::Span,
) -> Result<
    (
        alloc::string::String,
        alloc::vec::Vec<alloc::string::String>,
    ),
    crate::Diagnostic,
> {
    let Some((name, arguments)) = spelling.split_once('<') else {
        if super::name(spelling) {
            return Ok((
                alloc::string::String::from(spelling),
                alloc::vec::Vec::new(),
            ));
        }
        return Err(super::parse_diagnostic(span, "invalid variant name"));
    };
    let Some(arguments) = arguments.strip_suffix('>') else {
        return Err(super::parse_diagnostic(
            span,
            "invalid generic variant parameters",
        ));
    };
    let Some((first, second)) = arguments.split_once(',') else {
        return Err(super::parse_diagnostic(
            span,
            "generic variant requires two type parameters",
        ));
    };
    if !super::name(name) || !super::name(first) || !super::name(second) || first == second {
        return Err(super::parse_diagnostic(
            span,
            "invalid generic variant parameters",
        ));
    }
    Ok((
        alloc::string::String::from(name),
        alloc::vec![
            alloc::string::String::from(first),
            alloc::string::String::from(second)
        ],
    ))
}

fn variant_arm(
    cursor: super::Cursor<'_>,
) -> Result<
    (
        super::Cursor<'_>,
        alloc::string::String,
        alloc::string::String,
        bool,
    ),
    crate::Diagnostic,
> {
    let (cursor, left) = attempt!(cursor.ident());
    // `match` is the variant's generated eliminator suffix, so an arm with
    // that spelling would publish two words spelled `V.match` and make the
    // constructor unreachable.
    if left == "match" {
        return Err(super::parse_diagnostic(
            cursor.span,
            "variant arm 'match' collides with the generated eliminator",
        ));
    }
    let (cursor, left_type) = attempt!(cursor.next());
    let left_type = alloc::string::String::from(left_type);
    let (cursor, left_public) = attempt!(cursor.public());
    Ok((cursor, left, left_type, left_public))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn require(
    cursor: super::Cursor<'_>,
) -> Result<(super::Cursor<'_>, super::Member), crate::Diagnostic> {
    let (cursor, name) = attempt!(cursor.ident());
    if super::word(cursor.tokens.get(cursor.at), cursor.span).ok() == Some("--") {
        let cursor = attempt!(cursor.expected("--"));
        let cursor = attempt!(cursor.expected("I64"));
        let cursor = attempt!(cursor.expected("!"));
        let cursor = attempt!(cursor.expected("test.clock"));
        return Ok((
            cursor,
            super::Member::Require {
                name,
                input: alloc::string::String::new(),
                operation: alloc::string::String::from("test.clock"),
            },
        ));
    }
    let (cursor, input) = attempt!(cursor.next());
    let (cursor, input) = if input == "Text"
        && super::word(cursor.tokens.get(cursor.at), cursor.span).ok() == Some("Text")
    {
        (
            attempt!(cursor.expected("Text")),
            alloc::string::String::from("Text Text"),
        )
    } else {
        (cursor, alloc::string::String::from(input))
    };
    let cursor = attempt!(cursor.expected("--"));
    let cursor = attempt!(cursor.expected("!"));
    let (cursor, operation) = attempt!(cursor.next());
    Ok((
        cursor,
        super::Member::Require {
            name,
            input,
            operation: alloc::string::String::from(operation),
        },
    ))
}

fn export(
    cursor: super::Cursor<'_>,
) -> Result<(super::Cursor<'_>, super::Member), crate::Diagnostic> {
    let span = cursor.span;
    let (cursor, target) = attempt!(cursor.next());
    if !exported_name(target) {
        return Err(super::parse_diagnostic(
            span,
            "invalid exported schema, operation or definition",
        ));
    }
    Ok((
        cursor,
        super::Member::Export(alloc::string::String::from(target)),
    ))
}

fn exported_name(target: &str) -> bool {
    let bytes = target.as_bytes();
    let mut split = 0;
    while split < bytes.len() && bytes[split] != b'.' {
        split += 1;
    }
    let base = &target[..split];
    if !super::name(base) && !super::logical_name(base) {
        return false;
    }
    split == bytes.len()
        || match split.checked_add(1) {
            Some(next) => super::name(&target[next..]) || super::logical_name(&target[next..]),
            None => false,
        }
}
