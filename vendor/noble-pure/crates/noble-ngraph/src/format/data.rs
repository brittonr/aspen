#![expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the module path names the domain and the item name keeps its canonical operation spelling"
)]

//! Training-data files `noble-ngraph-data/0` (design D5).
//!
//! ```text
//! noble-ngraph-data/0
//! types f32 f32 ; f32
//! point 0x3f000000 0x3e800000 ; 0x3f400000
//! ```
//!
//! The types before `;` are the model's ordered input slot types, the types
//! after it the loss graph's target slot types. Row order is preserved; the
//! training-data digest binds it.

/// Schema line.
pub const SCHEMA: &str = "noble-ngraph-data/0";

/// A parsed training-data file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Data {
    /// Declared model input types.
    pub inputs: alloc::vec::Vec<crate::ir::ScalarType>,
    /// Declared loss target types.
    pub targets: alloc::vec::Vec<crate::ir::ScalarType>,
    /// Points in file order: (inputs, targets).
    pub points: alloc::vec::Vec<(
        alloc::vec::Vec<crate::sem::Value>,
        alloc::vec::Vec<crate::sem::Value>,
    )>,
}

/// Splits tokens at the single `;` separator.
#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; both halves are bounded by the supplied token slice length"
)]
fn halves<'a>(
    tokens: &[&'a str],
    line: usize,
) -> Result<(alloc::vec::Vec<&'a str>, alloc::vec::Vec<&'a str>), crate::diag::Diagnostic> {
    let mut left = alloc::vec::Vec::new();
    let mut right = alloc::vec::Vec::new();
    let mut seen = 0;
    for tok in tokens {
        let mut parts = tok.split(';');
        let first = parts.next().unwrap_or("");
        if !first.is_empty() {
            if seen == 0 {
                left.push(first);
            } else {
                right.push(first);
            }
        }
        for p in parts {
            seen += 1;
            if !p.is_empty() {
                right.push(p);
            }
        }
    }
    if seen != 1 {
        return Err(super::value::format_diag(
            line,
            "expected exactly one `;` separator",
        ));
    }
    Ok((left, right))
}

/// Parses a training-data file.
///
/// # Errors
/// `E-FORMAT` (schema, syntax, missing or duplicate `types`, separator, row
/// counts), `E-LITERAL-INEXACT` or `E-LITERAL-NONFINITE`.
#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; the points vector is bounded by the significant line count"
)]
pub fn parse(text: &str) -> Result<Data, crate::diag::Diagnostic> {
    let mut lines = super::lines(text);
    let header = lines
        .next()
        .ok_or_else(|| super::value::format_diag(1, "missing noble-ngraph-data/0 schema line"))?;
    if header.tokens != [SCHEMA] {
        return Err(super::value::format_diag(
            header.number,
            "missing noble-ngraph-data/0 schema line",
        ));
    }
    let mut mode = super::LiteralMode::Exact;
    let mut is_mode_declared = false;
    let mut is_record_started = false;
    let mut declared: Option<(
        alloc::vec::Vec<crate::ir::ScalarType>,
        alloc::vec::Vec<crate::ir::ScalarType>,
    )> = None;
    let mut points = alloc::vec::Vec::new();
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
                let (a, b) = halves(&l.tokens[1..], l.number)?;
                declared = Some((
                    super::value::types(&a, l.number)?,
                    super::value::types(&b, l.number)?,
                ));
            }
            "point" => {
                let (ti, tt) = declared.as_ref().ok_or_else(|| {
                    super::value::format_diag(l.number, "a point before the types line")
                })?;
                let (a, b) = halves(&l.tokens[1..], l.number)?;
                points.push((
                    super::value::row_with_mode(ti, &a, l.number, mode)?,
                    super::value::row_with_mode(tt, &b, l.number, mode)?,
                ));
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
    let (inputs, targets) =
        declared.ok_or_else(|| super::value::format_diag(header.number, "missing types line"))?;
    Ok(Data {
        inputs,
        targets,
        points,
    })
}

/// Prints a training-data file in canonical form.
#[must_use]
pub fn print(d: &Data) -> alloc::string::String {
    let mut s = alloc::format!(
        "{SCHEMA}\ntypes {} ; {}\n",
        super::value::type_names(&d.inputs),
        super::value::type_names(&d.targets)
    );
    for (i, t) in &d.points {
        s.push_str("point ");
        s.push_str(&super::value::row_text(i));
        s.push_str(" ; ");
        s.push_str(&super::value::row_text(t));
        s.push('\n');
    }
    s
}

impl Data {
    /// The binary32 training points.
    ///
    /// # Errors
    /// `E-INTERFACE` if a declared type is not `f32`.
    pub fn to_points(
        &self,
    ) -> Result<alloc::vec::Vec<crate::train::Point>, crate::diag::Diagnostic> {
        if self
            .inputs
            .iter()
            .chain(&self.targets)
            .any(|t| *t != crate::ir::ScalarType::F32)
        {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::Interface,
                crate::diag::Site::Graph,
                "training data must be f32",
            ));
        }
        let f = |vs: &[crate::sem::Value]| {
            vs.iter()
                .map(|v| match v {
                    crate::sem::Value::F32(b) => f32::from_bits(*b),
                    crate::sem::Value::I64(_) | crate::sem::Value::Bool(_) => f32::NAN,
                })
                .collect::<alloc::vec::Vec<f32>>()
        };
        Ok(self
            .points
            .iter()
            .map(|(i, t)| crate::train::Point {
                inputs: f(i),
                targets: f(t),
            })
            .collect())
    }

    /// The file form of binary32 training points.
    #[must_use]
    pub fn of_points(points: &[crate::train::Point]) -> Self {
        let f = |xs: &[f32]| {
            xs.iter()
                .map(|x| crate::sem::Value::f32(*x))
                .collect::<alloc::vec::Vec<crate::sem::Value>>()
        };
        let (ni, nt) = points
            .first()
            .map_or((0, 0), |p| (p.inputs.len(), p.targets.len()));
        Self {
            inputs: alloc::vec![crate::ir::ScalarType::F32; ni],
            targets: alloc::vec![crate::ir::ScalarType::F32; nt],
            points: points
                .iter()
                .map(|p| (f(&p.inputs), f(&p.targets)))
                .collect(),
        }
    }
}
