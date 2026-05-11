//! Verus specs for federation gossip message classifiers and size gates.
//!
//! Production gossip messages live in `src/gossip/messages.rs`; these specs model
//! the pure variant classification and serialized-size admission logic while
//! leaving postcard and Ed25519 as runtime/trusted shell boundaries.

use vstd::prelude::*;

verus! {

pub const GOSSIP_MAX_MESSAGE_SIZE: usize = 4096;
pub const GOSSIP_KIND_CLUSTER_ONLINE: u8 = 0;
pub const GOSSIP_KIND_RESOURCE_SEEDING: u8 = 1;
pub const GOSSIP_KIND_TOKEN_REVOKED: u8 = 2;
pub const GOSSIP_KIND_RESOURCE_UPDATE: u8 = 3;

pub enum GossipKeySourceSpec {
    ClusterKey,
    Revoker,
}

pub open spec fn is_known_gossip_kind(kind: u8) -> bool {
    kind == GOSSIP_KIND_CLUSTER_ONLINE
        || kind == GOSSIP_KIND_RESOURCE_SEEDING
        || kind == GOSSIP_KIND_TOKEN_REVOKED
        || kind == GOSSIP_KIND_RESOURCE_UPDATE
}

pub open spec fn gossip_cluster_key_source_spec(kind: u8) -> Option<GossipKeySourceSpec> {
    if kind == GOSSIP_KIND_CLUSTER_ONLINE
        || kind == GOSSIP_KIND_RESOURCE_SEEDING
        || kind == GOSSIP_KIND_RESOURCE_UPDATE
    {
        Some(GossipKeySourceSpec::ClusterKey)
    } else if kind == GOSSIP_KIND_TOKEN_REVOKED {
        Some(GossipKeySourceSpec::Revoker)
    } else {
        None::<GossipKeySourceSpec>
    }
}

pub open spec fn gossip_has_hlc_timestamp_spec(kind: u8) -> bool {
    kind == GOSSIP_KIND_CLUSTER_ONLINE
        || kind == GOSSIP_KIND_RESOURCE_SEEDING
        || kind == GOSSIP_KIND_RESOURCE_UPDATE
}

pub open spec fn gossip_uses_unix_timestamp_spec(kind: u8) -> bool {
    kind == GOSSIP_KIND_TOKEN_REVOKED
}

pub open spec fn gossip_decode_size_allowed_spec(len: nat) -> bool {
    len <= GOSSIP_MAX_MESSAGE_SIZE as nat
}

pub open spec fn gossip_encode_size_allowed_spec(len: nat) -> bool {
    len <= GOSSIP_MAX_MESSAGE_SIZE as nat
}

pub open spec fn gossip_from_bytes_admits_spec(len: nat, postcard_decodes: bool) -> bool {
    gossip_decode_size_allowed_spec(len) && postcard_decodes
}

pub open spec fn gossip_to_bytes_admits_spec(len: nat) -> bool {
    gossip_encode_size_allowed_spec(len)
}

pub fn is_known_gossip_kind_exec(kind: u8) -> (known: bool)
    ensures known == is_known_gossip_kind(kind)
{
    kind == GOSSIP_KIND_CLUSTER_ONLINE
        || kind == GOSSIP_KIND_RESOURCE_SEEDING
        || kind == GOSSIP_KIND_TOKEN_REVOKED
        || kind == GOSSIP_KIND_RESOURCE_UPDATE
}

pub fn gossip_has_hlc_timestamp_exec(kind: u8) -> (has_hlc: bool)
    ensures has_hlc == gossip_has_hlc_timestamp_spec(kind)
{
    kind == GOSSIP_KIND_CLUSTER_ONLINE
        || kind == GOSSIP_KIND_RESOURCE_SEEDING
        || kind == GOSSIP_KIND_RESOURCE_UPDATE
}

pub fn gossip_uses_unix_timestamp_exec(kind: u8) -> (uses_unix: bool)
    ensures uses_unix == gossip_uses_unix_timestamp_spec(kind)
{
    kind == GOSSIP_KIND_TOKEN_REVOKED
}

pub fn gossip_decode_size_allowed_exec(len: usize) -> (allowed: bool)
    ensures allowed == gossip_decode_size_allowed_spec(len as nat)
{
    len <= GOSSIP_MAX_MESSAGE_SIZE
}

pub fn gossip_encode_size_allowed_exec(len: usize) -> (allowed: bool)
    ensures allowed == gossip_encode_size_allowed_spec(len as nat)
{
    len <= GOSSIP_MAX_MESSAGE_SIZE
}

pub fn gossip_from_bytes_admits_exec(len: usize, postcard_decodes: bool) -> (admitted: bool)
    ensures admitted == gossip_from_bytes_admits_spec(len as nat, postcard_decodes)
{
    len <= GOSSIP_MAX_MESSAGE_SIZE && postcard_decodes
}

pub fn gossip_to_bytes_admits_exec(len: usize) -> (admitted: bool)
    ensures admitted == gossip_to_bytes_admits_spec(len as nat)
{
    len <= GOSSIP_MAX_MESSAGE_SIZE
}

pub proof fn known_gossip_kinds_have_key_source(kind: u8)
    requires is_known_gossip_kind(kind)
    ensures gossip_cluster_key_source_spec(kind).is_some()
{
}

pub proof fn unknown_gossip_kinds_have_no_key_source(kind: u8)
    requires !is_known_gossip_kind(kind)
    ensures gossip_cluster_key_source_spec(kind) == None::<GossipKeySourceSpec>
{
}

pub proof fn token_revoked_uses_revoker_and_no_hlc()
    ensures
        gossip_cluster_key_source_spec(GOSSIP_KIND_TOKEN_REVOKED) == Some(GossipKeySourceSpec::Revoker),
        !gossip_has_hlc_timestamp_spec(GOSSIP_KIND_TOKEN_REVOKED),
        gossip_uses_unix_timestamp_spec(GOSSIP_KIND_TOKEN_REVOKED),
{
}

pub proof fn content_messages_use_cluster_key_and_hlc(kind: u8)
    requires
        kind == GOSSIP_KIND_CLUSTER_ONLINE
            || kind == GOSSIP_KIND_RESOURCE_SEEDING
            || kind == GOSSIP_KIND_RESOURCE_UPDATE,
    ensures
        gossip_cluster_key_source_spec(kind) == Some(GossipKeySourceSpec::ClusterKey),
        gossip_has_hlc_timestamp_spec(kind),
        !gossip_uses_unix_timestamp_spec(kind),
{
}

pub proof fn gossip_decode_rejects_oversize(len: nat)
    requires len > GOSSIP_MAX_MESSAGE_SIZE as nat
    ensures !gossip_decode_size_allowed_spec(len)
{
}

pub proof fn gossip_decode_accepts_limit()
    ensures
        gossip_decode_size_allowed_spec(GOSSIP_MAX_MESSAGE_SIZE as nat),
        gossip_encode_size_allowed_spec(GOSSIP_MAX_MESSAGE_SIZE as nat),
{
}

pub proof fn gossip_decode_accepts_empty()
    ensures
        gossip_decode_size_allowed_spec(0),
        gossip_encode_size_allowed_spec(0),
{
}

pub proof fn gossip_from_bytes_rejects_oversize_even_if_postcard_decodes(len: nat)
    requires len > GOSSIP_MAX_MESSAGE_SIZE as nat
    ensures !gossip_from_bytes_admits_spec(len, true)
{
}

pub proof fn gossip_from_bytes_requires_postcard_success(len: nat)
    ensures !gossip_from_bytes_admits_spec(len, false)
{
}

pub proof fn gossip_from_bytes_admits_when_size_and_postcard_ok(len: nat)
    requires len <= GOSSIP_MAX_MESSAGE_SIZE as nat
    ensures gossip_from_bytes_admits_spec(len, true)
{
}

pub proof fn gossip_to_bytes_rejects_oversize(len: nat)
    requires len > GOSSIP_MAX_MESSAGE_SIZE as nat
    ensures !gossip_to_bytes_admits_spec(len)
{
}

pub proof fn gossip_kind_timestamp_partition(kind: u8)
    ensures !(gossip_has_hlc_timestamp_spec(kind) && gossip_uses_unix_timestamp_spec(kind))
{
}

} // verus!
