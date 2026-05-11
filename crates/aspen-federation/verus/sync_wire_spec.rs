//! Verus specs for federation sync wire length-prefix framing.
//!
//! Production framing lives in `src/sync/wire.rs`: postcard payloads are
//! guarded by a 4-byte big-endian length prefix and a 64 MiB maximum message
//! size. These specs model only the pure length/admission arithmetic and leave
//! async I/O plus postcard serialization/deserialization in the runtime shell.

use vstd::prelude::*;

verus! {

pub const SYNC_WIRE_PREFIX_LEN: usize = 4;
pub const SYNC_WIRE_MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

pub open spec fn sync_wire_size_allowed_spec(len: nat) -> bool {
    len <= SYNC_WIRE_MAX_MESSAGE_SIZE as nat
}

pub open spec fn sync_wire_read_admits_spec(prefix_read: bool, len: nat, body_read: bool, postcard_decodes: bool) -> bool {
    prefix_read && sync_wire_size_allowed_spec(len) && body_read && postcard_decodes
}

pub open spec fn sync_wire_write_admits_spec(serialized: bool, len: nat, prefix_written: bool, body_written: bool) -> bool {
    serialized && sync_wire_size_allowed_spec(len) && prefix_written && body_written
}

pub open spec fn sync_wire_frame_len_spec(body_len: nat) -> nat {
    SYNC_WIRE_PREFIX_LEN as nat + body_len
}

pub open spec fn sync_wire_can_cast_len_to_u32_spec(len: nat) -> bool {
    len <= u32::MAX as nat
}

pub open spec fn be_u32_value_spec(b0: u8, b1: u8, b2: u8, b3: u8) -> nat {
    (b0 as nat) * 16777216nat + (b1 as nat) * 65536nat + (b2 as nat) * 256nat + b3 as nat
}

pub fn sync_wire_size_allowed_exec(len: usize) -> (allowed: bool)
    ensures allowed == sync_wire_size_allowed_spec(len as nat)
{
    len <= SYNC_WIRE_MAX_MESSAGE_SIZE
}

pub fn sync_wire_read_admits_exec(prefix_read: bool, len: usize, body_read: bool, postcard_decodes: bool) -> (admitted: bool)
    ensures admitted == sync_wire_read_admits_spec(prefix_read, len as nat, body_read, postcard_decodes)
{
    prefix_read && len <= SYNC_WIRE_MAX_MESSAGE_SIZE && body_read && postcard_decodes
}

pub fn sync_wire_write_admits_exec(serialized: bool, len: usize, prefix_written: bool, body_written: bool) -> (admitted: bool)
    ensures admitted == sync_wire_write_admits_spec(serialized, len as nat, prefix_written, body_written)
{
    serialized && len <= SYNC_WIRE_MAX_MESSAGE_SIZE && prefix_written && body_written
}

pub proof fn sync_wire_accepts_empty_and_limit()
    ensures
        sync_wire_size_allowed_spec(0),
        sync_wire_size_allowed_spec(SYNC_WIRE_MAX_MESSAGE_SIZE as nat),
{
}

pub proof fn sync_wire_rejects_oversize(len: nat)
    requires len > SYNC_WIRE_MAX_MESSAGE_SIZE as nat
    ensures !sync_wire_size_allowed_spec(len)
{
}

pub proof fn sync_wire_max_fits_u32()
    ensures sync_wire_can_cast_len_to_u32_spec(SYNC_WIRE_MAX_MESSAGE_SIZE as nat)
{
}

pub proof fn sync_wire_allowed_lengths_fit_u32(len: nat)
    requires sync_wire_size_allowed_spec(len)
    ensures sync_wire_can_cast_len_to_u32_spec(len)
{
}

pub proof fn sync_wire_frame_len_includes_prefix(body_len: nat)
    ensures
        sync_wire_frame_len_spec(body_len) >= body_len,
        sync_wire_frame_len_spec(body_len) == body_len + SYNC_WIRE_PREFIX_LEN as nat,
{
}

pub proof fn sync_wire_read_rejects_oversize_even_if_io_and_postcard_succeed(len: nat)
    requires len > SYNC_WIRE_MAX_MESSAGE_SIZE as nat
    ensures !sync_wire_read_admits_spec(true, len, true, true)
{
}

pub proof fn sync_wire_read_requires_prefix(len: nat, body_read: bool, postcard_decodes: bool)
    ensures !sync_wire_read_admits_spec(false, len, body_read, postcard_decodes)
{
}

pub proof fn sync_wire_read_requires_body(len: nat, prefix_read: bool, postcard_decodes: bool)
    ensures !sync_wire_read_admits_spec(prefix_read, len, false, postcard_decodes)
{
}

pub proof fn sync_wire_read_requires_postcard_success(len: nat, prefix_read: bool, body_read: bool)
    ensures !sync_wire_read_admits_spec(prefix_read, len, body_read, false)
{
}

pub proof fn sync_wire_read_admits_when_all_guards_pass(len: nat)
    requires sync_wire_size_allowed_spec(len)
    ensures sync_wire_read_admits_spec(true, len, true, true)
{
}

pub proof fn sync_wire_write_rejects_oversize_even_if_io_succeeds(len: nat)
    requires len > SYNC_WIRE_MAX_MESSAGE_SIZE as nat
    ensures !sync_wire_write_admits_spec(true, len, true, true)
{
}

pub proof fn sync_wire_write_requires_serialization(len: nat, prefix_written: bool, body_written: bool)
    ensures !sync_wire_write_admits_spec(false, len, prefix_written, body_written)
{
}

pub proof fn sync_wire_write_requires_prefix_write(len: nat, serialized: bool, body_written: bool)
    ensures !sync_wire_write_admits_spec(serialized, len, false, body_written)
{
}

pub proof fn sync_wire_write_requires_body_write(len: nat, serialized: bool, prefix_written: bool)
    ensures !sync_wire_write_admits_spec(serialized, len, prefix_written, false)
{
}

pub proof fn sync_wire_write_admits_when_all_guards_pass(len: nat)
    requires sync_wire_size_allowed_spec(len)
    ensures sync_wire_write_admits_spec(true, len, true, true)
{
}

pub proof fn sync_wire_be_prefix_byte_bounds(b0: u8, b1: u8, b2: u8, b3: u8)
    ensures be_u32_value_spec(b0, b1, b2, b3) <= u32::MAX as nat
{
    assert(b0 as nat <= 255);
    assert(b1 as nat <= 255);
    assert(b2 as nat <= 255);
    assert(b3 as nat <= 255);
    assert(be_u32_value_spec(b0, b1, b2, b3) <= 255nat * 16777216nat + 255nat * 65536nat + 255nat * 256nat + 255nat) by(nonlinear_arith);
}

} // verus!
