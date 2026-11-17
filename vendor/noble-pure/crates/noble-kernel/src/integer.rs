//! Noble's fixed-width signed integer value. This is not calculator arithmetic.

/// A Noble I64. Every Rust `i64` is a valid value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct I64 {
    value: i64,
}

// r[impl K-NUM-01]
impl I64 {
    /// Construct a value from an already range-checked signed integer.
    /// Source literal parsing must reject out-of-range text before this boundary.
    #[must_use]
    pub const fn from_i64(value: i64) -> Self {
        Self { value }
    }

    /// Return the signed representation, without changing its bits.
    #[must_use]
    pub const fn to_i64(self) -> i64 {
        self.value
    }

    /// Add the top operand to this deeper operand, modulo 2^64.
    #[must_use]
    pub const fn wrapping_add(self, top: Self) -> Self {
        Self::from_i64(self.value.wrapping_add(top.value))
    }

    /// Subtract the top operand from this deeper operand, modulo 2^64.
    #[must_use]
    pub const fn wrapping_sub(self, top: Self) -> Self {
        Self::from_i64(self.value.wrapping_sub(top.value))
    }

    /// Multiply this deeper operand by the top operand, modulo 2^64.
    #[must_use]
    pub const fn wrapping_mul(self, top: Self) -> Self {
        Self::from_i64(self.value.wrapping_mul(top.value))
    }

    /// Compare two I64 values. This does not implement generic Noble equality.
    #[must_use]
    pub const fn is_equal(self, other: Self) -> bool {
        self.value == other.value
    }
}
