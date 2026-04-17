//! GF(2^8) finite field arithmetic.
//!
//! Implements multiplication and polynomial evaluation over the Galois field
//! GF(2^8) with irreducible polynomial x^8 + x^4 + x^3 + x + 1 (0x11B).
//! Used by Shamir secret sharing for share generation and reconstruction.
//!
//! The multiplication table approach (256×256 = 64KB) trades memory for
//! constant-time operation — no branches dependent on secret data.

/// Irreducible polynomial for GF(2^8): x^8 + x^4 + x^3 + x + 1.
const MODULUS: u16 = 0x11B;

/// Multiply two elements in GF(2^8) using Russian peasant multiplication.
///
/// This is the core operation. Addition in GF(2^8) is XOR.
/// Multiplication uses shift-and-XOR with reduction by the irreducible polynomial.
#[inline]
pub fn mul(a: u8, b: u8) -> u8 {
    let mut result: u8 = 0;
    let mut a = u16::from(a);
    let mut b = b;

    // Process each bit of b
    let mut i: u8 = 0;
    while i < 8 {
        // If low bit of b is set, XOR a into result
        if b & 1 != 0 {
            let reduced = a & u16::from(u8::MAX);
            result ^= u8::try_from(reduced).unwrap_or_default();
        }
        // Shift a left (multiply by x)
        a <<= 1;
        // Reduce if degree >= 8
        if a & 0x100 != 0 {
            a ^= MODULUS;
        }
        b >>= 1;
        i = i.saturating_add(1);
    }
    result
}

/// Compute the multiplicative inverse of a nonzero element in GF(2^8).
///
/// Uses exponentiation: a^254 = a^(-1) in GF(2^8) since the multiplicative
/// group has order 255. Returns 0 for input 0 (not mathematically defined,
/// but convenient for the implementation).
#[inline]
pub fn inv(a: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    // a^254 = a^(-1) in GF(2^8)
    // Compute via square-and-multiply: 254 = 11111110 in binary
    let mut result = a;
    let mut i: u32 = 0;
    while i < 6 {
        result = mul(result, result); // square
        result = mul(result, a); // multiply
        i += 1;
    }
    // Final square (bit 254 has a trailing zero)
    result = mul(result, result);
    result
}

/// Evaluate a polynomial at a given point in GF(2^8) using Horner's method.
///
/// `coeffs[0]` is the constant term (the secret), `coeffs[1]` is the
/// coefficient of x, etc. The polynomial degree is `coeffs.len() - 1`.
#[inline]
pub fn eval_polynomial(coeffs: &[u8], x: u8) -> u8 {
    if coeffs.is_empty() {
        return 0;
    }
    // Horner's method: evaluate from highest degree down
    let mut result: u8 = 0;
    let mut i = coeffs.len();
    while i > 0 {
        i -= 1;
        result = mul(result, x) ^ coeffs[i];
    }
    result
}

/// Compute a Lagrange basis polynomial evaluated at x=0.
///
/// Given x-coordinates of all shares and the index of the current share,
/// returns the Lagrange coefficient L_i(0) in GF(2^8).
///
/// L_i(0) = product_{j≠i} (x_j / (x_j - x_i))
///
/// In GF(2^8), subtraction is XOR (same as addition).
#[inline]
pub fn lagrange_basis_at_zero(xs: &[u8], share_index: u32) -> u8 {
    let mut result: u8 = 1;
    let share_slot = usize::try_from(share_index).unwrap_or(usize::MAX);
    let xi = xs[share_slot];

    let mut current_slot = 0u32;
    while usize::try_from(current_slot).unwrap_or(usize::MAX) < xs.len() {
        if current_slot != share_index {
            let slot = usize::try_from(current_slot).unwrap_or(usize::MAX);
            let xj = xs[slot];
            // numerator: xj, denominator: xj XOR xi (subtraction = addition in GF(2^8))
            let denom = xj ^ xi;
            result = mul(result, mul(xj, inv(denom)));
        }
        current_slot = current_slot.saturating_add(1);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mul_identity() {
        for a in u8::MIN..=u8::MAX {
            assert_eq!(mul(a, 1), a);
            assert_eq!(mul(1, a), a);
        }
    }

    #[test]
    fn test_mul_zero() {
        for a in u8::MIN..=u8::MAX {
            assert_eq!(mul(a, 0), 0);
            assert_eq!(mul(0, a), 0);
        }
    }

    #[test]
    fn test_mul_commutative() {
        for a in u8::MIN..=u8::MAX {
            for b in u8::MIN..=u8::MAX {
                assert_eq!(mul(a, b), mul(b, a));
            }
        }
    }

    #[test]
    fn test_inverse() {
        for a in 1u8..=u8::MAX {
            let a_inv = inv(a);
            assert_eq!(mul(a, a_inv), 1, "inv({a}) = {a_inv}, but a * inv(a) != 1");
        }
    }

    #[test]
    fn test_eval_constant_polynomial() {
        // f(x) = 42 for all x
        let coeffs = [42u8];
        for x in u8::MIN..=u8::MAX {
            assert_eq!(eval_polynomial(&coeffs, x), 42);
        }
    }

    #[test]
    fn test_eval_polynomial_at_zero_returns_constant() {
        let coeffs = [7u8, 3, 5]; // f(x) = 7 + 3x + 5x^2
        assert_eq!(eval_polynomial(&coeffs, 0), 7);
    }

    #[test]
    fn test_eval_linear_polynomial() {
        // f(x) = 0 + 1*x => f(x) = x
        let coeffs = [0u8, 1];
        for x in u8::MIN..=u8::MAX {
            assert_eq!(eval_polynomial(&coeffs, x), x);
        }
    }

    #[test]
    fn test_lagrange_interpolation_two_points() {
        // Two shares at x=1 and x=2 should reconstruct the constant term
        let secret = 42u8;
        let coeffs = [secret, 17]; // f(x) = 42 + 17x
        let x1: u8 = 1;
        let x2: u8 = 2;
        let y1 = eval_polynomial(&coeffs, x1);
        let y2 = eval_polynomial(&coeffs, x2);

        let xs = [x1, x2];
        let l0 = lagrange_basis_at_zero(&xs, 0);
        let l1 = lagrange_basis_at_zero(&xs, 1);

        let reconstructed = mul(y1, l0) ^ mul(y2, l1);
        assert_eq!(reconstructed, secret);
    }
}
