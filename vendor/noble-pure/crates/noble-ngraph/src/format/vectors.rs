#![expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the module path names the domain and the item name keeps its canonical operation spelling"
)]

//! Evaluation vector files `noble-ngraph-vectors/0` (design D5).
//!
//! ```text
//! noble-ngraph-vectors/0
//! types f32 f32
//! v 0x3fc00000 -0.5
//! v 0x7fc00000 0x80000000
//! ```
//!
//! `types` lists the graph's ordered input slot types (parameters never come
//! from vector files). The caller compares it with the graph (`E-INTERFACE`).

/// Schema line.
pub const SCHEMA: &str = "noble-ngraph-vectors/0";

/// A parsed vector file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vectors {
    /// Declared input types.
    pub types: alloc::vec::Vec<crate::ir::ScalarType>,
    /// Rows in file order.
    pub rows: alloc::vec::Vec<alloc::vec::Vec<crate::sem::Value>>,
}

/// Parses a vector file.
///
/// # Errors
/// `E-FORMAT` (schema, syntax, missing or duplicate `types`, row count),
/// `E-LITERAL-INEXACT` or `E-LITERAL-NONFINITE`.
#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; rows are bounded by the significant line count of the supplied text"
)]
pub fn parse(text: &str) -> Result<Vectors, crate::diag::Diagnostic> {
    let mut lines = super::lines(text);
    let header = lines.next().ok_or_else(|| {
        super::value::format_diag(1, "missing noble-ngraph-vectors/0 schema line")
    })?;
    if header.tokens != [SCHEMA] {
        return Err(super::value::format_diag(
            header.number,
            "missing noble-ngraph-vectors/0 schema line",
        ));
    }
    let mut mode = super::LiteralMode::Exact;
    let mut is_mode_declared = false;
    let mut is_record_started = false;
    let mut declared: Option<alloc::vec::Vec<crate::ir::ScalarType>> = None;
    let mut rows = alloc::vec::Vec::new();
    for l in lines {
        match l.tokens[0] {
            "literal-modes" => {
                if is_mode_declared {
                    return Err(super::value::format_diag(
                        l.number,
                        "duplicate literal-modes line",
                    ));
                }
                if is_record_started {
                    return Err(super::value::format_diag(
                        l.number,
                        "literal-modes must immediately follow the schema line",
                    ));
                }
                mode = super::value::literal_mode(&l.tokens, l.number)?;
                is_mode_declared = true;
            }
            "types" => {
                if declared.is_some() {
                    return Err(super::value::format_diag(l.number, "duplicate types line"));
                }
                declared = Some(super::value::types(&l.tokens[1..], l.number)?);
            }
            "v" => {
                let t = declared.as_ref().ok_or_else(|| {
                    super::value::format_diag(l.number, "a v row before the types line")
                })?;
                rows.push(super::value::row_with_mode(
                    t,
                    &l.tokens[1..],
                    l.number,
                    mode,
                )?);
            }
            o => {
                return Err(super::value::format_diag(
                    l.number,
                    alloc::format!("unknown line kind {o:?}"),
                ))
            }
        }
        if l.tokens[0] != "literal-modes" {
            is_record_started = true;
        }
    }
    let types =
        declared.ok_or_else(|| super::value::format_diag(header.number, "missing types line"))?;
    Ok(Vectors { types, rows })
}

/// Prints a vector file in canonical form.
#[must_use]
pub fn print(v: &Vectors) -> alloc::string::String {
    let mut s = alloc::format!("{SCHEMA}\ntypes {}\n", super::value::type_names(&v.types));
    for r in &v.rows {
        s.push_str("v ");
        s.push_str(&super::value::row_text(r));
        s.push('\n');
    }
    s
}
