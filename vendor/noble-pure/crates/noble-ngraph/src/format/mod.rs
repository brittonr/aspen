//! `Numeric-Graph-v1` file codecs (design D4–D6 of `numeric-graph-v1`).
//!
//! Every file starts with a schema line: `ngraph-ir/0 <sub-profile>` for
//! graphs ([`graph`]), `noble-ngraph-vectors/0` for evaluation vectors
//! ([`vectors`]), `noble-ngraph-data/0` for training data ([`data`]) and
//! `noble-ngraph-snapshot/0` for parameter snapshots ([`snapshot`]). `#`
//! starts a comment; blank lines, comments and runs of spaces carry no
//! meaning. Parsing only builds producer data: graphs are still admitted by
//! the independent checker, and snapshot identities are recomputed.

pub mod data;
pub mod graph;
pub mod snapshot;
pub mod value;
pub mod vectors;

/// Explicit `F32` decimal-literal policy selected by a file declaration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LiteralMode {
    /// Accept exact decimals and preserve the v1 refusal behavior.
    #[default]
    Exact,
    /// Accept `rne:<decimal>` only, with one exact-rational RNE rounding.
    RneV2,
}

impl LiteralMode {
    /// Name recorded by the v2 file declaration.
    pub const V2_NAME: &'static str = "noble-graph-literal-modes/v2";

    /// Parses the version token after `literal-modes`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        (name == Self::V2_NAME).then_some(Self::RneV2)
    }
}

/// Appends formatted text to `out`.
///
/// `String`'s `core::fmt::Write` implementation is infallible. This
/// helper is the crate's single reviewed boundary for that fact; callers
/// do not ignore a `fmt::Result`.
#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; canonical text emission appends to the owned String output buffer and cannot accept a shorter borrowed slice"
)]
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; the formatter is the single explicit text-emission boundary and appends to a caller-owned output buffer"
)]
pub(crate) fn push_fmt(out: &mut alloc::string::String, args: core::fmt::Arguments<'_>) {
    let result = core::fmt::Write::write_fmt(out, args);
    debug_assert!(result.is_ok(), "writing to a String cannot fail");
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; the line helper appends to the owned String output buffer and cannot accept a shorter borrowed slice"
)]
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; the line helper is the single explicit text-emission boundary and appends to a caller-owned output buffer"
)]
pub(crate) fn push_line(out: &mut alloc::string::String, args: core::fmt::Arguments<'_>) {
    push_fmt(out, args);
    out.push('\n');
}

macro_rules! write_text {
    ($out:expr, $($arg:tt)*) => {
        $crate::format::push_fmt(&mut $out, format_args!($($arg)*))
    };
}

macro_rules! write_line {
    ($out:expr, $($arg:tt)*) => {
        $crate::format::push_line(&mut $out, format_args!($($arg)*))
    };
}

#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-maintainers; the macro is re-exported at module scope so call sites keep the canonical formatting spelling"
)]
pub(crate) use write_line;
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-maintainers; the macro is re-exported at module scope so call sites keep the canonical formatting spelling"
)]
pub(crate) use write_text;

/// One significant line: its 1-based number and its whitespace tokens.
pub(crate) struct Line<'a> {
    pub(crate) number: usize,
    pub(crate) tokens: alloc::vec::Vec<&'a str>,
}

/// Significant lines of `text`: comments removed, blank lines skipped.
pub(crate) fn lines(text: &str) -> impl Iterator<Item = Line<'_>> {
    text.lines().enumerate().filter_map(|(i, l)| {
        let body = l.split('#').next().unwrap_or("");
        let tokens: alloc::vec::Vec<&str> = body.split_whitespace().collect();
        (!tokens.is_empty()).then_some(Line {
            number: i + 1,
            tokens,
        })
    })
}

/// Decodes lowercase or uppercase hex bytes; `None` on odd length or a
/// non-hex digit.
#[must_use]
pub fn bytes_from_hex(s: &str) -> Option<alloc::vec::Vec<u8>> {
    let b = s.as_bytes();
    if !b.len().is_multiple_of(2) {
        return None;
    }
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    };
    b.chunks_exact(2)
        .map(|p| Some(digit(p[0])? << 4 | digit(p[1])?))
        .collect()
}

/// Decodes a 64-digit hex digest.
#[must_use]
pub fn digest_from_hex(s: &str) -> Option<crate::ir::Digest> {
    let v = bytes_from_hex(s)?;
    <crate::ir::Digest>::try_from(v.as_slice()).ok()
}

/// Whether `t` is one or more ASCII decimal digits.
fn is_digits(t: &str) -> bool {
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
}

/// An unsigned decimal integer: ASCII digits only, so a sign (including a
/// leading `+`) is refused, as everywhere in the `ngraph` file formats and
/// `noble ngraph` flags (design D5).
#[must_use]
pub fn decimal_u64(t: &str) -> Option<u64> {
    if is_digits(t) {
        t.parse().ok()
    } else {
        None
    }
}

/// [`decimal_u64`] within `u32`.
#[must_use]
pub fn decimal_u32(t: &str) -> Option<u32> {
    decimal_u64(t).and_then(|v| u32::try_from(v).ok())
}

/// A signed decimal integer: an optional `-`, then ASCII digits.
#[must_use]
pub fn decimal_i64(t: &str) -> Option<i64> {
    if is_digits(t.strip_prefix('-').unwrap_or(t)) {
        t.parse().ok()
    } else {
        None
    }
}

/// `0x` followed by one to `max_digits` hex digits (either case).
#[must_use]
pub fn hex_bits(t: &str, max_digits: usize) -> Option<u64> {
    let h = t.strip_prefix("0x")?;
    if h.is_empty() || h.len() > max_digits || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(h, 16).ok()
}
