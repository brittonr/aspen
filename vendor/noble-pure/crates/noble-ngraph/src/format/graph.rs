//! crate::ir::Graph files: the identity-complete text form of `ngraph-ir/0` (design D4
//! of `numeric-graph-v1`; option B0(b) authoring). Parsing only builds
//! producer data; [`crate::check`] decides admission.
//!
//! ```text
//! ngraph-ir/0 ngraph-f32/0
//! subject authored 3be1…(64 hex) closure -   # or: subject authored-name NAME
//! shape input s (p s u)                      # optional; trees s | u | (p A B)
//! shape output s
//! slot input f32
//! slot param f32
//! %0 = input 0
//! %1 = param 1
//! %2 = const f32 1.5            # decimal must be exact; or 0x3fc00000 bits
//! %3 = f32.affine -0 1.5 %0 2 one   # bias, then (coefficient operand)*
//! %4 = f32.index %0 refuse clamp %2 %3
//! %5 = guard %0 range 0 1
//! fact %0 integral input-validated
//! fact %3 finite derived interval
//! out %3
//! ```
//!
//! - `subject <authored|authored-nickel|extracted|captured> <64 hex> closure <ids|->` is
//!   required exactly once; `subject authored-name NAME` (one or more tokens,
//!   joined by single spaces) is authoring sugar for
//!   [`crate::ir::Subject::authored`], and [`print`] always writes the hex form.
//! - `shape input|output <trees…>` records `Some(trees)`; an absent line is
//!   `None`.
//! - Range-fact literals: `0x…` is binary32, `true|false` Bool, `i64:N` an
//!   explicit `I64`; an unprefixed decimal takes the sub-profile's type.
//! - `derived <rule>` requires its rule; rules and evidence witnesses are
//!   printed so that they parse back exactly (percent-encoded rule bytes,
//!   hex witnesses, `""`/`-` for empty ones).
//!
//! `GraphLogicalId` is recomputed from the parsed records alone; comments,
//! whitespace, file names and paths are never identity-bearing, and producer
//! origins are not printed. [`print`] is canonical: `parse(print(g))` equals
//! `g` with origins cleared.
//!
//! Codes: unknown operations, syntax errors (including a number with a
//! leading `+`) and a missing, duplicate or malformed subject or shape line
//! (including a shape tree nesting more than [`MAX_SHAPE_DEPTH`] pairs) give
//! `E-GRAPH-MALFORMED`; an unknown sub-profile `E-PROFILE`; inexact decimal
//! literals `E-LITERAL-INEXACT`; `nan`/`inf` literals `E-LITERAL-NONFINITE`
//! (non-finite bit patterns are refused by the checker).

use alloc::borrow::ToOwned;
use alloc::string::ToString;

use crate::identity::IdentityScheme;

fn syntax(line: usize, why: impl Into<alloc::string::String>) -> crate::diag::Diagnostic {
    crate::diag::Diagnostic::malformed(
        crate::diag::Malformed::Syntax,
        crate::diag::Site::Line(line),
        why,
    )
}

struct Toks<'a> {
    t: alloc::vec::Vec<&'a str>,
    i: usize,
    line: usize,
    mode: super::LiteralMode,
}

impl<'a> Toks<'a> {
    fn next(&mut self) -> Result<&'a str, crate::diag::Diagnostic> {
        let t = self
            .t
            .get(self.i)
            .copied()
            .ok_or_else(|| syntax(self.line, "unexpected end of line"))?;
        self.i += 1;
        Ok(t)
    }
    fn peek(&self) -> Option<&'a str> {
        self.t.get(self.i).copied()
    }
    /// The remaining tokens of the line, without copying the vector.
    fn rest(&mut self) -> &[&'a str] {
        let r = &self.t[self.i..];
        self.i = self.t.len();
        r
    }
    fn done(&self) -> Result<(), crate::diag::Diagnostic> {
        if self.i == self.t.len() {
            Ok(())
        } else {
            Err(syntax(
                self.line,
                alloc::format!("trailing token {:?}", self.t[self.i]),
            ))
        }
    }
    fn node(&mut self) -> Result<crate::ir::NodeId, crate::diag::Diagnostic> {
        let t = self.next()?;
        t.strip_prefix('%')
            .and_then(super::decimal_u32)
            .map(crate::ir::NodeId)
            .ok_or_else(|| syntax(self.line, alloc::format!("expected %node, got {t:?}")))
    }
    fn u32(&mut self) -> Result<u32, crate::diag::Diagnostic> {
        let t = self.next()?;
        super::decimal_u32(t)
            .ok_or_else(|| syntax(self.line, alloc::format!("expected integer, got {t:?}")))
    }
    fn f32_bits(&mut self) -> Result<u32, crate::diag::Diagnostic> {
        let t = self.next()?;
        parse_f32_with_mode(t, self.line, self.mode)
    }
    fn i64(&mut self) -> Result<i64, crate::diag::Diagnostic> {
        let t = self.next()?;
        super::decimal_i64(t)
            .ok_or_else(|| syntax(self.line, alloc::format!("expected i64, got {t:?}")))
    }
}

/// Parses an `F32` literal token: `0x` plus one to eight hex digits, or an
/// exactly representable decimal (the vector-file rule, design D4/D5).
///
/// # Errors
/// `E-LITERAL-NONFINITE`, `E-LITERAL-INEXACT` or a syntax error.
pub fn parse_f32(t: &str, line: usize) -> Result<u32, crate::diag::Diagnostic> {
    parse_f32_with_mode(t, line, super::LiteralMode::Exact)
}

fn parse_f32_with_mode(
    t: &str,
    line: usize,
    mode: super::LiteralMode,
) -> Result<u32, crate::diag::Diagnostic> {
    super::value::f32_token_with_mode(t, mode).map_err(|r| match r {
        super::value::F32Refusal::Malformed => {
            syntax(line, alloc::format!("bad f32 literal {t:?}"))
        }
        super::value::F32Refusal::Nonfinite => crate::diag::Diagnostic::new(
            crate::diag::Code::LiteralNonfinite,
            crate::diag::Site::Line(line),
            alloc::format!("non-finite f32 literal {t}"),
        ),
        super::value::F32Refusal::Inexact => crate::diag::Diagnostic::new(
            crate::diag::Code::LiteralInexact,
            crate::diag::Site::Line(line),
            alloc::format!("{t} is not exactly representable in binary32"),
        ),
        super::value::F32Refusal::ModeRequired => crate::diag::Diagnostic::new(
            crate::diag::Code::Profile,
            crate::diag::Site::Line(line),
            "rne: literals require literal-modes noble-graph-literal-modes/v2",
        ),
        super::value::F32Refusal::Overflow => crate::diag::Diagnostic::new(
            crate::diag::Code::LiteralOverflow,
            crate::diag::Site::Line(line),
            alloc::format!("{t} rounds beyond finite binary32 range"),
        ),
        super::value::F32Refusal::TooLong => crate::diag::Diagnostic::new(
            crate::diag::Code::Budget(crate::diag::BudgetKind::LiteralBytes),
            crate::diag::Site::Line(line),
            alloc::format!(
                "{t} exceeds the {} byte RNE literal limit",
                crate::sem::MAX_RNE_DECIMAL_BYTES
            ),
        ),
    })
}

fn rule(t: &str, line: usize) -> Result<crate::ir::OutOfRange, crate::diag::Diagnostic> {
    match t {
        "refuse" => Ok(crate::ir::OutOfRange::Refuse),
        "clamp" => Ok(crate::ir::OutOfRange::Clamp),
        "zero" => Ok(crate::ir::OutOfRange::Zero),
        _ => Err(syntax(line, alloc::format!("bad out-of-range rule {t:?}"))),
    }
}

/// Parses a scalar type name (`i64`, `bool`, `f32`).
#[must_use]
pub fn scalar_type(t: &str) -> Option<crate::ir::ScalarType> {
    match t {
        "i64" => Some(crate::ir::ScalarType::I64),
        "bool" => Some(crate::ir::ScalarType::Bool),
        "f32" => Some(crate::ir::ScalarType::F32),
        _ => None,
    }
}

fn ty(t: &str, line: usize) -> Result<crate::ir::ScalarType, crate::diag::Diagnostic> {
    scalar_type(t).ok_or_else(|| syntax(line, alloc::format!("bad type {t:?}")))
}

/// A range-fact literal: `0x…` binary32, `true|false`, `i64:N`, or an
/// unprefixed decimal of the sub-profile's type.
fn range_literal(
    t: &mut Toks<'_>,
    profile: crate::ir::SubProfile,
) -> Result<crate::ir::Literal, crate::diag::Diagnostic> {
    let tok = t.next()?;
    let line = t.line;
    if tok.starts_with("0x") {
        return Ok(crate::ir::Literal::F32(parse_f32_with_mode(
            tok, line, t.mode,
        )?));
    }
    match tok {
        "true" => return Ok(crate::ir::Literal::Bool(true)),
        "false" => return Ok(crate::ir::Literal::Bool(false)),
        _ => {}
    }
    if let Some(v) = tok.strip_prefix("i64:") {
        return super::decimal_i64(v)
            .map(crate::ir::Literal::I64)
            .ok_or_else(|| syntax(line, alloc::format!("expected i64, got {tok:?}")));
    }
    match profile {
        crate::ir::SubProfile::F32 => Ok(crate::ir::Literal::F32(parse_f32_with_mode(
            tok, line, t.mode,
        )?)),
        crate::ir::SubProfile::I64 => super::decimal_i64(tok)
            .map(crate::ir::Literal::I64)
            .ok_or_else(|| syntax(line, alloc::format!("expected i64, got {tok:?}"))),
    }
}

#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; the one-hot group is bounded by the remaining fact tokens"
)]
fn fact(
    t: &mut Toks<'_>,
    profile: crate::ir::SubProfile,
) -> Result<crate::ir::Fact, crate::diag::Diagnostic> {
    let kind = t.next()?;
    Ok(match kind {
        "range" => crate::ir::Fact::Range {
            lo: range_literal(t, profile)?,
            hi: range_literal(t, profile)?,
        },
        "grid" => crate::ir::Fact::Grid {
            log2: i32::try_from(t.i64()?).map_err(|_| syntax(t.line, "grid exponent"))?,
        },
        "bit" => crate::ir::Fact::Bit,
        "finite" => crate::ir::Fact::Finite,
        "integral" => crate::ir::Fact::Integral,
        "shape" => crate::ir::Fact::Shape(t.u32()?),
        "one-hot" => {
            let is_zero_allowed = match t.next()? {
                "allow-zero" => true,
                "exact" => false,
                o => return Err(syntax(t.line, alloc::format!("bad one-hot mode {o:?}"))),
            };
            let mut group = alloc::vec::Vec::new();
            while t.peek().is_some_and(|p| p.starts_with('%')) {
                group.push(t.node()?);
            }
            crate::ir::Fact::OneHot {
                group,
                allow_zero: is_zero_allowed,
            }
        }
        o => return Err(syntax(t.line, alloc::format!("unknown fact kind {o:?}"))),
    })
}

fn subject_kind(t: &str) -> Option<crate::ir::SubjectKind> {
    match t {
        "authored" => Some(crate::ir::SubjectKind::Authored),
        "authored-nickel" => Some(crate::ir::SubjectKind::AuthoredNickel),
        "extracted" => Some(crate::ir::SubjectKind::Extracted),
        "captured" => Some(crate::ir::SubjectKind::Captured),
        _ => None,
    }
}

/// Parses `<64 hex> closure <ids|->` after the subject kind.
fn subject_record(
    t: &mut Toks<'_>,
    kind: crate::ir::SubjectKind,
) -> Result<crate::ir::Subject, crate::diag::Diagnostic> {
    let line = t.line;
    let identity = super::digest_from_hex(t.next()?)
        .ok_or_else(|| syntax(line, "subject identity must be 64 hex digits"))?;
    if t.next()? != "closure" {
        return Err(syntax(line, "expected `closure` in the subject line"));
    }
    let ids = t.next()?;
    let closure = if ids == "-" {
        alloc::vec::Vec::new()
    } else {
        ids.split(',')
            .map(|d| {
                super::decimal_u64(d)
                    .ok_or_else(|| syntax(line, alloc::format!("bad closure id {d:?}")))
            })
            .collect::<Result<alloc::vec::Vec<u64>, crate::diag::Diagnostic>>()?
    };
    Ok(crate::ir::Subject {
        kind,
        identity,
        closure,
    })
}

/// Parses the shape trees of a `shape input|output` line. A tree that would
/// nest more than [`MAX_SHAPE_DEPTH`] pairs is refused before the parser
/// descends any deeper (design D4).
#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; the output is bounded by the supplied shape tokens"
)]
fn shape_trees(
    tokens: &[&str],
    line: usize,
) -> Result<alloc::vec::Vec<crate::ir::ShapeTree>, crate::diag::Diagnostic> {
    let joined = tokens.join(" ").replace('(', " ( ").replace(')', " ) ");
    let toks: alloc::vec::Vec<&str> = joined.split_whitespace().collect();
    /// One tree inside `open` enclosing pairs.
    #[expect(
        tigerstyle::no_recursion,
        reason = "Owner: noble-maintainers; the parser is depth-bounded by MAX_SHAPE_DEPTH"
    )]
    fn tree(
        toks: &[&str],
        at: &mut usize,
        line: usize,
        open: u32,
    ) -> Result<crate::ir::ShapeTree, crate::diag::Diagnostic> {
        let t = toks
            .get(*at)
            .copied()
            .ok_or_else(|| syntax(line, "unterminated shape tree"))?;
        *at += 1;
        match t {
            "s" => Ok(crate::ir::ShapeTree::SCALAR),
            "u" => Ok(crate::ir::ShapeTree::UNIT),
            "(" => {
                if open >= crate::ir::MAX_SHAPE_DEPTH {
                    return Err(crate::ir::shape_depth_refusal(crate::diag::Site::Line(
                        line,
                    )));
                }
                if toks.get(*at).copied() != Some("p") {
                    return Err(syntax(line, "expected `p` after `(` in a shape tree"));
                }
                *at += 1;
                let a = tree(toks, at, line, open + 1)?;
                let b = tree(toks, at, line, open + 1)?;
                if toks.get(*at).copied() != Some(")") {
                    return Err(syntax(line, "expected `)` in a shape tree"));
                }
                *at += 1;
                crate::ir::ShapeTree::pair(a, b)
                    .map_err(|_| crate::ir::shape_depth_refusal(crate::diag::Site::Line(line)))
            }
            o => Err(syntax(line, alloc::format!("bad shape tree token {o:?}"))),
        }
    }
    let mut at = 0;
    let mut out = alloc::vec::Vec::new();
    while at < toks.len() {
        out.push(tree(&toks, &mut at, line, 0)?);
    }
    Ok(out)
}

/// Percent-decodes a `derived` rule token (`""` is the empty rule).
fn decode_rule(tok: &str, line: usize) -> Result<alloc::string::String, crate::diag::Diagnostic> {
    if tok == "\"\"" {
        return Ok(alloc::string::String::new());
    }
    let b = tok.as_bytes();
    let mut out = alloc::vec::Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let v = b
                .get(i + 1..i + 3)
                .and_then(|h| core::str::from_utf8(h).ok())
                .filter(|h| h.bytes().all(|c| c.is_ascii_hexdigit()))
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| syntax(line, alloc::format!("bad escape in rule {tok:?}")))?;
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    alloc::string::String::from_utf8(out)
        .map_err(|_| syntax(line, alloc::format!("rule {tok:?} is not UTF-8")))
}

/// Percent-encodes a `derived` rule so that it is one printable token.
fn encode_rule(rule: &str) -> alloc::string::String {
    if rule.is_empty() {
        return "\"\"".to_owned();
    }
    let mut s = alloc::string::String::with_capacity(rule.len());
    for &b in rule.as_bytes() {
        if b.is_ascii_graphic() && !matches!(b, b'%' | b'#' | b'"') {
            s.push(char::from(b));
        } else {
            crate::format::write_text!(s, "%{b:02X}");
        }
    }
    s
}

fn provenance(t: &mut Toks<'_>) -> Result<crate::ir::Provenance, crate::diag::Diagnostic> {
    let line = t.line;
    Ok(match t.next()? {
        "input-validated" => crate::ir::Provenance::InputValidated,
        "asserted" => crate::ir::Provenance::Asserted,
        "derived" => crate::ir::Provenance::Derived(decode_rule(t.next()?, line)?),
        "guard" => crate::ir::Provenance::Guard(t.node()?),
        "evidence" => {
            let w = t.next()?;
            crate::ir::Provenance::Evidence(if w == "-" {
                alloc::vec::Vec::new()
            } else {
                super::bytes_from_hex(w).ok_or_else(|| {
                    syntax(line, alloc::format!("evidence witness {w:?} is not hex"))
                })?
            })
        }
        o => return Err(syntax(line, alloc::format!("bad provenance {o:?}"))),
    })
}

#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; affine terms and index tables are bounded by the supplied operation tokens"
)]
fn op(
    t: &mut Toks<'_>,
    profile: crate::ir::SubProfile,
) -> Result<crate::ir::Op, crate::diag::Diagnostic> {
    let line = t.line;
    let name = t.next()?;
    Ok(match name {
        "input" => crate::ir::Op::Input(t.u32()?),
        "param" => crate::ir::Op::Param(t.u32()?),
        "const" => {
            let ty = ty(t.next()?, line)?;
            crate::ir::Op::Const(match ty {
                crate::ir::ScalarType::F32 => crate::ir::Literal::F32(t.f32_bits()?),
                crate::ir::ScalarType::I64 => crate::ir::Literal::I64(t.i64()?),
                crate::ir::ScalarType::Bool => crate::ir::Literal::Bool(match t.next()? {
                    "true" => true,
                    "false" => false,
                    o => return Err(syntax(line, alloc::format!("bad bool {o:?}"))),
                }),
            })
        }
        "i64.add" => crate::ir::Op::I64Add(t.node()?, t.node()?),
        "i64.sub" => crate::ir::Op::I64Sub(t.node()?, t.node()?),
        "i64.mul" => crate::ir::Op::I64Mul(t.node()?, t.node()?),
        "i64.eq" => crate::ir::Op::I64Eq(t.node()?, t.node()?),
        "f32.add" => crate::ir::Op::F32Add(t.node()?, t.node()?),
        "f32.sub" => crate::ir::Op::F32Sub(t.node()?, t.node()?),
        "f32.mul" => crate::ir::Op::F32Mul(t.node()?, t.node()?),
        "f32.affine" => {
            let bias = t.f32_bits()?;
            let mut terms = alloc::vec::Vec::new();
            while t.peek().is_some() {
                let coefficient = t.f32_bits()?;
                let operand = if t.peek() == Some("one") {
                    t.next()?;
                    crate::ir::AffineOperand::One
                } else {
                    crate::ir::AffineOperand::Node(t.node()?)
                };
                terms.push(crate::ir::AffineTerm {
                    coefficient,
                    operand,
                });
            }
            crate::ir::Op::F32Affine { bias, terms }
        }
        "f32.relu" => crate::ir::Op::F32Relu(t.node()?),
        "f32.reglu" => crate::ir::Op::F32Reglu {
            left: t.node()?,
            gate: t.node()?,
        },
        "f32.step" => crate::ir::Op::F32Step(t.node()?),
        "f32.select" => crate::ir::Op::F32Select {
            bit: t.node()?,
            off: t.node()?,
            on: t.node()?,
        },
        "f32.index" => {
            let index = t.node()?;
            let below = rule(t.next()?, line)?;
            let above = rule(t.next()?, line)?;
            let mut table = alloc::vec::Vec::new();
            while t.peek().is_some() {
                table.push(t.node()?);
            }
            crate::ir::Op::F32Index {
                index,
                table,
                below,
                above,
            }
        }
        "guard" => {
            let value = t.node()?;
            let fact = fact(t, profile)?;
            crate::ir::Op::Guard { fact, value }
        }
        other => {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::UnknownOp,
                crate::diag::Site::Line(line),
                alloc::format!("operation {other:?} is not in the ngraph-ir/0 whitelist"),
            ))
        }
    })
}

/// Charges the collected-token floor of every significant line against
/// `graph_bytes` before any token vector is built.
///
/// Each token occupies one `&str` entry (16 bytes) in its line's vector, so a
/// 64 MiB file of one-token lines would otherwise collect tens of millions of
/// entries before the retained-bytes budget applied. A line whose tokens
/// alone would retain more than `graph_bytes` is refused here; the vectors
/// live only while their line is parsed, so a per-line charge bounds the peak.
///
/// # Errors
/// `E-BUDGET-GRAPH-BYTES` with the budget and the floor that crosses it.
fn token_budget(text: &str, graph_bytes: u64) -> Result<(), crate::diag::Diagnostic> {
    let per_token_bytes = core::mem::size_of::<&str>() as u64;
    for line in text.lines() {
        let body = line.split('#').next().unwrap_or("");
        let mut tokens = 0u64;
        for _ in body.split_whitespace() {
            tokens = tokens.saturating_add(1);
        }
        let observed = tokens.saturating_mul(per_token_bytes);
        if observed > graph_bytes {
            return Err(crate::diag::Diagnostic::budget(
                crate::diag::BudgetKind::GraphBytes,
                graph_bytes,
                observed,
            ));
        }
    }
    Ok(())
}

/// Parses a graph file. `ids` is used only for the `subject authored-name`
/// sugar.
///
/// # Errors
/// The first D6 refusal encountered while reading.
pub fn parse(
    ids: &dyn IdentityScheme,
    text: &str,
) -> Result<crate::ir::Graph, crate::diag::Diagnostic> {
    parse_within(ids, text, crate::check::Budgets::default().graph_bytes)
}

/// [`parse`] under a graph byte budget: the collected-token floor of any one
/// significant line must fit, so the token vectors cannot outgrow the budget
/// before the checker charges the parsed graph. The `/0` policy may only lower
/// a budget, so `parse` uses the default.
///
/// # Errors
/// `E-BUDGET-GRAPH-BYTES` for an over-budget line, then [`parse`]'s refusals.
pub fn parse_within(
    ids: &dyn IdentityScheme,
    text: &str,
    graph_bytes: u64,
) -> Result<crate::ir::Graph, crate::diag::Diagnostic> {
    token_budget(text, graph_bytes)?;
    let mut lines = super::lines(text);
    let header = lines.next().ok_or_else(|| syntax(1, "empty graph file"))?;
    let hl = header.number;
    if header.tokens[0] != crate::ir::IR_VERSION {
        return Err(syntax(hl, "missing ngraph-ir/0 schema line"));
    }
    let profile = header
        .tokens
        .get(1)
        .copied()
        .and_then(crate::ir::SubProfile::from_name)
        .ok_or_else(|| {
            crate::diag::Diagnostic::new(
                crate::diag::Code::Profile,
                crate::diag::Site::Line(hl),
                "unknown sub-profile",
            )
        })?;
    if header.tokens.len() != 2 {
        return Err(syntax(hl, "trailing tokens on the schema line"));
    }
    let placeholder = crate::ir::Subject {
        kind: crate::ir::SubjectKind::Authored,
        identity: [0; 32],
        closure: alloc::vec::Vec::new(),
    };
    let mut b = crate::ir::GraphBuilder::new(profile, placeholder);
    let mut subject: Option<crate::ir::Subject> = None;
    let mut input_shape: Option<alloc::vec::Vec<crate::ir::ShapeTree>> = None;
    let mut output_shape: Option<alloc::vec::Vec<crate::ir::ShapeTree>> = None;
    let mut literal_mode = super::LiteralMode::Exact;
    let mut is_literal_mode_declared = false;
    let mut is_record_started = false;
    for l in lines {
        let line = l.number;
        let mut t = Toks {
            t: l.tokens,
            i: 0,
            line,
            mode: literal_mode,
        };
        let head = t.next()?;
        match head {
            "literal-modes" => {
                if is_literal_mode_declared {
                    return Err(syntax(line, "duplicate literal-modes line"));
                }
                if is_record_started {
                    return Err(crate::diag::Diagnostic::new(
                        crate::diag::Code::Profile,
                        crate::diag::Site::Line(line),
                        "literal-modes must immediately follow the schema line",
                    ));
                }
                let name = t.next()?;
                literal_mode = super::LiteralMode::from_name(name).ok_or_else(|| {
                    crate::diag::Diagnostic::new(
                        crate::diag::Code::Profile,
                        crate::diag::Site::Line(line),
                        alloc::format!("unknown graph literal mode version {name:?}"),
                    )
                })?;
                t.mode = literal_mode;
                is_literal_mode_declared = true;
            }
            "subject" => {
                if subject.is_some() {
                    return Err(syntax(line, "duplicate subject line"));
                }
                let kind = t.next()?;
                subject = Some(if kind == "authored-name" {
                    let name = t.rest().join(" ");
                    if name.is_empty() {
                        return Err(syntax(line, "authored-name needs a name"));
                    }
                    crate::ir::Subject::authored(ids, &name)
                } else {
                    let k = subject_kind(kind)
                        .ok_or_else(|| syntax(line, alloc::format!("bad subject kind {kind:?}")))?;
                    subject_record(&mut t, k)?
                });
            }
            "shape" => {
                let which = t.next()?;
                let trees = shape_trees(t.rest(), line)?;
                let slot = match which {
                    "input" => &mut input_shape,
                    "output" => &mut output_shape,
                    o => return Err(syntax(line, alloc::format!("bad shape side {o:?}"))),
                };
                if slot.is_some() {
                    return Err(syntax(line, alloc::format!("duplicate shape {which} line")));
                }
                *slot = Some(trees);
            }
            "slot" => {
                let kind = match t.next()? {
                    "input" => crate::ir::SlotKind::Input,
                    "param" => crate::ir::SlotKind::Param,
                    o => return Err(syntax(line, alloc::format!("bad slot kind {o:?}"))),
                };
                let ty = ty(t.next()?, line)?;
                b.declare_slot(kind, ty);
            }
            "out" => {
                let n = t.node()?;
                b.output(n);
            }
            "fact" => {
                let n = t.node()?;
                let f = fact(&mut t, profile)?;
                let p = provenance(&mut t)?;
                b.fact(n, f, p);
            }
            h if h.starts_with('%') => {
                let id = super::decimal_u64(&h[1..])
                    .and_then(|v| usize::try_from(v).ok())
                    .ok_or_else(|| syntax(line, "bad node id"))?;
                if id != b.len() {
                    return Err(syntax(
                        line,
                        alloc::format!("node %{id} out of sequence (next is %{})", b.len()),
                    ));
                }
                if t.next()? != "=" {
                    return Err(syntax(line, "expected ="));
                }
                let o = op(&mut t, profile)?;
                b.op(o);
            }
            other => return Err(syntax(line, alloc::format!("unknown line kind {other:?}"))),
        }
        t.done()?;
        if head != "literal-modes" {
            is_record_started = true;
        }
    }
    let mut g = b.finish();
    g.subject = subject.ok_or_else(|| syntax(hl, "missing subject line"))?;
    g.input_shape = input_shape;
    g.output_shape = output_shape;
    Ok(g)
}

fn lit(l: crate::ir::Literal, profile: crate::ir::SubProfile) -> alloc::string::String {
    match (l, profile) {
        (crate::ir::Literal::I64(v), crate::ir::SubProfile::I64) => v.to_string(),
        (crate::ir::Literal::I64(v), crate::ir::SubProfile::F32) => alloc::format!("i64:{v}"),
        (crate::ir::Literal::Bool(v), _) => v.to_string(),
        (crate::ir::Literal::F32(b), _) => alloc::format!("0x{b:08x}"),
    }
}

fn const_lit(l: crate::ir::Literal) -> alloc::string::String {
    match l {
        crate::ir::Literal::I64(v) => v.to_string(),
        crate::ir::Literal::Bool(v) => v.to_string(),
        crate::ir::Literal::F32(b) => alloc::format!("0x{b:08x}"),
    }
}

fn fact_text(f: &crate::ir::Fact, profile: crate::ir::SubProfile) -> alloc::string::String {
    match f {
        crate::ir::Fact::Range { lo, hi } => {
            alloc::format!("range {} {}", lit(*lo, profile), lit(*hi, profile))
        }
        crate::ir::Fact::Grid { log2 } => alloc::format!("grid {log2}"),
        crate::ir::Fact::Bit => "bit".into(),
        crate::ir::Fact::Finite => "finite".into(),
        crate::ir::Fact::Integral => "integral".into(),
        crate::ir::Fact::Shape(n) => alloc::format!("shape {n}"),
        crate::ir::Fact::OneHot { group, allow_zero } => {
            let mut s = alloc::format!(
                "one-hot {}",
                if *allow_zero { "allow-zero" } else { "exact" }
            );
            for g in group {
                crate::format::write_text!(s, " %{}", g.0);
            }
            s
        }
    }
}

fn shape_text(trees: &[crate::ir::ShapeTree]) -> alloc::string::String {
    // Recursion depth is bounded by `crate::ir::MAX_SHAPE_DEPTH`.
    #[expect(
        tigerstyle::borrowed_argument_types,
        reason = "Owner: noble-maintainers; canonical shape text grows the owned output String and cannot use a borrowed slice"
    )]
    #[expect(
        tigerstyle::no_recursion,
        reason = "Owner: noble-maintainers; the writer is bounded by the declared shape nesting limit"
    )]
    fn tree(t: &crate::ir::ShapeTree, s: &mut alloc::string::String) {
        match t.node() {
            crate::ir::ShapeNode::Scalar => s.push('s'),
            crate::ir::ShapeNode::Unit => s.push('u'),
            crate::ir::ShapeNode::Pair(a, b) => {
                s.push_str("(p ");
                tree(a, s);
                s.push(' ');
                tree(b, s);
                s.push(')');
            }
        }
    }
    let mut s = alloc::string::String::new();
    for t in trees {
        s.push(' ');
        tree(t, &mut s);
    }
    s
}

/// The subject line body after `subject `: `<kind> <64 hex> closure <ids|->`.
#[must_use]
pub fn subject_text(s: &crate::ir::Subject) -> alloc::string::String {
    let closure = if s.closure.is_empty() {
        "-".to_owned()
    } else {
        s.closure
            .iter()
            .map(u64::to_string)
            .collect::<alloc::vec::Vec<_>>()
            .join(",")
    };
    alloc::format!(
        "{} {} closure {closure}",
        s.kind.name(),
        crate::ir::hex(&s.identity)
    )
}

/// Prints a graph in the canonical file form (`F32` literals as exact bit
/// patterns; origins omitted).
#[must_use]
pub fn print(g: &crate::ir::Graph) -> alloc::string::String {
    let mut s = alloc::format!("{} {}\n", crate::ir::IR_VERSION, g.profile.name());
    crate::format::write_line!(s, "subject {}", subject_text(&g.subject));
    if let Some(t) = &g.input_shape {
        crate::format::write_line!(s, "shape input{}", shape_text(t));
    }
    if let Some(t) = &g.output_shape {
        crate::format::write_line!(s, "shape output{}", shape_text(t));
    }
    for slot in &g.slots {
        let k = match slot.kind {
            crate::ir::SlotKind::Input => "input",
            crate::ir::SlotKind::Param => "param",
        };
        crate::format::write_line!(s, "slot {k} {}", slot.ty.name());
    }
    let n = |x: &crate::ir::NodeId| alloc::format!("%{}", x.0);
    for (i, node) in g.nodes.iter().enumerate() {
        let body = match &node.op {
            crate::ir::Op::Input(k) | crate::ir::Op::Param(k) => {
                alloc::format!("{} {k}", node.op.name())
            }
            crate::ir::Op::Const(l) => alloc::format!("const {} {}", l.ty().name(), const_lit(*l)),
            crate::ir::Op::I64Add(a, b)
            | crate::ir::Op::I64Sub(a, b)
            | crate::ir::Op::I64Mul(a, b)
            | crate::ir::Op::I64Eq(a, b)
            | crate::ir::Op::F32Add(a, b)
            | crate::ir::Op::F32Sub(a, b)
            | crate::ir::Op::F32Mul(a, b) => alloc::format!("{} {} {}", node.op.name(), n(a), n(b)),
            crate::ir::Op::F32Affine { bias, terms } => {
                let mut s = alloc::format!("f32.affine 0x{bias:08x}");
                for t in terms {
                    let o = match t.operand {
                        crate::ir::AffineOperand::One => "one".to_owned(),
                        crate::ir::AffineOperand::Node(x) => n(&x),
                    };
                    crate::format::write_text!(s, " 0x{:08x} {o}", t.coefficient);
                }
                s
            }
            crate::ir::Op::F32Relu(x) | crate::ir::Op::F32Step(x) => {
                alloc::format!("{} {}", node.op.name(), n(x))
            }
            crate::ir::Op::F32Reglu { left, gate } => {
                alloc::format!("f32.reglu {} {}", n(left), n(gate))
            }
            crate::ir::Op::F32Select { bit, off, on } => {
                alloc::format!("f32.select {} {} {}", n(bit), n(off), n(on))
            }
            crate::ir::Op::F32Index {
                index,
                table,
                below,
                above,
            } => {
                let mut s =
                    alloc::format!("f32.index {} {} {}", n(index), below.name(), above.name());
                for t in table {
                    crate::format::write_text!(s, " {}", n(t));
                }
                s
            }
            crate::ir::Op::Guard { fact, value } => {
                alloc::format!("guard {} {}", n(value), fact_text(fact, g.profile))
            }
        };
        crate::format::write_line!(s, "%{i} = {body}");
    }
    for f in &g.facts {
        let p = match &f.provenance {
            crate::ir::Provenance::InputValidated => "input-validated".to_owned(),
            crate::ir::Provenance::Asserted => "asserted".to_owned(),
            crate::ir::Provenance::Derived(r) => alloc::format!("derived {}", encode_rule(r)),
            crate::ir::Provenance::Guard(gd) => alloc::format!("guard {}", n(gd)),
            crate::ir::Provenance::Evidence(w) if w.is_empty() => "evidence -".to_owned(),
            crate::ir::Provenance::Evidence(w) => alloc::format!("evidence {}", crate::ir::hex(w)),
        };
        crate::format::write_line!(
            s,
            "fact {} {} {p}",
            n(&f.node),
            fact_text(&f.fact, g.profile)
        );
    }
    for o in &g.outputs {
        crate::format::write_line!(s, "out {}", n(o));
    }
    s
}
