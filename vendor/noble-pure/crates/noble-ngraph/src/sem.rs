//! D3 binary32 contract and K-NUM-01 `I64` operations, and `≡obs`.
//!
//! Host binary32 arithmetic is IEEE 754 RNE on the pinned x86_64 target
//! (SSE, no FTZ/DAZ, no contraction: rustc never contracts `a * b + c`). The
//! independent `rustc_apfloat` leg of the `noble-ngraph-experiment` harness
//! checks this per operation; it is not assumed.
//!
//! The library is `no_std`: float functions that `core` does not provide are
//! the correctly rounded software routines in [`soft`].

/// A graph value at the ABI (D3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Value {
    /// `I64`.
    I64(i64),
    /// `Bool`.
    Bool(bool),
    /// `F32` as binary32 bits.
    F32(u32),
}

impl Value {
    /// Type.
    #[must_use]
    pub const fn ty(self) -> crate::ir::ScalarType {
        match self {
            Self::I64(_) => crate::ir::ScalarType::I64,
            Self::Bool(_) => crate::ir::ScalarType::Bool,
            Self::F32(_) => crate::ir::ScalarType::F32,
        }
    }

    /// `F32` value.
    #[must_use]
    pub const fn f32(v: f32) -> Self {
        Self::F32(v.to_bits())
    }

    /// The literal's value.
    #[must_use]
    pub const fn of_literal(l: crate::ir::Literal) -> Self {
        match l {
            crate::ir::Literal::I64(v) => Self::I64(v),
            crate::ir::Literal::Bool(v) => Self::Bool(v),
            crate::ir::Literal::F32(v) => Self::F32(v),
        }
    }

    /// Little-endian ABI bytes (D3).
    #[expect(
        tigerstyle::borrowed_argument_types,
        reason = "Owner: noble-maintainers; the ABI encoding appends a variable number of bytes to the owned output buffer"
    )]
    pub fn write_abi(self, out: &mut alloc::vec::Vec<u8>) {
        match self {
            Self::I64(v) => out.extend_from_slice(&v.to_le_bytes()),
            Self::Bool(v) => out.push(u8::from(v)),
            Self::F32(v) => out.extend_from_slice(&v.to_le_bytes()),
        }
    }
}

/// `≡obs` on one value: bitwise, except all NaN encodings form one class.
#[must_use]
pub fn obs_eq(a: Value, b: Value) -> bool {
    match (a, b) {
        (Value::F32(x), Value::F32(y)) => {
            let (fx, fy) = (f32::from_bits(x), f32::from_bits(y));
            (fx.is_nan() && fy.is_nan()) || x == y
        }
        _ => a == b,
    }
}

/// `≡obs` on value sequences.
#[must_use]
pub fn obs_eq_all(a: &[Value], b: &[Value]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| obs_eq(*x, *y))
}

/// Binary32 `+0` bits.
pub const POS_ZERO: u32 = 0x0000_0000;
/// Binary32 `-0` bits (bias of a bias-free sum).
pub const NEG_ZERO: u32 = 0x8000_0000;
/// Binary32 `1` bits.
pub const ONE: u32 = 0x3f80_0000;

/// `rn(a + b)`.
#[must_use]
pub fn add(a: f32, b: f32) -> f32 {
    a + b
}

/// `rn(a - b)`.
#[must_use]
pub fn sub(a: f32, b: f32) -> f32 {
    a - b
}

/// `rn(a × b)`.
#[must_use]
pub fn mul(a: f32, b: f32) -> f32 {
    a * b
}

/// `relu(x)`: NaN ↦ NaN; `x > 0 ? x : +0` (so `relu(-0) = +0`).
#[must_use]
pub fn relu(x: f32) -> f32 {
    if x.is_nan() || x > 0.0 {
        x
    } else {
        0.0
    }
}

/// `reglu(l, g) = rn(l × relu(g))`.
#[must_use]
pub fn reglu(l: f32, g: f32) -> f32 {
    l * relu(g)
}

/// `step(s) = s ≥ 0.5 ? 1 : +0` (defined on the checked range `[0,1]`).
#[must_use]
pub fn step(s: f32) -> f32 {
    if s >= 0.5 {
        1.0
    } else {
        0.0
    }
}

/// Whether bits are an exact bit (`+0` or `1`).
#[must_use]
pub const fn is_bit(bits: u32) -> bool {
    bits == POS_ZERO || bits == ONE
}

/// `select`: bitwise copy of `on` if `bit` is `1`, else of `off`. Callers
/// guarantee `bit` is an exact bit (checked fact).
#[must_use]
pub const fn select(bit: u32, off: u32, on: u32) -> u32 {
    if bit == ONE {
        on
    } else {
        off
    }
}

/// A refused `index` read: the caller reports `G-FACT-FALSE` with the
/// table's range fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexRefused;

/// Index position selected by `index` under the out-of-range rules: `Ok(Some(k))`
/// = candidate `k`; `Ok(None)` = `+0` (`zero` rule); `Err(IndexRefused)` = refused (guard
/// failure). A non-integral or NaN index is refused (it cannot be selected; the
/// checked `integral` fact normally excludes it statically).
pub fn index_position(
    i: f32,
    len: usize,
    below: crate::ir::OutOfRange,
    above: crate::ir::OutOfRange,
) -> Result<Option<usize>, IndexRefused> {
    if i.is_nan() || i.is_infinite() || soft::trunc_f32(i) != i {
        return Err(IndexRefused);
    }
    if i < 0.0 {
        return match below {
            crate::ir::OutOfRange::Refuse => Err(IndexRefused),
            crate::ir::OutOfRange::Clamp => Ok(Some(0)),
            crate::ir::OutOfRange::Zero => Ok(None),
        };
    }
    // |i| < 2^24 under the checked range fact; larger finite values still
    // compare correctly against `len` (≤ 65,536) as f64.
    let fi = f64::from(i);
    if fi >= len as f64 {
        return match above {
            crate::ir::OutOfRange::Refuse => Err(IndexRefused),
            crate::ir::OutOfRange::Clamp => Ok(Some(len - 1)),
            crate::ir::OutOfRange::Zero => Ok(None),
        };
    }
    Ok(Some(fi as usize))
}

/// Ordered `affine` fold: `v₀ = bias; vᵢ = rn(vᵢ₋₁ + rn(cᵢ × xᵢ))`.
#[must_use]
pub fn affine(bias: f32, terms: impl IntoIterator<Item = (f32, f32)>) -> f32 {
    let mut v = bias;
    for (c, x) in terms {
        v += c * x;
    }
    v
}

/// K-NUM-01 wrapping `a + b` (`a` deeper).
#[must_use]
pub const fn i64_add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

/// K-NUM-01 wrapping `a - b` (`a` deeper).
#[must_use]
pub const fn i64_sub(a: i64, b: i64) -> i64 {
    a.wrapping_sub(b)
}

/// K-NUM-01 wrapping `a * b` (`a` deeper).
#[must_use]
pub const fn i64_mul(a: i64, b: i64) -> i64 {
    a.wrapping_mul(b)
}

/// A decimal literal reduced to its significant digits `int ‖ frac`: the
/// value is `±(int ‖ frac) × 10^exponent`, and the joined digits have no
/// leading or trailing zero (both parts are empty for zero).
struct Decimal<'a> {
    negative: bool,
    /// Significant digits before the `.`.
    int: &'a str,
    /// Significant digits after the `.`.
    frac: &'a str,
    /// Power of ten of the last significant digit (saturating).
    exponent: i64,
}

/// Whether `text` is a decimal literal of the `ngraph` file formats:
/// `[-]digits[.digits][(e|E)[+|-]digits]`, where either the integer or the
/// fraction digits may be empty but not both. A leading `+` is not part of
/// the grammar (design D5).
#[must_use]
pub fn is_decimal(text: &str) -> bool {
    decimal(text).is_some()
}

/// Splits a decimal literal and strips its zeros in one linear pass, without
/// copying digits.
fn decimal(text: &str) -> Option<Decimal<'_>> {
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (mantissa, exp_text) = match body.find(['e', 'E']) {
        Some(p) => (&body[..p], Some(&body[p + 1..])),
        None => (body, None),
    };
    let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if (int_part.is_empty() && frac_part.is_empty())
        || !all_digits(int_part)
        || !all_digits(frac_part)
    {
        return None;
    }
    let written: i64 = match exp_text {
        None => 0,
        Some(e) => {
            let (sign, digits) = match e.as_bytes().first() {
                Some(b'-') => (-1, &e[1..]),
                Some(b'+') => (1, &e[1..]),
                Some(_) | None => (1, e),
            };
            if digits.is_empty() || !all_digits(digits) {
                return None;
            }
            // Saturation keeps every overlong exponent outside the window.
            digits.bytes().fold(0i64, |acc, b| {
                acc.saturating_mul(10).saturating_add(i64::from(b - b'0'))
            }) * sign
        }
    };
    let int_digits = int_part.trim_start_matches('0');
    let frac_digits = frac_part.trim_end_matches('0');
    let length = |s: &str| i64::try_from(s.len()).ok();
    Some(if frac_digits.is_empty() {
        // An integer: its trailing zeros move into the exponent.
        let kept = int_digits.trim_end_matches('0');
        Decimal {
            negative,
            int: kept,
            frac: "",
            exponent: written.saturating_add(length(int_digits)? - length(kept)?),
        }
    } else if int_digits.is_empty() {
        // A pure fraction: its leading zeros are positions, not digits.
        Decimal {
            negative,
            int: "",
            frac: frac_digits.trim_start_matches('0'),
            exponent: written.saturating_sub(length(frac_digits)?),
        }
    } else {
        Decimal {
            negative,
            int: int_digits,
            frac: frac_digits,
            exponent: written.saturating_sub(length(frac_digits)?),
        }
    })
}

/// Most significant digits of a finite nonzero binary32 value written
/// exactly. Every such value is `M × 2^q` with `M < 2^24` and
/// `-149 ≤ q ≤ 104`. For `q ≥ 0` it is an integer below `2^128 < 10^39`. For
/// `q < 0` it is `M × 5^-q × 10^q`, whose digits `M × 5^-q` are at most
/// `(2^24 - 1) × 5^149 < 10^112` (112 digits, reached by
/// `(2^24 - 1) × 2^-149`).
const EXACT_DIGITS: i64 = 112;
/// Lowest power of ten of an exact binary32 value's last significant digit
/// (`q = -149`).
const EXACT_MIN_EXPONENT: i64 = -149;
/// Highest such power (`2^128 < 10^39` leaves at most 39 integer digits).
const EXACT_MAX_EXPONENT: i64 = 38;
/// The leading digit's power of ten plus one lies in `[-44, 39]`, because
/// `2^-149 > 10^-45` and `2^128 < 10^39`.
const EXACT_MIN_MAGNITUDE: i64 = -44;
/// See [`EXACT_MIN_MAGNITUDE`].
const EXACT_MAX_MAGNITUDE: i64 = 39;

/// The binary32 encoding of a decimal literal when it denotes a binary32
/// value exactly (`E-LITERAL-INEXACT` otherwise; design D3, D5).
///
/// Leading zeros and trailing fraction zeros are stripped in one linear
/// pass, and a literal outside the window that every exact binary32 value
/// satisfies (at most [`EXACT_DIGITS`] significant digits, last digit at
/// `10^-149 … 10^38`, leading digit at `10^-45 … 10^38`) is refused before
/// any big-integer work. Inside the window the exact comparison works on at
/// most 112 digits and exponents of at most 149, so the whole check is linear
/// in the literal's length. `None` also for text outside the decimal grammar
/// ([`is_decimal`]); callers report that separately.
#[must_use]
pub fn exact_f32_decimal(text: &str) -> Option<u32> {
    let d = decimal(text)?;
    let count = d.int.len().saturating_add(d.frac.len());
    if count == 0 {
        return Some(if d.negative { NEG_ZERO } else { POS_ZERO });
    }
    let n = i64::try_from(count).ok()?;
    let magnitude = n.saturating_add(d.exponent);
    if n > EXACT_DIGITS
        || !(EXACT_MIN_EXPONENT..=EXACT_MAX_EXPONENT).contains(&d.exponent)
        || !(EXACT_MIN_MAGNITUDE..=EXACT_MAX_MAGNITUDE).contains(&magnitude)
    {
        return None;
    }
    // At most 112 digits and an exponent in [-149, 38]: a short literal.
    let sign = if d.negative { "-" } else { "" };
    let v: f32 = alloc::format!("{sign}{}{}e{}", d.int, d.frac, d.exponent)
        .parse()
        .ok()?;
    if !v.is_finite() {
        return None;
    }
    let e10 = i32::try_from(d.exponent).ok()?;
    // decimal = D · 10^e10 ; binary = M · 2^e2 (M odd or 0).
    let bits = v.abs().to_bits();
    let (m, e2) = if bits == 0 {
        (0u64, 0i32)
    } else {
        let exp = i32::try_from((bits >> 23) & 0xff).unwrap_or(0);
        let frac = u64::from(bits & 0x7f_ffff);
        if exp == 0 {
            (frac, -149)
        } else {
            (frac | 0x80_0000, exp - 150)
        }
    };
    if m == 0 {
        // Nonzero digits that rounded to zero.
        return None;
    }
    // Compare D·10^e10 with m·2^e2 by cross-multiplying to integers.
    let mut lhs = Big::from_decimal(d.int.bytes().chain(d.frac.bytes()));
    let mut rhs = Big::from_u64(m);
    if e10 >= 0 {
        lhs.mul_pow(10, e10.unsigned_abs());
    } else {
        rhs.mul_pow(10, e10.unsigned_abs());
    }
    if e2 >= 0 {
        rhs.mul_pow(2, e2.unsigned_abs());
    } else {
        lhs.mul_pow(2, e2.unsigned_abs());
    }
    lhs.trim();
    rhs.trim();
    (lhs.0 == rhs.0).then_some(v.to_bits())
}

/// Maximum byte length accepted by the explicit decimal-to-binary32 RNE path.
pub const MAX_RNE_DECIMAL_BYTES: usize = 4096;

/// Refusal reasons for explicit decimal RNE import.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecimalRefusal {
    /// The token is not in the graph decimal grammar.
    Malformed,
    /// The decimal rounds to infinity; v2 chooses a checked refusal.
    Overflow,
    /// The token exceeds [`MAX_RNE_DECIMAL_BYTES`].
    TooLong,
}

/// Parse an explicit decimal as an exact base-ten rational and round it once
/// to binary32 using round-to-nearest, ties-to-even. No host floating-point
/// parser or intermediate binary format participates in this result.
///
/// Exponents are saturated by [`decimal`], the decimal order is classified
/// before any big-integer expansion, and the remaining exact work is bounded
/// by the token-length cap.
pub fn rne_f32_decimal(text: &str) -> Result<u32, DecimalRefusal> {
    if text.len() > MAX_RNE_DECIMAL_BYTES {
        return Err(DecimalRefusal::TooLong);
    }
    let d = decimal(text).ok_or(DecimalRefusal::Malformed)?;
    let digits = d.int.len().saturating_add(d.frac.len());
    if digits == 0 {
        return Ok(if d.negative { NEG_ZERO } else { POS_ZERO });
    }
    let digit_count = i64::try_from(digits).map_err(|_| DecimalRefusal::TooLong)?;
    let decimal_order = digit_count.saturating_sub(1).saturating_add(d.exponent);
    if decimal_order > 38 {
        return Err(DecimalRefusal::Overflow);
    }
    if decimal_order < -46 {
        return Ok(if d.negative { NEG_ZERO } else { POS_ZERO });
    }

    let mut numerator = Big::from_decimal(d.int.bytes().chain(d.frac.bytes()));
    let mut denominator = Big::from_u64(1);
    if d.exponent >= 0 {
        let power = u32::try_from(d.exponent).map_err(|_| DecimalRefusal::TooLong)?;
        numerator.mul_pow(10, power);
    } else {
        let power =
            u32::try_from(d.exponent.unsigned_abs()).map_err(|_| DecimalRefusal::TooLong)?;
        denominator.mul_pow(10, power);
    }

    let exponent = floor_log2_ratio(&numerator, &denominator);
    let magnitude = if exponent >= -126 {
        let scale = 23 - exponent;
        let (scaled_numerator, scaled_denominator) = scale_ratio(&numerator, &denominator, scale);
        let rounded = round_quotient(&scaled_numerator, &scaled_denominator, 1 << 24);
        let (significand, exponent) = if rounded == 1 << 24 {
            (1 << 23, exponent + 1)
        } else {
            (rounded, exponent)
        };
        if exponent > 127 {
            return Err(DecimalRefusal::Overflow);
        }
        let exponent = u32::try_from(exponent + 127).map_err(|_| DecimalRefusal::Overflow)?;
        (exponent << 23) | (significand - (1 << 23))
    } else {
        let scaled_numerator = scaled_by_power(&numerator, 149);
        let rounded = round_quotient(&scaled_numerator, &denominator, (1 << 23) + 1);
        rounded.min(1 << 23)
    };
    Ok(if d.negative {
        magnitude | 0x8000_0000
    } else {
        magnitude
    })
}

/// Errors shared by the explicit scalar conversion adapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionError {
    /// The requested exact conversion would discard information.
    Inexact,
    /// The source encoding is NaN or infinity.
    NotFinite,
    /// The destination integer range cannot contain the result.
    Overflow,
}

impl ConversionError {
    /// Stable diagnostic code for callers that surface a refused conversion.
    #[must_use]
    pub const fn code(self) -> crate::diag::Code {
        match self {
            Self::Inexact => crate::diag::Code::ConversionInexact,
            Self::NotFinite => crate::diag::Code::ConversionNonfinite,
            Self::Overflow => crate::diag::Code::ConversionOverflow,
        }
    }
}

/// I64 to binary32 round-to-nearest, ties-to-even conversion, implemented on
/// integer bits and valid for every signed 64-bit input.
#[must_use]
pub fn i64_to_f32_rne(value: i64) -> u32 {
    let is_negative = value < 0;
    let magnitude = value.unsigned_abs();
    if magnitude == 0 {
        return POS_ZERO;
    }
    let exponent = 63 - i32::try_from(magnitude.leading_zeros()).unwrap_or(64);
    let shift = u32::try_from((exponent - 23).max(0)).unwrap_or(0);
    let mut significand = if exponent < 23 {
        magnitude << u32::try_from(23 - exponent).unwrap_or(0)
    } else {
        magnitude >> shift
    };
    if exponent > 23 {
        let remainder = magnitude & ((1u64 << shift) - 1);
        let halfway = 1u64 << (shift - 1);
        if remainder > halfway || (remainder == halfway && significand & 1 == 1) {
            significand += 1;
        }
    }
    let mut encoded_exponent = exponent;
    if significand == 1 << 24 {
        significand >>= 1;
        encoded_exponent += 1;
    }
    let fraction = u32::try_from(significand - (1 << 23)).unwrap_or(0);
    let mut bits = (u32::try_from(encoded_exponent + 127).unwrap_or(0) << 23) | fraction;
    if is_negative {
        bits |= 0x8000_0000;
    }
    bits
}

/// I64 to binary32 conversion that refuses an inexact rounded result.
pub fn i64_to_f32_exact(value: i64) -> Result<u32, ConversionError> {
    let bits = i64_to_f32_rne(value);
    let magnitude = value.unsigned_abs();
    let exponent = 63 - i32::try_from(magnitude.leading_zeros()).unwrap_or(64);
    let shift = u32::try_from((exponent - 23).max(0)).unwrap_or(0);
    if shift != 0 && magnitude & ((1u64 << shift) - 1) != 0 {
        Err(ConversionError::Inexact)
    } else {
        Ok(bits)
    }
}

/// Binary32 to I64 checked truncation-toward-zero conversion.
pub fn f32_to_i64_trunc(bits: u32) -> Result<i64, ConversionError> {
    f32_to_i64(bits, false)
}

/// Binary32 to I64 conversion that accepts only integral, in-range finite
/// values.
pub fn f32_to_i64_exact(bits: u32) -> Result<i64, ConversionError> {
    f32_to_i64(bits, true)
}

fn f32_to_i64(bits: u32, require_exact: bool) -> Result<i64, ConversionError> {
    let is_negative = bits >> 31 != 0;
    let encoded_exponent = (bits >> 23) & 0xff;
    if encoded_exponent == 0xff {
        return Err(ConversionError::NotFinite);
    }
    let fraction = bits & 0x7f_ffff;
    let significand = if encoded_exponent == 0 {
        fraction
    } else {
        fraction | 0x80_0000
    };
    if significand == 0 {
        return Ok(0);
    }
    let exponent = if encoded_exponent == 0 {
        -149
    } else {
        i32::try_from(encoded_exponent).unwrap_or(0) - 150
    };
    let magnitude = if exponent >= 0 {
        let shift = u32::try_from(exponent).unwrap_or(u32::MAX);
        if shift >= 64 || u64::from(significand) > (u64::MAX >> shift) {
            return Err(ConversionError::Overflow);
        }
        u64::from(significand) << shift
    } else {
        let shift = u32::try_from(-exponent).unwrap_or(u32::MAX);
        if require_exact && (shift >= 32 || significand & ((1u32 << shift) - 1) != 0) {
            return Err(ConversionError::Inexact);
        }
        if shift >= 32 {
            0
        } else {
            u64::from(significand >> shift)
        }
    };
    let bound = if is_negative {
        1u64 << 63
    } else {
        u64::try_from(i64::MAX).unwrap_or(0)
    };
    if magnitude > bound {
        return Err(ConversionError::Overflow);
    }
    if is_negative {
        if magnitude == 1u64 << 63 {
            Ok(i64::MIN)
        } else {
            i64::try_from(magnitude)
                .map(|v| -v)
                .map_err(|_| ConversionError::Overflow)
        }
    } else {
        i64::try_from(magnitude).map_err(|_| ConversionError::Overflow)
    }
}

/// Minimal unsigned big integer for exact literal checks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Big(alloc::vec::Vec<u32>);

impl Big {
    #[expect(
        tigerstyle::unchecked_narrowing,
        reason = "Owner: noble-maintainers; the low and high masks select exactly 32 bits each, so the u64 split is lossless"
    )]
    fn from_u64(v: u64) -> Self {
        let mut b = Self(alloc::vec![(v & 0xffff_ffff) as u32, (v >> 32) as u32]);
        b.trim();
        b
    }
    /// The value of ASCII decimal digits (callers pass at most 112).
    fn from_decimal(digits: impl Iterator<Item = u8>) -> Self {
        let mut b = Self::default();
        for c in digits {
            b.mul_small(10);
            b.add_small(u32::from(c - b'0'));
        }
        b
    }
    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }
    fn compare(&self, other: &Self) -> core::cmp::Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
    fn assign_mul_small(&mut self, value: &Self, factor: u32) {
        self.0.clear();
        self.0.resize(value.0.len(), 0);
        let mut carry = 0u64;
        for (out, limb) in self.0.iter_mut().zip(&value.0) {
            let product = u64::from(*limb) * u64::from(factor) + carry;
            *out = u32::try_from(product & 0xffff_ffff).unwrap_or(0);
            carry = product >> 32;
        }
        if carry != 0 {
            self.0.push(u32::try_from(carry).unwrap_or(0));
        }
        self.trim();
    }
    fn subtract_assign(&mut self, other: &Self) {
        debug_assert!(self.compare(other) != core::cmp::Ordering::Less);
        let mut borrow = 0u64;
        for (index, limb) in self.0.iter_mut().enumerate() {
            let subtrahend = u64::from(other.0.get(index).copied().unwrap_or(0)) + borrow;
            let minuend = u64::from(*limb);
            *limb = u32::try_from(minuend.wrapping_sub(subtrahend) & 0xffff_ffff).unwrap_or(0);
            borrow = u64::from(minuend < subtrahend);
        }
        debug_assert_eq!(borrow, 0);
        self.trim();
    }
    #[expect(
        tigerstyle::unchecked_narrowing,
        reason = "Owner: noble-maintainers; a u32x u32 product plus a u32 carry is below 2^64, so the masked limb and shifted carry fit u32"
    )]
    fn mul_small(&mut self, k: u32) {
        let mut carry = 0u64;
        for l in &mut self.0 {
            let p = u64::from(*l) * u64::from(k) + carry;
            *l = (p & 0xffff_ffff) as u32;
            carry = p >> 32;
        }
        if carry != 0 {
            self.0.push(carry as u32);
        }
    }
    #[expect(
        tigerstyle::unchecked_narrowing,
        reason = "Owner: noble-maintainers; a u32 limb plus a u32 carry is below 2^33, so the masked limb and shifted carry fit u32"
    )]
    fn add_small(&mut self, k: u32) {
        let mut carry = u64::from(k);
        for l in &mut self.0 {
            if carry == 0 {
                break;
            }
            let s = u64::from(*l) + carry;
            *l = (s & 0xffff_ffff) as u32;
            carry = s >> 32;
        }
        if carry != 0 {
            self.0.push(carry as u32);
        }
    }
    fn mul_pow(&mut self, base: u32, n: u32) {
        for _ in 0..n {
            self.mul_small(base);
        }
        self.trim();
    }
}

fn scaled_by_power(value: &Big, exponent: u32) -> Big {
    let mut scaled = value.clone();
    scaled.mul_pow(2, exponent);
    scaled
}

fn scale_ratio(numerator: &Big, denominator: &Big, exponent: i32) -> (Big, Big) {
    if exponent >= 0 {
        (
            scaled_by_power(numerator, u32::try_from(exponent).unwrap_or(0)),
            denominator.clone(),
        )
    } else {
        (
            numerator.clone(),
            scaled_by_power(denominator, exponent.unsigned_abs()),
        )
    }
}

fn ratio_at_least_power(numerator: &Big, denominator: &Big, exponent: i32) -> bool {
    if exponent >= 0 {
        numerator.compare(&scaled_by_power(
            denominator,
            u32::try_from(exponent).unwrap_or(0),
        )) != core::cmp::Ordering::Less
    } else {
        scaled_by_power(numerator, exponent.unsigned_abs()).compare(denominator)
            != core::cmp::Ordering::Less
    }
}

fn floor_log2_ratio(numerator: &Big, denominator: &Big) -> i32 {
    let mut lower = -256;
    let mut upper = 256;
    while lower + 1 < upper {
        let middle = lower + (upper - lower) / 2;
        if ratio_at_least_power(numerator, denominator, middle) {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    lower
}

fn quotient_at_most(numerator: &Big, denominator: &Big, maximum: u32) -> u32 {
    let mut lower = 0u32;
    let mut upper = maximum + 1;
    let mut product = Big::default();
    while lower + 1 < upper {
        let middle = lower + (upper - lower) / 2;
        product.assign_mul_small(denominator, middle);
        if product.compare(numerator) != core::cmp::Ordering::Greater {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    lower
}

fn round_quotient(numerator: &Big, denominator: &Big, maximum: u32) -> u32 {
    let quotient = quotient_at_most(numerator, denominator, maximum);
    let mut remainder = numerator.clone();
    let mut product = Big::default();
    product.assign_mul_small(denominator, quotient);
    remainder.subtract_assign(&product);
    let mut doubled_remainder = remainder;
    doubled_remainder.mul_small(2);
    match doubled_remainder.compare(denominator) {
        core::cmp::Ordering::Greater => quotient + 1,
        core::cmp::Ordering::Equal if quotient & 1 == 1 => quotient + 1,
        core::cmp::Ordering::Equal | core::cmp::Ordering::Less => quotient,
    }
}

/// Software float routines for the `no_std` library (design D2): exact
/// `trunc`/`floor`/`ceil`/`round` by exponent and mantissa masking, an
/// exact power of two, and a correctly rounded (roundTiesToEven) binary64
/// square root. Each agrees bitwise with the corresponding `std` method on
/// every non-NaN input; NaN inputs give a NaN.
pub mod soft {
    const F64_SIGN: u64 = 1 << 63;
    const F64_FRAC: u64 = (1 << 52) - 1;

    /// `f64::trunc`: rounds toward zero, keeping the sign of zero.
    #[expect(
        tigerstyle::unchecked_narrowing,
        reason = "Owner: noble-maintainers; the exponent is masked to 11 bits before the i32 conversion"
    )]
    #[must_use]
    pub const fn trunc_f64(x: f64) -> f64 {
        let bits = x.to_bits();
        let biased = ((bits >> 52) & 0x7ff) as i32;
        if biased == 0x7ff {
            return x;
        }
        let e = biased - 1023;
        if e < 0 {
            return f64::from_bits(bits & F64_SIGN);
        }
        if e >= 52 {
            return x;
        }
        let mask = (1u64 << (52 - e)) - 1;
        f64::from_bits(bits & !mask)
    }

    /// `f32::trunc`: rounds toward zero, keeping the sign of zero.
    #[must_use]
    pub const fn trunc_f32(x: f32) -> f32 {
        let bits = x.to_bits();
        let biased = ((bits >> 23) & 0xff) as i32;
        if biased == 0xff {
            return x;
        }
        let e = biased - 127;
        if e < 0 {
            return f32::from_bits(bits & 0x8000_0000);
        }
        if e >= 23 {
            return x;
        }
        let mask = (1u32 << (23 - e)) - 1;
        f32::from_bits(bits & !mask)
    }

    /// `f64::floor`.
    #[must_use]
    pub fn floor_f64(x: f64) -> f64 {
        let t = trunc_f64(x);
        // `t == x` covers integers, zeros and infinities; NaN falls through
        // to the comparison below and is returned unchanged.
        if t == x || x.is_nan() {
            return x;
        }
        // |t| < 2^52 here, so `t - 1` is exact.
        if x < 0.0 {
            t - 1.0
        } else {
            t
        }
    }

    /// `f64::ceil`.
    #[must_use]
    pub fn ceil_f64(x: f64) -> f64 {
        let t = trunc_f64(x);
        if t == x || x.is_nan() {
            return x;
        }
        if x > 0.0 {
            t + 1.0
        } else {
            t
        }
    }

    /// `f64::round`: half-way cases round away from zero.
    #[must_use]
    pub fn round_f64(x: f64) -> f64 {
        let t = trunc_f64(x);
        if t == x || x.is_nan() {
            return x;
        }
        // `x - t` is the exact fractional part (representable), |·| < 1.
        let frac = x - t;
        if x > 0.0 {
            if frac >= 0.5 {
                t + 1.0
            } else {
                t
            }
        } else if frac <= -0.5 {
            t - 1.0
        } else {
            t
        }
    }

    /// The exact binary64 value `2^k`: subnormal below `-1022`, `+0`
    /// below `-1074` and `+∞` above `1023`.
    #[must_use]
    pub const fn pow2_f64(k: i32) -> f64 {
        if k > 1023 {
            f64::INFINITY
        } else if k >= -1022 {
            f64::from_bits(((k + 1023) as u64) << 52)
        } else if k >= -1074 {
            f64::from_bits(1u64 << (k + 1074))
        } else {
            0.0
        }
    }

    /// `f64::sqrt`, correctly rounded to nearest, ties to even.
    ///
    /// For finite `x > 0`, `x = M·2^E` with a normalized 53- or 54-bit
    /// integer `M` and even `E`. Then `r = isqrt(M·2^58)` has 56 bits; its
    /// top 53 bits are the truncated significand, bit 2 is the round bit and
    /// the low two bits with the remainder `M·2^58 − r²` form the sticky
    /// bit. An exact tie cannot occur (an odd 54-bit square root would need
    /// a 107-bit square), but the rule handles one.
    #[expect(
        tigerstyle::unchecked_narrowing,
        reason = "Owner: noble-maintainers; the exponent is masked to 11 bits and the 56-bit square-root significand fits u64"
    )]
    #[must_use]
    pub fn sqrt_f64(x: f64) -> f64 {
        let bits = x.to_bits();
        if x.is_nan() || (bits & F64_SIGN != 0 && x != 0.0) {
            return f64::NAN;
        }
        if x == 0.0 || x == f64::INFINITY {
            return x;
        }
        let biased = ((bits >> 52) & 0x7ff) as i32;
        let (mut m, mut e) = if biased == 0 {
            (bits & F64_FRAC, -1074)
        } else {
            ((bits & F64_FRAC) | (1 << 52), biased - 1075)
        };
        // Normalize subnormals so that bit 52 is the leading bit.
        let shift = m.leading_zeros() as i32 - 11;
        m <<= shift;
        e -= shift;
        if e & 1 != 0 {
            m <<= 1;
            e -= 1;
        }
        let n = u128::from(m) << 58;
        let r = n.isqrt();
        let rem = n - r * r;
        let is_round = (r >> 2) & 1 == 1;
        let has_sticky = r & 0b11 != 0 || rem != 0;
        let mut sig = (r >> 3) as u64;
        if is_round && (has_sticky || sig & 1 == 1) {
            sig += 1;
        }
        let mut e_res = 3 + (e - 58) / 2;
        if sig == 1 << 53 {
            sig >>= 1;
            e_res += 1;
        }
        let biased_res = (e_res + 1075) as u64;
        f64::from_bits((biased_res << 52) | (sig & F64_FRAC))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_decimal_literals() {
        assert_eq!(exact_f32_decimal("1.5"), Some(1.5f32.to_bits()));
        assert_eq!(exact_f32_decimal("-0"), Some(NEG_ZERO));
        assert_eq!(exact_f32_decimal("0.1"), None);
        assert_eq!(exact_f32_decimal("16777216"), Some(16_777_216f32.to_bits()));
        assert_eq!(exact_f32_decimal("16777217"), None);
        assert_eq!(exact_f32_decimal("1e40"), None);
        // 2^-149, the least subnormal, written exactly.
        // Decimal digits of 5^149 (least significant first), then e-149.
        let mut digits = alloc::vec![1u8];
        for _ in 0..149 {
            let mut carry = 0;
            for d in &mut digits {
                let v = *d * 5 + carry;
                *d = v % 10;
                carry = v / 10;
            }
            if carry > 0 {
                digits.push(carry);
            }
        }
        let mut s: alloc::string::String =
            digits.iter().rev().map(|d| char::from(b'0' + d)).collect();
        s.push_str("e-149");
        assert_eq!(exact_f32_decimal(&s), Some(1));
    }

    #[test]
    fn signed_zero_rules() {
        assert_eq!(relu(-0.0).to_bits(), POS_ZERO);
        assert_eq!(reglu(-1.0, 0.0).to_bits(), NEG_ZERO);
        assert_eq!(affine(-0.0, [(1.0, -0.0)]).to_bits(), NEG_ZERO);
        assert_eq!(affine(0.0, [(1.0, -0.0)]).to_bits(), POS_ZERO);
        assert!(obs_eq(Value::F32(0x7fc0_0000), Value::F32(0xffc0_0001)));
        assert!(!obs_eq(Value::F32(POS_ZERO), Value::F32(NEG_ZERO)));
    }
}
