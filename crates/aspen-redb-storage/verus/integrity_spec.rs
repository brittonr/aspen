//! Verus specifications for integrity helper structural facts.
//!
//! Blake3 and hex decoding stay runtime/trusted boundaries; this module proves
//! Aspen's fixed-width hash shape, default chain-tip shape, constant-iteration
//! comparison semantics, and snapshot verification boolean composition.

use vstd::prelude::*;

verus! {

pub const CHAIN_HASH_LEN: usize = 32;

pub open spec fn genesis_hash_spec() -> Seq<u8> {
    Seq::new(CHAIN_HASH_LEN as nat, |i: int| 0u8)
}

pub open spec fn chain_hash_shape(hash: Seq<u8>) -> bool {
    hash.len() == CHAIN_HASH_LEN as int
}

pub open spec fn hash_bytes_equal(a: Seq<u8>, b: Seq<u8>) -> bool {
    a.len() == b.len() && forall|i: int| #![auto] 0 <= i < a.len() ==> a[i] == b[i]
}

pub open spec fn decoded_hash_admission(decoded_len: nat) -> bool {
    decoded_len == CHAIN_HASH_LEN as nat
}

pub open spec fn snapshot_verify_spec(
    data_hash_matches: bool,
    meta_hash_matches: bool,
    chain_hash_matches: bool,
    expected_chain_present: bool,
) -> bool {
    data_hash_matches && meta_hash_matches && (!expected_chain_present || chain_hash_matches)
}

pub struct ChainTipStateSpec {
    pub hash: Seq<u8>,
    pub index: u64,
}

pub open spec fn default_chain_tip_spec() -> ChainTipStateSpec {
    ChainTipStateSpec { hash: genesis_hash_spec(), index: 0 }
}

pub fn constant_time_compare_bytes(a: &[u8], b: &[u8]) -> (result: bool)
    requires a@.len() == b@.len()
    ensures result == hash_bytes_equal(a@, b@)
{
    let mut mismatch = false;
    let mut index: usize = 0;
    while index < a.len()
        invariant
            a@.len() == b@.len(),
            0 <= index <= a.len(),
            mismatch == exists|seen: int| #![auto] 0 <= seen < index && a@[seen] != b@[seen],
        decreases a.len() - index
    {
        if a[index] != b[index] {
            mismatch = true;
        }
        index += 1;
    }

    assert(mismatch == exists|seen: int| #![auto] 0 <= seen < a@.len() && a@[seen] != b@[seen]);
    if mismatch {
        assert(!hash_bytes_equal(a@, b@));
        false
    } else {
        assert(forall|seen: int| #![auto] 0 <= seen < a@.len() ==> a@[seen] == b@[seen]);
        true
    }
}

pub fn constant_time_compare_chain_hash(a: &[u8], b: &[u8]) -> (result: bool)
    requires
        a@.len() == CHAIN_HASH_LEN as int,
        b@.len() == CHAIN_HASH_LEN as int,
    ensures
        result == hash_bytes_equal(a@, b@),
        chain_hash_shape(a@),
        chain_hash_shape(b@),
{
    constant_time_compare_bytes(a, b)
}

pub fn hash_from_decoded_bytes_admitted(decoded_len: u64) -> (result: bool)
    ensures result == decoded_hash_admission(decoded_len as nat)
{
    decoded_len == CHAIN_HASH_LEN as u64
}

pub fn verify_snapshot_integrity_shape(
    data_hash_matches: bool,
    meta_hash_matches: bool,
    chain_hash_matches: bool,
    expected_chain_present: bool,
) -> (result: bool)
    ensures result == snapshot_verify_spec(
        data_hash_matches,
        meta_hash_matches,
        chain_hash_matches,
        expected_chain_present,
    )
{
    let is_basic_valid = data_hash_matches && meta_hash_matches;
    if expected_chain_present {
        is_basic_valid && chain_hash_matches
    } else {
        is_basic_valid
    }
}

pub proof fn genesis_hash_has_fixed_shape()
    ensures chain_hash_shape(genesis_hash_spec())
{
}

pub proof fn default_chain_tip_is_genesis_at_zero()
    ensures
        default_chain_tip_spec().index == 0,
        default_chain_tip_spec().hash == genesis_hash_spec(),
        chain_hash_shape(default_chain_tip_spec().hash),
{
}

pub proof fn decoded_wrong_length_rejected(decoded_len: nat)
    requires decoded_len != CHAIN_HASH_LEN as nat
    ensures !decoded_hash_admission(decoded_len)
{
}

pub proof fn decoded_exact_length_admitted()
    ensures decoded_hash_admission(CHAIN_HASH_LEN as nat)
{
}

pub proof fn snapshot_without_chain_requires_basic_hashes(
    data_hash_matches: bool,
    meta_hash_matches: bool,
    chain_hash_matches: bool,
)
    ensures snapshot_verify_spec(data_hash_matches, meta_hash_matches, chain_hash_matches, false)
        == (data_hash_matches && meta_hash_matches)
{
}

pub proof fn snapshot_with_chain_requires_all_hashes(
    data_hash_matches: bool,
    meta_hash_matches: bool,
    chain_hash_matches: bool,
)
    ensures snapshot_verify_spec(data_hash_matches, meta_hash_matches, chain_hash_matches, true)
        == (data_hash_matches && meta_hash_matches && chain_hash_matches)
{
}

} // verus!
