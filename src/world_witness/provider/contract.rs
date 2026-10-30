//! Reviewed external highest-state provider contract selected for the strong witness profile.
//! The revision names the immutable published release-channel source compiled into this consumer.

/// Published `git@github.com:OnixResearch/release-channel.git` commit pinned in Cargo.toml.
pub const SOURCE_REVISION: &str = "fa389e19d5c9bd84cd75df914cc5418c70d92bfb";
pub const CONTRACT_ID: &str = "release-channel-index-v1";
pub const LICENSE: &str = "MIT";

/// Binds an independently installed provider key to the reviewed contract; it grants no authority.
pub fn reviewed(key: &[u8; 32]) -> molten_core::world_witness::ProviderContract {
    molten_core::world_witness::ProviderContract {
        provider_id: super::identity(key),
        contract_id: CONTRACT_ID.to_owned(),
        source_revision: SOURCE_REVISION.to_owned(),
        license: LICENSE.to_owned(),
    }
}
