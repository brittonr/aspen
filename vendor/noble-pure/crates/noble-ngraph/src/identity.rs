//! Injected identity hashing (design D2).
//!
//! The library builds every identity as a domain-separated, length-prefixed
//! byte stream and feeds it to a scheme supplied by the composition root. It
//! never names or implements a hash function itself, so it has no
//! third-party dependency. `noble-cli` and `noble-ngraph-experiment`
//! implement the scheme with BLAKE3; a hash identifies bytes and proves
//! nothing.

/// One running digest computation.
pub trait IdentityHasher {
    /// Appends `bytes` to the digest input.
    fn update(&mut self, bytes: &[u8]);
    /// Finishes the digest.
    fn finish(self: alloc::boxed::Box<Self>) -> crate::ir::Digest;
}

/// A 32-byte identity hash scheme supplied by the composition root.
pub trait IdentityScheme {
    /// Stable scheme name recorded in reports (for example `blake3`).
    fn name(&self) -> &'static str;
    /// Starts a new digest.
    fn start(&self) -> alloc::boxed::Box<dyn IdentityHasher + '_>;
}
