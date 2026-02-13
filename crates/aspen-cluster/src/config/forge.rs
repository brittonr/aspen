//! Forge decentralized git configuration.
//!
//! Controls the Forge subsystem which provides decentralized Git hosting.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Forge decentralized git configuration.
///
/// Controls the Forge subsystem which provides decentralized Git hosting
/// via iroh-blobs for object storage and Raft KV for ref storage.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct ForgeConfig {
    /// Enable Forge gossip announcements for ref updates.
    ///
    /// When enabled, ref updates are broadcast via iroh-gossip to other nodes.
    /// This enables automatic CI triggering and repository synchronization.
    ///
    /// Requires the `forge` feature and `iroh.enable_gossip = true`.
    ///
    /// Default: false
    #[serde(default)]
    pub enable_gossip: bool,
}
