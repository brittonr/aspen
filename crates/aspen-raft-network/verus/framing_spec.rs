//! Verus specs for raft-network shard framing and connection classifiers.

use vstd::prelude::*;

verus! {

pub const SHARD_PREFIX_SIZE: usize = 4;
pub const STATUS_CONNECTED: u8 = 1;
pub const STATUS_DISCONNECTED: u8 = 0;
pub const FAILURE_HEALTHY: u8 = 0;
pub const FAILURE_ACTOR_CRASH: u8 = 1;
pub const FAILURE_NODE_CRASH: u8 = 2;
pub const RESPONSE_FATAL: u8 = 1;

pub open spec fn byte0(id: u32) -> u8 {
    ((id as u64) / 16777216) as u8
}

pub open spec fn byte1(id: u32) -> u8 {
    (((id as u64) / 65536) % 256) as u8
}

pub open spec fn byte2(id: u32) -> u8 {
    (((id as u64) / 256) % 256) as u8
}

pub open spec fn byte3(id: u32) -> u8 {
    ((id as u64) % 256) as u8
}

pub open spec fn shard_prefix_spec(id: u32) -> Seq<u8> {
    seq![byte0(id), byte1(id), byte2(id), byte3(id)]
}

pub open spec fn decode_shard_prefix_spec(bytes: Seq<u8>) -> u32
    recommends bytes.len() >= SHARD_PREFIX_SIZE as nat
{
    (((bytes[0] as u32) << 24) | ((bytes[1] as u32) << 16) | ((bytes[2] as u32) << 8) | (bytes[3] as u32))
}

pub open spec fn try_decode_shard_prefix_spec(bytes: Seq<u8>) -> Option<u32> {
    if bytes.len() < SHARD_PREFIX_SIZE as nat {
        None::<u32>
    } else {
        Some(decode_shard_prefix_spec(bytes))
    }
}

pub open spec fn has_prefix_spec(bytes: Seq<u8>, expected: u32) -> bool {
    bytes.len() >= SHARD_PREFIX_SIZE as nat && decode_shard_prefix_spec(bytes) == expected
}

pub open spec fn extract_sharded_response_ok_spec(bytes: Seq<u8>, expected: Option<u32>) -> bool {
    match expected {
        None => true,
        Some(id) => has_prefix_spec(bytes, id),
    }
}

pub open spec fn extracted_payload_spec(bytes: Seq<u8>, expected: Option<u32>) -> Seq<u8>
    recommends extract_sharded_response_ok_spec(bytes, expected)
{
    match expected {
        None => bytes,
        Some(_) => bytes.subrange(SHARD_PREFIX_SIZE as int, bytes.len() as int),
    }
}

pub open spec fn maybe_prefixed_len_spec(message_len: nat, shard_id: Option<u32>) -> nat {
    match shard_id {
        None => message_len,
        Some(_) => message_len + SHARD_PREFIX_SIZE as nat,
    }
}

pub open spec fn maybe_prefixed_spec(message: Seq<u8>, shard_id: Option<u32>) -> Seq<u8> {
    match shard_id {
        None => message,
        Some(id) => shard_prefix_spec(id) + message,
    }
}

pub open spec fn response_health_spec(response_kind: u8) -> (u8, u8) {
    if response_kind == RESPONSE_FATAL {
        (STATUS_DISCONNECTED, STATUS_CONNECTED)
    } else {
        (STATUS_CONNECTED, STATUS_CONNECTED)
    }
}

pub open spec fn classify_node_failure_spec(raft_status: u8, iroh_status: u8) -> u8 {
    if raft_status == STATUS_CONNECTED {
        FAILURE_HEALTHY
    } else if iroh_status == STATUS_CONNECTED {
        FAILURE_ACTOR_CRASH
    } else {
        FAILURE_NODE_CRASH
    }
}

pub fn encode_shard_prefix_exec(id: u32) -> (prefix: [u8; 4])
    ensures
        prefix@ == shard_prefix_spec(id),
        prefix@.len() == SHARD_PREFIX_SIZE as nat,
{
    let raw = id as u64;
    [
        (raw / 16777216) as u8,
        ((raw / 65536) % 256) as u8,
        ((raw / 256) % 256) as u8,
        (raw % 256) as u8,
    ]
}

pub fn try_decode_len_exec(len: usize) -> (ok: bool)
    ensures ok == (len as nat >= SHARD_PREFIX_SIZE as nat)
{
    len >= 4
}

pub fn extract_sharded_response_ok_exec(len: usize, decoded_matches: bool, has_expected_shard: bool) -> (ok: bool)
    ensures
        ok == if has_expected_shard { len as nat >= SHARD_PREFIX_SIZE as nat && decoded_matches } else { true },
{
    if has_expected_shard {
        len >= 4 && decoded_matches
    } else {
        true
    }
}

pub fn maybe_prefixed_len_exec(message_len: usize, has_shard: bool) -> (len: usize)
    requires has_shard ==> message_len <= usize::MAX - 4
    ensures len as nat == if has_shard { message_len as nat + SHARD_PREFIX_SIZE as nat } else { message_len as nat }
{
    if has_shard {
        message_len + 4
    } else {
        message_len
    }
}

pub fn classify_response_health_exec(response_kind: u8) -> (statuses: (u8, u8))
    ensures statuses == response_health_spec(response_kind)
{
    if response_kind == RESPONSE_FATAL {
        (STATUS_DISCONNECTED, STATUS_CONNECTED)
    } else {
        (STATUS_CONNECTED, STATUS_CONNECTED)
    }
}

pub fn classify_node_failure_exec(raft_status: u8, iroh_status: u8) -> (failure: u8)
    ensures failure == classify_node_failure_spec(raft_status, iroh_status)
{
    if raft_status == STATUS_CONNECTED {
        FAILURE_HEALTHY
    } else if iroh_status == STATUS_CONNECTED {
        FAILURE_ACTOR_CRASH
    } else {
        FAILURE_NODE_CRASH
    }
}

pub proof fn shard_prefix_has_fixed_width(id: u32)
    ensures shard_prefix_spec(id).len() == SHARD_PREFIX_SIZE as nat
{
}

pub proof fn try_decode_rejects_short(bytes: Seq<u8>)
    requires bytes.len() < SHARD_PREFIX_SIZE as nat
    ensures try_decode_shard_prefix_spec(bytes) == None::<u32>
{
}

pub proof fn try_decode_accepts_min_width(bytes: Seq<u8>)
    requires bytes.len() >= SHARD_PREFIX_SIZE as nat
    ensures try_decode_shard_prefix_spec(bytes).is_some()
{
}

pub proof fn unsharded_extract_always_ok(bytes: Seq<u8>)
    ensures
        extract_sharded_response_ok_spec(bytes, None::<u32>),
        extracted_payload_spec(bytes, None::<u32>) == bytes,
{
}

pub proof fn sharded_extract_rejects_short(bytes: Seq<u8>, expected: u32)
    requires bytes.len() < SHARD_PREFIX_SIZE as nat
    ensures !extract_sharded_response_ok_spec(bytes, Some(expected))
{
}

pub proof fn sharded_extract_payload_drops_prefix(bytes: Seq<u8>, expected: u32)
    requires extract_sharded_response_ok_spec(bytes, Some(expected))
    ensures
        extracted_payload_spec(bytes, Some(expected)).len() + SHARD_PREFIX_SIZE as nat == bytes.len(),
        extracted_payload_spec(bytes, Some(expected)) == bytes.subrange(SHARD_PREFIX_SIZE as int, bytes.len() as int),
{
}

pub proof fn maybe_prefix_without_shard_identity(message: Seq<u8>)
    ensures
        maybe_prefixed_spec(message, None::<u32>) == message,
        maybe_prefixed_len_spec(message.len(), None::<u32>) == message.len(),
{
}

pub proof fn maybe_prefix_with_shard_extends_by_four(message: Seq<u8>, id: u32)
    ensures
        maybe_prefixed_spec(message, Some(id)).len() == message.len() + SHARD_PREFIX_SIZE as nat,
        maybe_prefixed_len_spec(message.len(), Some(id)) == message.len() + SHARD_PREFIX_SIZE as nat,
        maybe_prefixed_spec(message, Some(id)).subrange(0, SHARD_PREFIX_SIZE as int) == shard_prefix_spec(id),
{
}

pub proof fn fatal_response_marks_only_raft_disconnected()
    ensures response_health_spec(RESPONSE_FATAL) == (STATUS_DISCONNECTED, STATUS_CONNECTED)
{
}

pub proof fn nonfatal_response_marks_both_connected(kind: u8)
    requires kind != RESPONSE_FATAL
    ensures response_health_spec(kind) == (STATUS_CONNECTED, STATUS_CONNECTED)
{
}

pub proof fn node_failure_truth_table()
    ensures
        classify_node_failure_spec(STATUS_CONNECTED, STATUS_CONNECTED) == FAILURE_HEALTHY,
        classify_node_failure_spec(STATUS_CONNECTED, STATUS_DISCONNECTED) == FAILURE_HEALTHY,
        classify_node_failure_spec(STATUS_DISCONNECTED, STATUS_CONNECTED) == FAILURE_ACTOR_CRASH,
        classify_node_failure_spec(STATUS_DISCONNECTED, STATUS_DISCONNECTED) == FAILURE_NODE_CRASH,
{
}

} // verus!
