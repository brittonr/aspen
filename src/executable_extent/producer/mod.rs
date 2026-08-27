//! Exact Mantle producer bundle and receipt admission.

pub(crate) mod admission;
mod conformance;
pub(crate) mod model;
pub(crate) mod review;

/// Bounded producer record admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Json,
    Schema,
    Profile,
    Shape,
    Digest,
    Identity,
    Linkage,
    Publication,
    Conformance,
    DetachedReview,
}

pub(crate) type Bundle = model::Bundle;
pub(crate) type Member = model::Member;
pub(crate) type Receipt = model::Receipt;

pub(crate) fn decode_bundle(bytes: &[u8]) -> Result<Bundle, Error> {
    admission::decode_bundle(bytes)
}

pub(crate) fn decode_receipt(bytes: &[u8], bundle: &Bundle) -> Result<Receipt, Error> {
    admission::decode_receipt(bytes, bundle)
}

pub(crate) fn decode_digest(text: &str) -> Result<[u8; blake3::OUT_LEN], Error> {
    admission::decode_digest(text)
}

pub(crate) fn encode_digest(identity: &[u8; blake3::OUT_LEN]) -> String {
    admission::encode_digest(identity)
}

pub(crate) fn detached_review(bundle: &Bundle) -> Result<(), Error> {
    review::detached(bundle)
}
