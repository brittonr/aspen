//! Bounded lexical classification of declaration units.
mod body;
mod imports;
mod logic;
mod members;

// Module/import unit forms are unchanged; only logical contract/proof
// declarations admit the additive explicit revision 2.
#[octet::sealed_enum]
pub(super) enum ParsedUnit {
    Module {
        name: alloc::string::String,
        version: u32,
        members: alloc::vec::Vec<Member>,
    },
    Import {
        name: alloc::string::String,
        version: u32,
        alias: alloc::string::String,
    },
    Definition,
    Expression,
}

// Member kinds remain closed; new logical revisions retain the same member fields.
#[octet::sealed_enum]
pub(super) enum Member {
    Opaque {
        name: alloc::string::String,
        base: alloc::string::String,
        public: bool,
    },
    Variant {
        name: alloc::string::String,
        parameters: alloc::vec::Vec<alloc::string::String>,
        left: alloc::string::String,
        left_type: alloc::string::String,
        left_public: bool,
        right: alloc::string::String,
        right_type: alloc::string::String,
        right_public: bool,
    },
    Require {
        name: alloc::string::String,
        input: alloc::string::String,
        operation: alloc::string::String,
    },
    Signature {
        name: alloc::string::String,
        binders: alloc::string::String,
        words: alloc::vec::Vec<alloc::string::String>,
        span: crate::Span,
    },
    Export(alloc::string::String),
    Definition {
        name: alloc::string::String,
        bytes: alloc::vec::Vec<u8>,
        span: crate::Span,
        body_span: crate::Span,
    },
    Contract(Contract),
    Proof(Proof),
}

pub(super) struct Binding {
    pub name: alloc::string::String,
    pub ty: alloc::string::String,
    pub span: crate::Span,
}

pub(super) struct Contract {
    pub revision: u32,
    pub name: alloc::string::String,
    pub subject: alloc::string::String,
    pub subject_span: crate::Span,
    pub inputs: alloc::vec::Vec<Binding>,
    pub outputs: alloc::vec::Vec<Binding>,
    pub requires: crate::intrinsic::Form,
    pub ensures: crate::intrinsic::Form,
    pub span: crate::Span,
}

pub(super) enum ProofClaim {
    For(alloc::string::String, crate::Span),
    Pure(crate::intrinsic::Form),
}

pub(super) struct Proof {
    pub revision: u32,
    pub name: alloc::string::String,
    pub claim: ProofClaim,
    pub term: crate::intrinsic::Form,
    pub span: crate::Span,
}

fn parse_diagnostic(span: crate::Span, reason: &str) -> crate::Diagnostic {
    crate::invalid(span, reason)
}

fn open_at(tokens: &[crate::source::lexer::Token], at: usize) -> bool {
    let Some(token) = tokens.get(at) else {
        return false;
    };
    let crate::source::lexer::TokenKind::Open = &token.kind else {
        return false;
    };
    true
}

fn word(
    token: Option<&crate::source::lexer::Token>,
    span: crate::Span,
) -> Result<&str, crate::Diagnostic> {
    let Some(crate::source::lexer::Token {
        kind: crate::source::lexer::TokenKind::Word(bytes),
        ..
    }) = token
    else {
        return Err(parse_diagnostic(span, "expected declaration word"));
    };
    core::str::from_utf8(bytes).map_err(|_| parse_diagnostic(span, "invalid UTF-8 word"))
}

pub(super) fn name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let first = bytes.next();
    matches!(first, Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        && bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_')
        && !reserved_name(name)
}

// Intrinsic declaration identifiers include hyphenated proof/theorem names,
// without widening ordinary definition, type, module, or alias identifiers.
pub(super) fn logical_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        && bytes.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        && !reserved_name(name)
}

fn reserved_name(name: &str) -> bool {
    matches!(
        name,
        "def"
            | "module"
            | "import"
            | "as"
            | "opaque"
            | "variant"
            | "require"
            | "signature"
            | "export"
            | "public"
            | "private"
            | "true"
            | "false"
            | "I64"
            | "Bool"
            | "Text"
            | "Unit"
            | "Pair"
            | "Sum"
            | "List"
            | "Resource"
            | "Program"
            | "Syntax"
    )
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; str.bytes and Iterator::all are non-const on pinned compilers; this validates canonical decimal versions; reassess const iteration."
)]
fn valid_version(version: &str) -> bool {
    !version.is_empty()
        && version.bytes().all(|c| c.is_ascii_digit())
        && version.as_bytes().first() != Some(&b'0')
}

fn versioned(
    text: &str,
    span: crate::Span,
) -> Result<(alloc::string::String, u32), crate::Diagnostic> {
    let (name_part, version) = attempt!(super::split_once_ascii(text, b'@')
        .ok_or_else(|| parse_diagnostic(span, "module requires name@version")));
    if !name(name_part) || !valid_version(version) {
        return Err(parse_diagnostic(span, "invalid module name or version"));
    }
    let version = attempt!(version
        .parse::<u32>()
        .map_err(|_| parse_diagnostic(span, "module version overflows")));
    Ok((alloc::string::String::from(name_part), version))
}

fn visibility(text: &str, span: crate::Span) -> Result<bool, crate::Diagnostic> {
    if text.eq("public") {
        return Ok(true);
    }
    if text.eq("private") {
        return Ok(false);
    }
    Err(parse_diagnostic(
        span,
        "expected public or private visibility",
    ))
}

struct Cursor<'a> {
    tokens: &'a [crate::source::lexer::Token],
    at: usize,
    span: crate::Span,
}

impl<'a> Cursor<'a> {
    fn next(mut self) -> Result<(Self, &'a str), crate::Diagnostic> {
        let value = attempt!(word(self.tokens.get(self.at), self.span));
        self.at += 1;
        Ok((self, value))
    }
    fn ident(self) -> Result<(Self, alloc::string::String), crate::Diagnostic> {
        let span = self.span;
        let (cursor, value) = attempt!(self.next());
        if !name(value) {
            return Err(parse_diagnostic(
                span,
                "invalid or reserved declaration name",
            ));
        }
        Ok((cursor, alloc::string::String::from(value)))
    }
    fn logical_ident(self) -> Result<(Self, alloc::string::String), crate::Diagnostic> {
        let span = self.span;
        let (cursor, value) = attempt!(self.next());
        if !logical_name(value) {
            return Err(parse_diagnostic(span, "invalid logical declaration name"));
        }
        Ok((cursor, alloc::string::String::from(value)))
    }
    fn expected(self, expected: &str) -> Result<Self, crate::Diagnostic> {
        let span = self.span;
        let (cursor, actual) = attempt!(self.next());
        if actual.eq(expected) {
            Ok(cursor)
        } else {
            Err(parse_diagnostic(span, "invalid declaration signature"))
        }
    }
    fn public(self) -> Result<(Self, bool), crate::Diagnostic> {
        let span = self.span;
        let (cursor, value) = attempt!(self.next());
        Ok((cursor, attempt!(visibility(value, span))))
    }
}

#[expect(
    tigerstyle::interior_mutability_in_core,
    reason = "Owner: noble-contracts; K-CHECK-07 request budget: one request-local Rc<Cell<u32>> counter a host opts into with ModuleSession::share_request_work; it is single-threaded (Rc is !Send), never outlives the session and its staged snapshots, and every meter debits it so nested checking cannot restart the request's budget. Explicit threading would turn the &self closure-walk and origin-replay APIs into &mut ones."
)]
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-contracts; the only mutable input is the K-CHECK-07 request budget, a request-local counter debited by the meters of this checking stage; no source, published compiler state or caller value is mutated."
)]
pub(super) fn parse(
    input_bytes: &[u8],
    limits: crate::Limits,
    request_work: Option<&alloc::rc::Rc<core::cell::Cell<u32>>>,
) -> Result<ParsedUnit, crate::Diagnostic> {
    let (tokens, span) = attempt!(scan(input_bytes, limits, request_work));
    let first = tokens.first();
    let is_module = declaration_word(first, b"module");
    let is_import = declaration_word(first, b"import");
    if !is_module && !is_import {
        return Ok(ordinary_unit(first));
    }
    let cursor = Cursor {
        tokens: &tokens,
        at: 1,
        span,
    };
    parse_declaration(input_bytes, limits, cursor, is_import)
}

fn parse_declaration(
    input_bytes: &[u8],
    limits: crate::Limits,
    cursor: Cursor<'_>,
    is_import: bool,
) -> Result<ParsedUnit, crate::Diagnostic> {
    let span = cursor.span;
    let (cursor, version_word) = attempt!(cursor.next());
    let (name, version) = attempt!(versioned(version_word, span));
    if is_import {
        return imports::parse(cursor, name, version);
    }
    members::parse(input_bytes, limits, cursor, name, version)
}

fn ordinary_unit(first: Option<&crate::source::lexer::Token>) -> ParsedUnit {
    if matches!(
        first,
        Some(crate::source::lexer::Token {
            kind: crate::source::lexer::TokenKind::Def,
            ..
        })
    ) {
        ParsedUnit::Definition
    } else {
        ParsedUnit::Expression
    }
}

fn declaration_word(token: Option<&crate::source::lexer::Token>, spelling: &[u8]) -> bool {
    matches!(token, Some(crate::source::lexer::Token {
        kind: crate::source::lexer::TokenKind::Word(bytes), ..
    }) if bytes.as_slice().eq(spelling))
}

#[derive(Clone, Copy)]
enum ScanContext {
    Member,
    ExportTarget,
    ContractVersion,
    ContractName,
    ContractOpen,
    ContractBody,
    ContractLogical,
    ProofVersion,
    ProofName,
    ProofHeader,
    ProofTarget,
    ProofProposition,
    ProofTerm,
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn logical_revision(token: &crate::source::lexer::Token) -> Option<u32> {
    if token.span.end.checked_sub(token.span.start) != Some(1) {
        return None;
    }
    match &token.kind {
        crate::source::lexer::TokenKind::Literal(noble_kernel::untrusted::Lit::I64(1)) => Some(1),
        crate::source::lexer::TokenKind::Literal(noble_kernel::untrusted::Lit::I64(2)) => Some(2),
        _ => None,
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn word_is(token: &crate::source::lexer::Token, spelling: &[u8]) -> bool {
    matches!(&token.kind, crate::source::lexer::TokenKind::Word(bytes) if bytes == spelling)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
)]
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; this stage imports the domain types it operates on directly, matching the module vocabulary"
)]
fn scan_context(
    context: ScanContext,
    token: &crate::source::lexer::Token,
    before_depth: u32,
    after_depth: u32,
) -> ScanContext {
    use crate::source::lexer::TokenKind;
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
    )]
    #[expect(
        tigerstyle::renamed_imports,
        reason = "Owner: noble-contracts; the local alias matches the domain vocabulary used throughout this module"
    )]
    use ScanContext as C;
    match context {
        C::Member if before_depth == 1 && word_is(token, b"export") => C::ExportTarget,
        C::ExportTarget => C::Member,
        C::Member if before_depth == 1 && word_is(token, b"contract") => C::ContractVersion,
        C::Member if before_depth == 1 && word_is(token, b"proof") => C::ProofVersion,
        C::Member => C::Member,
        C::ContractVersion if logical_revision(token).is_some() => C::ContractName,
        C::ContractName if matches!(&token.kind, TokenKind::Word(_)) => C::ContractOpen,
        C::ContractOpen if matches!(&token.kind, TokenKind::Open) => C::ContractBody,
        C::ContractBody if after_depth == 1 => C::Member,
        C::ContractBody if before_depth == 2 && word_is(token, b"requires") => C::ContractLogical,
        C::ContractBody if before_depth == 2 && word_is(token, b"ensures") => C::ContractLogical,
        C::ContractBody => C::ContractBody,
        C::ContractLogical => C::ContractBody,
        C::ProofVersion if logical_revision(token).is_some() => C::ProofName,
        C::ProofName if matches!(&token.kind, TokenKind::Word(_)) => C::ProofHeader,
        C::ProofHeader if matches!(&token.kind, TokenKind::ProofColon) => C::ProofProposition,
        C::ProofHeader if word_is(token, b"for") => C::ProofTarget,
        C::ProofTarget if matches!(&token.kind, TokenKind::Word(_)) => C::ProofTerm,
        C::ProofProposition => C::ProofTerm,
        C::ProofTerm => C::Member,
        _ => C::Member,
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; bounded Scanner.next and Meter preserve the first lexical Diagnostic before checked-span construction; guest errors must not panic; reassess fallible-check density."
)]
#[expect(
    tigerstyle::interior_mutability_in_core,
    reason = "Owner: noble-contracts; K-CHECK-07 request budget: one request-local Rc<Cell<u32>> counter a host opts into with ModuleSession::share_request_work; it is single-threaded (Rc is !Send), never outlives the session and its staged snapshots, and every meter debits it so nested checking cannot restart the request's budget. Explicit threading would turn the &self closure-walk and origin-replay APIs into &mut ones."
)]
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-contracts; the only mutable input is the K-CHECK-07 request budget, a request-local counter debited by the meters of this checking stage; no source, published compiler state or caller value is mutated."
)]
fn scan(
    input_bytes: &[u8],
    limits: crate::Limits,
    request_work: Option<&alloc::rc::Rc<core::cell::Cell<u32>>>,
) -> Result<(alloc::vec::Vec<crate::source::lexer::Token>, crate::Span), crate::Diagnostic> {
    let mut meter = crate::Meter::pooled(limits, request_work);
    let mut scanner = attempt!(crate::source::lexer::Scanner::new_declared(
        input_bytes,
        &mut meter
    ));
    let initial_slots = attempt!(token_slots(limits, input_bytes.len()));
    let mut tokens = alloc::vec::Vec::with_capacity(initial_slots);
    let mut context = ScanContext::Member;
    let mut depth = 0u32;
    let mut next = scanner.next(&mut meter);
    let mut failure = None;
    while let Some(token) = match next {
        Ok(token) => token,
        Err(problem) => {
            failure = Some(problem);
            None
        }
    } {
        let before_depth = depth;
        depth = match &token.kind {
            crate::source::lexer::TokenKind::Open => match depth.checked_add(1) {
                Some(next) => next,
                None => {
                    failure = Some(parse_diagnostic(
                        token.span,
                        "module nesting exceeds finite format",
                    ));
                    break;
                }
            },
            crate::source::lexer::TokenKind::Close => depth.saturating_sub(1),
            _ => depth,
        };
        context = scan_context(context, &token, before_depth, depth);
        tokens.push(token);
        next = match context {
            ScanContext::ContractLogical
            | ScanContext::ProofProposition
            | ScanContext::ProofTerm => scanner.next_logical_body(&mut meter).map(Some),
            ScanContext::ProofHeader => scanner.next_proof_head(&mut meter),
            _ => scanner.next(&mut meter),
        };
    }
    if let Some(problem) = failure {
        return Err(problem);
    }
    let span = crate::Span {
        start: 0,
        end: attempt!(crate::index(
            input_bytes.len(),
            crate::Span { start: 0, end: 0 }
        )),
    };
    Ok((tokens, span))
}

fn token_slots(limits: crate::Limits, source_bytes: usize) -> Result<usize, crate::Diagnostic> {
    usize::try_from(limits.nodes)
        .map(|max_tokens| max_tokens.min(source_bytes))
        .map_err(|_| {
            parse_diagnostic(
                crate::Span { start: 0, end: 0 },
                "token limit exceeds the host address space",
            )
        })
}
