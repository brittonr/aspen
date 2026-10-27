//! Product-owned carrier contract. Decoding never authorizes an effect.

mod bounds;
mod declaration;
pub mod host;
mod intake;
mod reply;

pub use declaration::host_world;
pub use intake::decode_host_request;
pub use intake::encode_host_request;
pub use reply::decode_host_response;
pub use reply::encode_host_response;

pub const HOST_PROFILE_ID: &str = "molten.wasm.component.effects.v1";
pub const HOST_WIT_PACKAGE: &str = "molten:component-effects@1.0.0";
pub const HOST_WIT_WORLD: &str = "actor-effects";
pub const HOST_WIT_SOURCE_REF: &str = "blake3:9478f7ddd9380bcbe898f2101d615674067e0f167ddc6f0fc345553751b5eaa2";
pub const HOST_REQUEST_IMPORT: &str = "effect-request";
pub const HOST_REQUEST_SCHEMA: &str = "molten.component-effect-request.v1";
pub const HOST_RESPONSE_SCHEMA: &str = "molten.component-effect-response.v1";
pub const HOST_PAYLOAD_BYTES: u64 = 65_536;
pub const HOST_MAX_DEPTH: u64 = 32;
pub const HOST_MAX_NODES: u64 = 4_096;
pub const HOST_BLOB_CAPABILITY: &str = "hostcall:blob-get";

/// Guest assertions, not an admission or a grant of current authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRequest {
    pub(super) generation: u64,
    pub(super) incarnation: u64,
    pub(super) sequence: u64,
    pub(super) grant: super::admission::ImportGrant,
    pub(super) effect: crate::effects::EffectRequest,
    pub(super) payload: preserves::IOValue,
    pub(super) request_ref: String,
}

impl HostRequest {
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    pub const fn incarnation(&self) -> u64 {
        self.incarnation
    }
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    pub const fn grant(&self) -> &super::admission::ImportGrant {
        &self.grant
    }
    pub const fn effect(&self) -> &crate::effects::EffectRequest {
        &self.effect
    }
    pub const fn payload(&self) -> &preserves::IOValue {
        &self.payload
    }
    pub fn request_ref(&self) -> &str {
        &self.request_ref
    }
}

pub struct HostRequestInput<'a> {
    pub generation: u64,
    pub incarnation: u64,
    pub sequence: u64,
    pub grant: &'a super::admission::ImportGrant,
    pub effect: &'a preserves::IOValue,
    pub payload: &'a preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostResponse {
    pub(super) request_ref: String,
    pub(super) response: crate::effects::EffectResponse,
    pub(super) output: Option<preserves::IOValue>,
}

impl HostResponse {
    pub fn request_ref(&self) -> &str {
        &self.request_ref
    }
    pub const fn response(&self) -> &crate::effects::EffectResponse {
        &self.response
    }
    pub const fn output(&self) -> Option<&preserves::IOValue> {
        self.output.as_ref()
    }
}

pub struct HostResponseInput<'a> {
    pub request: &'a HostRequest,
    pub response: &'a preserves::IOValue,
    pub output: Option<&'a preserves::IOValue>,
}

fn invalid(message: impl Into<String>) -> super::model::Denial {
    super::model::Denial::classified(super::model::DenialClass::InvalidPreservesPayload, message)
}

fn convert(error: crate::error::Failure) -> super::model::Denial {
    invalid(error.to_string())
}

fn bounded(bytes: &[u8]) -> super::model::Result<()> {
    bounds::check(bytes)
}

#[cfg(test)]
mod tests;
