//! `splitmix64/0`: the host-side PRNG named by D11 (initialization, data and
//! random-graph generation). Never used inside a graph.

/// PRNG revision recorded in provenance.
pub const PRNG: &str = "splitmix64/0";

/// SplitMix64 (Steele, Lea, Flood 2014) with the standard constants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// A generator seeded with `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform binary64 in `[0, 1)` from the top 53 bits.
    pub fn next_f64(&mut self) -> f64 {
        let v = (self.next_u64() >> 11) as f64;
        v * (1.0 / 9_007_199_254_740_992.0)
    }

    /// Uniform integer in `[0, n)` (`n > 0`; modulo bias is irrelevant here).
    pub fn below(&mut self, n: u64) -> u64 {
        let divisor = n.max(1);
        self.next_u64().checked_rem(divisor).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_vector() {
        // Published SplitMix64 outputs for seed 1234567.
        let mut r = SplitMix64::new(1_234_567);
        assert_eq!(r.next_u64(), 6_457_827_717_110_365_317);
        assert_eq!(r.next_u64(), 3_203_168_211_198_807_973);
    }
}
