//! Versioned deterministic 64-bit fold over canonical encodings.
//!
//! A value produced here is an **integrity index, not proof validity**
//! (VC-ID-03). Agreement between two digests means the two canonical
//! encodings are presumed identical; it never asserts that an accepted claim
//! is true. Validity is established only by an `Admission`, `replay`, or
//! `release` decision that regenerates the digest from retained trusted data
//! rather than from producer-supplied text.
//!
//! The fold is a bounded, non-recursive byte fold: every loop is capped by
//! the caller's already-validated slice length, and no path allocates.

/// Canonical digest-encoding revision. Bumping it changes every digest and so
/// invalidates every retained applicability decision that compares digests.
pub const DIGEST_VERSION: u32 = 1;

/// Domain separators. Distinct subjects fold into distinct domains so a
/// statement digest can never collide with a subject identity by accident.
pub(crate) const DOMAIN_STATEMENT: u64 = 0x53_54_41_54_45_4d_4e_54;
pub(crate) const DOMAIN_SUBJECT: u64 = 0x53_55_42_4a_45_43_54_00;
pub(crate) const DOMAIN_COMPOSE: u64 = 0x43_4f_4d_50_4f_53_45_00;

pub(crate) const DOMAIN_TEMPLATE: u64 = 0x54_45_4d_50_4c_41_54_45;
pub(crate) const DOMAIN_GUARD: u64 = 0x47_55_41_52_44_00_00_00;

/// Domain for a typed-chunk envelope's canonical body fold. The typed-chunk
/// library recomputes every chunk identity and source revision in this domain
/// from retained bytes; the value is a lookup aid, never admission authority
/// (TCHUNK-01).
pub(crate) const DOMAIN_CHUNK: u64 = 0x43_48_55_4e_4b_00_00_01;

/// Domain for a typed-chunk emitted-source fold, kept distinct from the
/// envelope domain so an identity can never collide with a source revision.
pub(crate) const DOMAIN_CHUNK_SOURCE: u64 = 0x43_48_55_4e_4b_00_00_02;

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// An incremental fold over canonical encodings.
#[derive(Clone, Copy)]
pub(crate) struct Fold {
    state: u64,
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; folds mutate only fresh owned digest scratch, never the borrowed canonical input or host state"
)]
impl Fold {
    /// Start a fold in `domain` at the current canonical encoding revision.
    pub(crate) fn new(domain: u64) -> Self {
        let mut fold = Fold { state: OFFSET };
        fold.absorb(domain);
        fold.absorb(u64::from(DIGEST_VERSION));
        fold
    }

    /// Absorb one canonical integer in little-endian byte order.
    pub(crate) fn absorb(&mut self, value: u64) {
        let mut shift = 0u32;
        let mut index = 0u32;
        while index < 8 {
            let byte = (value >> shift) & 0xff;
            self.state ^= byte;
            self.state = self.state.wrapping_mul(PRIME);
            shift = shift.saturating_add(8);
            index = index.saturating_add(1);
        }
    }

    // Supported 64-bit host and 32-bit Wasm lengths widen exactly to u64.
    pub(crate) fn absorb_count(&mut self, count: usize) {
        self.absorb(count as u64);
    }

    /// Absorb a length-prefixed byte string. The caller guarantees `bytes` is
    /// already bounded by the retained-data limits; the loop is a plain
    /// bounded scan and never allocates.
    pub(crate) fn absorb_bytes(&mut self, bytes: &[u8]) {
        self.absorb_count(bytes.len());
        let mut index = 0usize;
        while index < bytes.len() {
            self.state ^= u64::from(bytes[index]);
            self.state = self.state.wrapping_mul(PRIME);
            index = index.saturating_add(1);
        }
    }

    /// Finalize with a fixed avalanche so short encodings still spread.
    pub(crate) fn finish(self) -> u64 {
        let mut state = self.state;
        state ^= state >> 33;
        state = state.wrapping_mul(0xff51_afd7_ed55_8ccd);
        state ^= state >> 33;
        state = state.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        state ^= state >> 33;
        state
    }
}
