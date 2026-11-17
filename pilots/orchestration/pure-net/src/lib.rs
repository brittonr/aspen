// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
//! Explicit opt-in native pure evaluation, not a sandbox or execution grant.
//! Canonical objects use the existing Preserves codec; producer-internal raw
//! byte digests are separately labelled facts, not Aspen semantic identities.
mod budget;
mod execution;
pub mod fixtures;
mod producer;
mod types;
pub use budget::Limits;
pub use execution::execute;
pub use producer::compile;
pub use producer::encode_program;
pub use types::*;

pub(crate) const MANIFEST: &str = "pure-net-eval/manifest/v1";
pub(crate) const PROGRAM: &str = "pure-net-eval/program/v1";
pub(crate) const INPUTS: &str = "pure-net-eval/ordered-i64-inputs/v1";
pub(crate) const RECEIPT: &str = "pure-net-eval/receipt/v1";
pub(crate) const MAX_BYTES: usize = 262_144;

pub(crate) fn encode<T: serde::Serialize>(domain: &str, value: &T) -> Result<Vec<u8>, Error> {
    molten_orchestrator_codec::frame(domain, value).map_err(|_| Error::Malformed)
}
pub(crate) fn decode<T: serde::Serialize + serde::de::DeserializeOwned>(
    domain: &str,
    bytes: &[u8],
    cap: u64,
) -> Result<T, Error> {
    if bytes.len() as u64 > cap || bytes.len() > MAX_BYTES {
        return Err(Error::Exhausted(Resource::Decode));
    }
    molten_orchestrator_codec::unframe(domain, bytes).map_err(|_| Error::Malformed)
}
pub(crate) fn identity<T: serde::Serialize>(domain: &str, value: &T) -> Result<molten_batch_core::Id, Error> {
    molten_orchestrator_codec::identity(domain, value).map_err(|_| Error::Malformed)
}
pub fn manifest(host: HostBindings, backend: Backend, limits: Limits) -> Result<Manifest, Error> {
    let value = Manifest {
        version: "pure-net-eval/v1".into(),
        backend,
        bindings: Bindings::exact(host),
        limits,
    };
    value.validate()?;
    Ok(value)
}
pub fn encode_manifest(value: &Manifest) -> Result<Vec<u8>, Error> {
    value.validate()?;
    encode(MANIFEST, value)
}
pub fn decode_manifest(bytes: &[u8]) -> Result<Manifest, Error> {
    let value: Manifest = decode(MANIFEST, bytes, 32_768)?;
    value.validate()?;
    Ok(value)
}
pub fn encode_inputs(values: &[i64]) -> Result<Vec<u8>, Error> {
    if values.len() > 1024 {
        return Err(Error::Exhausted(Resource::Inputs));
    }
    encode(INPUTS, &OrderedInputs {
        values: values.to_vec(),
    })
}
pub(crate) fn artifact_id(kind: molten_batch_core::ContentKind, bytes: &[u8]) -> Result<molten_batch_core::Id, Error> {
    if bytes.len() > molten_batch_core::MAX_CONTENT_BYTES {
        return Err(Error::Exhausted(Resource::Decode));
    }
    identity("content-manifest.v2", &molten_batch_core::ContentManifest {
        kind,
        raw_digest: molten_orchestrator_codec::digest(bytes),
        length: bytes.len() as u64,
    })
}
pub fn content_id_manifest(bytes: &[u8]) -> Result<molten_batch_core::Id, Error> {
    decode_manifest(bytes)?;
    artifact_id(molten_batch_core::ContentKind::PureNetManifest, bytes)
}
pub fn content_id_program(bytes: &[u8]) -> Result<molten_batch_core::Id, Error> {
    let _: MaterializedProgram = decode(PROGRAM, bytes, MAX_BYTES as u64)?;
    artifact_id(molten_batch_core::ContentKind::PureNetProgram, bytes)
}
pub fn content_id_inputs(bytes: &[u8]) -> Result<molten_batch_core::Id, Error> {
    let _: OrderedInputs = decode(INPUTS, bytes, 32_768)?;
    artifact_id(molten_batch_core::ContentKind::Data, bytes)
}
pub fn encode_receipt(value: &Receipt) -> Result<Vec<u8>, Error> {
    encode(RECEIPT, value)
}
pub fn decode_receipt(bytes: &[u8]) -> Result<Receipt, Error> {
    decode(RECEIPT, bytes, MAX_BYTES as u64)
}

#[cfg(test)]
mod tests;
