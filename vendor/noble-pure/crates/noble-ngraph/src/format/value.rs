#![expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the module path names the domain and the item name keeps its canonical operation spelling"
)]

//! Value tokens of vector and data files (design D5) and the report
//! rendering of observed values (design D7).

use alloc::borrow::ToOwned;
use alloc::string::ToString;

fn format_error(line: usize, why: impl Into<alloc::string::String>) -> crate::diag::Diagnostic {
    crate::diag::Diagnostic::new(
        crate::diag::Code::Format,
        crate::diag::Site::Line(line),
        why,
    )
}

/// Why an `f32` token was refused; each file format maps `Malformed` to its
/// own syntax code.
pub(crate) enum F32Refusal {
    /// Neither `0x` bits nor a decimal.
    Malformed,
    /// A `nan`/`inf` word (`E-LITERAL-NONFINITE`).
    Nonfinite,
    /// A decimal that no binary32 value equals (`E-LITERAL-INEXACT`).
    Inexact,
    /// An explicit RNE marker without the versioned v2 file declaration.
    ModeRequired,
    /// The decimal rounds to infinity (`E-LITERAL-OVERFLOW`).
    Overflow,
    /// The explicit decimal exceeded its bounded token length.
    TooLong,
}

/// Reads an `f32` token under the declared literal policy.
pub(crate) fn f32_token_with_mode(tok: &str, mode: super::LiteralMode) -> Result<u32, F32Refusal> {
    if let Some(decimal) = tok.strip_prefix("rne:") {
        if mode != super::LiteralMode::RneV2 {
            return Err(F32Refusal::ModeRequired);
        }
        return crate::sem::rne_f32_decimal(decimal).map_err(|refusal| match refusal {
            crate::sem::DecimalRefusal::Malformed => F32Refusal::Malformed,
            crate::sem::DecimalRefusal::Overflow => F32Refusal::Overflow,
            crate::sem::DecimalRefusal::TooLong => F32Refusal::TooLong,
        });
    }
    if tok.starts_with("0x") {
        return super::hex_bits(tok, 8)
            .and_then(|b| u32::try_from(b).ok())
            .ok_or(F32Refusal::Malformed);
    }
    // Only the non-finite *words* of the decimal grammar are
    // `E-LITERAL-NONFINITE` (design D5: NaN and infinities are given as bits).
    // A leading `+` is a malformed number token (NGRAPH-FORMAT-01), and a
    // longer token such as `banana` or `0.5inf` is neither a word nor a
    // decimal, so both are `Malformed`.
    let word = tok.strip_prefix('-').unwrap_or(tok);
    if ["inf", "infinity", "nan"]
        .iter()
        .any(|w| word.eq_ignore_ascii_case(w))
    {
        return Err(F32Refusal::Nonfinite);
    }
    if !crate::sem::is_decimal(tok) {
        return Err(F32Refusal::Malformed);
    }
    crate::sem::exact_f32_decimal(tok).ok_or(F32Refusal::Inexact)
}

/// Parses one value token of type `ty` on file line `line`.
///
/// - `f32`: `0x` plus one to eight hex digits (any encoding, including NaN,
///   infinities and subnormals) or an exactly representable finite decimal;
/// - `i64`: a signed decimal within range;
/// - `bool`: `true` or `false`.
///
/// # Errors
/// `E-LITERAL-INEXACT` for an inexact decimal, `E-LITERAL-NONFINITE` for a
/// `nan`/`inf` word, `E-FORMAT` otherwise.
pub fn parse(
    ty: crate::ir::ScalarType,
    tok: &str,
    line: usize,
) -> Result<crate::sem::Value, crate::diag::Diagnostic> {
    parse_with_mode(ty, tok, line, super::LiteralMode::Exact)
}

/// Parses one value token under the file's selected literal-mode version.
pub fn parse_with_mode(
    ty: crate::ir::ScalarType,
    tok: &str,
    line: usize,
    mode: super::LiteralMode,
) -> Result<crate::sem::Value, crate::diag::Diagnostic> {
    match ty {
        crate::ir::ScalarType::F32 => f32_token_with_mode(tok, mode)
            .map(crate::sem::Value::F32)
            .map_err(|r| match r {
                F32Refusal::Malformed => {
                    format_error(line, alloc::format!("bad f32 value {tok:?}"))
                }
                F32Refusal::Nonfinite => crate::diag::Diagnostic::new(
                    crate::diag::Code::LiteralNonfinite,
                    crate::diag::Site::Line(line),
                    alloc::format!(
                        "non-finite f32 value {tok}; give NaN and infinities as 0x bits"
                    ),
                ),
                F32Refusal::Inexact => crate::diag::Diagnostic::new(
                    crate::diag::Code::LiteralInexact,
                    crate::diag::Site::Line(line),
                    alloc::format!("{tok} is not exactly representable in binary32"),
                ),
                F32Refusal::ModeRequired => crate::diag::Diagnostic::new(
                    crate::diag::Code::Profile,
                    crate::diag::Site::Line(line),
                    "rne: literals require literal-modes noble-graph-literal-modes/v2",
                ),
                F32Refusal::Overflow => crate::diag::Diagnostic::new(
                    crate::diag::Code::LiteralOverflow,
                    crate::diag::Site::Line(line),
                    alloc::format!("{tok} rounds beyond finite binary32 range"),
                ),
                F32Refusal::TooLong => crate::diag::Diagnostic::new(
                    crate::diag::Code::Budget(crate::diag::BudgetKind::LiteralBytes),
                    crate::diag::Site::Line(line),
                    alloc::format!(
                        "{tok} exceeds the {} byte RNE literal limit",
                        crate::sem::MAX_RNE_DECIMAL_BYTES
                    ),
                ),
            }),
        crate::ir::ScalarType::I64 => super::decimal_i64(tok)
            .map(crate::sem::Value::I64)
            .ok_or_else(|| format_error(line, alloc::format!("bad i64 value {tok:?}"))),
        crate::ir::ScalarType::Bool => match tok {
            "true" => Ok(crate::sem::Value::Bool(true)),
            "false" => Ok(crate::sem::Value::Bool(false)),
            _ => Err(format_error(line, alloc::format!("bad bool value {tok:?}"))),
        },
    }
}

/// The canonical file token of a value: `f32` as its exact `0x%08x`
/// encoding, `i64` in decimal, `bool` as `true`/`false`.
#[must_use]
pub fn token(v: crate::sem::Value) -> alloc::string::String {
    match v {
        crate::sem::Value::F32(b) => alloc::format!("0x{b:08x}"),
        crate::sem::Value::I64(x) => x.to_string(),
        crate::sem::Value::Bool(x) => x.to_string(),
    }
}

/// The report rendering of an observed value: `f32:0x%08x`, with every NaN
/// encoding rendered as the single class `f32:nan` (D3 `≡obs`); `i64:N`;
/// `bool:true|false`.
#[must_use]
pub fn render(v: crate::sem::Value) -> alloc::string::String {
    match v {
        crate::sem::Value::F32(b) if f32::from_bits(b).is_nan() => "f32:nan".to_owned(),
        crate::sem::Value::F32(b) => alloc::format!("f32:0x{b:08x}"),
        crate::sem::Value::I64(x) => alloc::format!("i64:{x}"),
        crate::sem::Value::Bool(x) => alloc::format!("bool:{x}"),
    }
}

/// Parses the type tokens of a `types` line.
pub(crate) fn types(
    tokens: &[&str],
    line: usize,
) -> Result<alloc::vec::Vec<crate::ir::ScalarType>, crate::diag::Diagnostic> {
    tokens
        .iter()
        .map(|t| {
            super::graph::scalar_type(t)
                .ok_or_else(|| format_error(line, alloc::format!("bad type {t:?}")))
        })
        .collect()
}

pub(crate) fn row_with_mode(
    types: &[crate::ir::ScalarType],
    tokens: &[&str],
    line: usize,
    mode: super::LiteralMode,
) -> Result<alloc::vec::Vec<crate::sem::Value>, crate::diag::Diagnostic> {
    if tokens.len() != types.len() {
        return Err(format_error(
            line,
            alloc::format!("{} values for {} types", tokens.len(), types.len()),
        ));
    }
    types
        .iter()
        .zip(tokens)
        .map(|(ty, tok)| parse_with_mode(*ty, tok, line, mode))
        .collect()
}

pub(crate) fn literal_mode(
    tokens: &[&str],
    line: usize,
) -> Result<super::LiteralMode, crate::diag::Diagnostic> {
    if tokens.len() != 2 || tokens[0] != "literal-modes" {
        return Err(format_error(line, "expected literal-modes VERSION"));
    }
    super::LiteralMode::from_name(tokens[1]).ok_or_else(|| {
        crate::diag::Diagnostic::new(
            crate::diag::Code::Profile,
            crate::diag::Site::Line(line),
            alloc::format!("unknown graph literal mode version {:?}", tokens[1]),
        )
    })
}

pub(crate) fn type_names(types: &[crate::ir::ScalarType]) -> alloc::string::String {
    types
        .iter()
        .map(|t| t.name())
        .collect::<alloc::vec::Vec<_>>()
        .join(" ")
}

pub(crate) fn row_text(values: &[crate::sem::Value]) -> alloc::string::String {
    values
        .iter()
        .map(|v| token(*v))
        .collect::<alloc::vec::Vec<_>>()
        .join(" ")
}

pub(crate) fn format_diag(
    line: usize,
    why: impl Into<alloc::string::String>,
) -> crate::diag::Diagnostic {
    format_error(line, why)
}
