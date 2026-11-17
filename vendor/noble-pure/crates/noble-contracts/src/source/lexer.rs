#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; scanning advances only a preparation-owned cursor, decoded token buffers, and meter within admitted source-byte and work/node bounds; original source bytes remain borrowed immutable and no committed namespace is touched."
)]

mod text;
pub(super) mod tokens;

pub(super) enum TokenKind {
    Open,
    Close,
    Def,
    Literal(noble_kernel::untrusted::Lit),
    Text(alloc::vec::Vec<u8>),
    Word(alloc::vec::Vec<u8>),
    ProofColon,
    LogicalBody(alloc::vec::Vec<LogicalToken>),
}

pub(super) enum LogicalKind {
    OpenParen,
    CloseParen,
    Colon,
    Comma,
    Atom(alloc::vec::Vec<u8>),
}

pub(super) struct LogicalToken {
    pub kind: LogicalKind,
    pub span: crate::Span,
}

pub(super) struct Token {
    pub kind: TokenKind,
    pub span: crate::Span,
}

pub(super) struct Scanner<'a> {
    source: &'a [u8],
    at: usize,
    full: crate::Span,
    declared: bool,
}

impl<'a> Scanner<'a> {
    pub fn new(
        source_bytes: &'a [u8],
        meter: &mut crate::Meter,
    ) -> Result<Self, crate::Diagnostic> {
        let full = crate::Span {
            start: 0,
            end: attempt!(crate::index(
                source_bytes.len(),
                crate::Span { start: 0, end: 0 }
            )),
        };
        if full.end > meter.limits.bytes {
            return Err(super::exhausted(full, "source byte limit exceeded"));
        }
        attempt!(meter.charge(full.end, full));
        if let Err(error) = core::str::from_utf8(source_bytes) {
            let start = attempt!(crate::index(error.valid_up_to(), full));
            let end = match error.error_len() {
                Some(length) => start.saturating_add(attempt!(crate::index(length, full))),
                None => full.end,
            };
            return Err(crate::invalid(
                crate::Span { start, end },
                "invalid UTF-8 source encoding",
            ));
        }
        Ok(Self {
            source: source_bytes,
            at: 0,
            full,
            declared: false,
        })
    }

    pub fn new_declared(
        source_bytes: &'a [u8],
        meter: &mut crate::Meter,
    ) -> Result<Self, crate::Diagnostic> {
        let mut scanner = attempt!(Self::new(source_bytes, meter));
        scanner.declared = true;
        Ok(scanner)
    }

    pub fn next(&mut self, meter: &mut crate::Meter) -> Result<Option<Token>, crate::Diagnostic> {
        attempt!(self.skip(meter));
        let start = self.at;
        let byte = match self.source.get(self.at).copied() {
            Some(byte) => byte,
            None => return Ok(None),
        };
        attempt!(meter.node(self.full));
        let kind = match byte {
            b'[' => {
                self.at += 1;
                TokenKind::Open
            }
            b']' => {
                self.at += 1;
                TokenKind::Close
            }
            b'"' => TokenKind::Text(attempt!(self.text(meter))),
            _ => attempt!(self.word(meter)),
        };
        Ok(Some(Token {
            kind,
            span: attempt!(self.span(start)),
        }))
    }

    /// The `:` separator is a declaration token only after `proof 1 Name`.
    pub fn next_proof_head(
        &mut self,
        meter: &mut crate::Meter,
    ) -> Result<Option<Token>, crate::Diagnostic> {
        attempt!(self.skip(meter));
        if self.source.get(self.at) != Some(&b':') {
            return self.next(meter);
        }
        let start = self.at;
        attempt!(meter.node(self.full));
        self.at += 1;
        Ok(Some(Token {
            kind: TokenKind::ProofColon,
            span: attempt!(self.span(start)),
        }))
    }

    /// Consume one *complete* bracketed logical body. Ordinary `next` never
    /// recognizes parentheses, so malformed proof text cannot escape into a
    /// definition or alter executable word tokenization.
    pub fn next_logical_body(
        &mut self,
        meter: &mut crate::Meter,
    ) -> Result<Token, crate::Diagnostic> {
        attempt!(self.skip(meter));
        let start = self.at;
        if !self.declared || self.source.get(start) != Some(&b'[') {
            return Err(crate::invalid(self.full, "expected bracketed logical body"));
        }
        attempt!(meter.node(self.full));
        attempt!(meter.depth(1, self.full));
        self.at += 1;
        let mut tokens = alloc::vec::Vec::new();
        let mut parens = 0u32;
        loop {
            attempt!(self.skip(meter));
            let token_start = self.at;
            let Some(byte) = self.source.get(self.at).copied() else {
                return Err(crate::invalid(self.full, "unclosed logical body"));
            };
            if byte == b']' {
                if parens != 0 {
                    return Err(crate::invalid(
                        attempt!(self.span(token_start)),
                        "unclosed logical parenthesis",
                    ));
                }
                attempt!(meter.node(self.full));
                self.at += 1;
                return Ok(Token {
                    kind: TokenKind::LogicalBody(tokens),
                    span: attempt!(self.span(start)),
                });
            }
            attempt!(meter.node(self.full));
            let kind = match byte {
                b'(' => {
                    parens = attempt!(parens.checked_add(1).ok_or_else(|| {
                        crate::invalid(self.full, "logical nesting exceeds finite format")
                    }));
                    if parens > 64 {
                        return Err(crate::invalid(
                            attempt!(self.span(token_start)),
                            "logical nesting exceeds supported depth",
                        ));
                    }
                    attempt!(meter.depth(parens.saturating_add(1), self.full));
                    self.at += 1;
                    LogicalKind::OpenParen
                }
                b')' => {
                    let Some(next) = parens.checked_sub(1) else {
                        return Err(crate::invalid(self.full, "unmatched logical parenthesis"));
                    };
                    parens = next;
                    self.at += 1;
                    LogicalKind::CloseParen
                }
                b':' => {
                    self.at += 1;
                    LogicalKind::Colon
                }
                b',' => {
                    self.at += 1;
                    LogicalKind::Comma
                }
                b'[' | b'"' | b'\\' => {
                    return Err(crate::invalid(
                        self.full,
                        "invalid logical delimiter or escape",
                    ));
                }
                _ => LogicalKind::Atom(attempt!(self.logical_atom(meter))),
            };
            // Every logical token, including a nonempty atom, consumes a source byte.
            if tokens.len() >= self.source.len() {
                return Err(crate::invalid(
                    self.full,
                    "logical body exceeds its source bytes",
                ));
            }
            tokens.push(LogicalToken {
                kind,
                span: attempt!(self.span(token_start)),
            });
        }
    }

    fn logical_atom(
        &mut self,
        meter: &mut crate::Meter,
    ) -> Result<alloc::vec::Vec<u8>, crate::Diagnostic> {
        let start = self.at;
        while let Some(byte) = self.source.get(self.at).copied() {
            if whitespace(byte) || matches!(byte, b'#' | b'[' | b']' | b'(' | b')' | b':' | b',') {
                break;
            }
            if !(byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'_' | b'+'
                        | b'-'
                        | b'*'
                        | b'/'
                        | b'='
                        | b'<'
                        | b'>'
                        | b'?'
                        | b'!'
                        | b'.'
                        | b'@'
                ))
            {
                return Err(crate::invalid(self.full, "unsupported logical atom"));
            }
            attempt!(meter.charge(1, self.full));
            self.at += 1;
        }
        if self.at == start {
            return Err(crate::invalid(self.full, "unsupported logical atom"));
        }
        Ok(self.source[start..self.at].to_vec())
    }

    fn skip(&mut self, meter: &mut crate::Meter) -> Result<(), crate::Diagnostic> {
        let mut failure = None;
        while let Some(byte) = self.source.get(self.at).copied() {
            match self.skip_step(byte, meter) {
                Ok(true) => {}
                Ok(false) => break,
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(()),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; trivia scanning charges source work and can construct an exhaustion diagnostic; comment scanning is a bounded runtime traversal."
    )]
    fn skip_step(&mut self, byte: u8, meter: &mut crate::Meter) -> Result<bool, crate::Diagnostic> {
        attempt!(meter.charge(1, self.full));
        if whitespace(byte) {
            self.at += 1;
            Ok(true)
        } else if byte == b'#' {
            attempt!(self.comment(meter));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn comment(&mut self, meter: &mut crate::Meter) -> Result<(), crate::Diagnostic> {
        let mut failure = None;
        while let Some(byte) = self.source.get(self.at).copied() {
            if let Err(problem) = meter.charge(1, self.full) {
                failure = Some(problem);
                break;
            }
            if byte == b'\r' || byte == b'\n' {
                break;
            }
            self.at += 1;
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(()),
        }
    }

    fn word(&mut self, meter: &mut crate::Meter) -> Result<TokenKind, crate::Diagnostic> {
        let start = self.at;
        let mut failure = None;
        while let Some(byte) = self.source.get(self.at).copied() {
            if let Err(problem) = meter.charge(1, self.full) {
                failure = Some(problem);
                break;
            }
            if whitespace(byte) || matches!(byte, b'#' | b'[' | b']' | b'"') {
                break;
            }
            self.at += 1;
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        let span = attempt!(self.span(start));
        let bytes = match self.source.get(start..self.at) {
            Some(bytes) => bytes,
            None => return Err(crate::internal(span)),
        };
        tokens::classify(bytes, span, meter, self.declared)
    }

    fn span(&self, start: usize) -> Result<crate::Span, crate::Diagnostic> {
        Ok(crate::Span {
            start: attempt!(crate::index(start, self.full)),
            end: attempt!(crate::index(self.at, self.full)),
        })
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; consuming an escape byte charges work and returns an owned unterminated-literal diagnostic on missing input."
    )]
    fn byte(&mut self, meter: &mut crate::Meter) -> Result<u8, crate::Diagnostic> {
        attempt!(meter.charge(1, self.full));
        match self.source.get(self.at).copied() {
            Some(byte) => {
                self.at += 1;
                Ok(byte)
            }
            None => Err(crate::invalid(
                self.full,
                "unterminated text literal or escape",
            )),
        }
    }
}

const fn whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}
