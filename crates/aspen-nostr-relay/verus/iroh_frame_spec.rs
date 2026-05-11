//! Verus specs for Nostr-over-Iroh length-prefixed frame admission.
//!
//! Production `src/iroh_transport.rs::{write_frame,read_frame}` performs async
//! stream I/O. This module verifies the deterministic scalar contract around
//! that shell: 4-byte big-endian prefix shape, payload-size admission, oversize
//! rejection before body allocation, clean early EOF classification, and total
//! frame length arithmetic.

use vstd::prelude::*;

verus! {

pub const LENGTH_PREFIX_BYTES: u32 = 4;
pub const MAX_EVENT_SIZE: u32 = 64 * 1024;
pub const MAX_IROH_CONNECTIONS: u32 = 128;

pub enum PrefixReadOutcome {
    Complete,
    FinishedEarly,
    IoError,
}

pub enum FrameReadAction {
    ReadBody,
    CleanEof,
    RejectOversize,
    PropagateError,
}

pub open spec fn frame_len_admitted(len: u32) -> bool {
    len <= MAX_EVENT_SIZE
}

pub open spec fn write_len_representable(payload_len: u64) -> bool {
    payload_len <= u32::MAX as u64
}

pub open spec fn write_len_admitted(payload_len: u64) -> bool {
    write_len_representable(payload_len) && payload_len <= MAX_EVENT_SIZE as u64
}

pub open spec fn frame_total_len(payload_len: u32) -> u32 {
    (LENGTH_PREFIX_BYTES + payload_len) as u32
}

pub open spec fn encoded_prefix_len(payload_len: u32) -> u32 {
    LENGTH_PREFIX_BYTES
}

pub open spec fn decode_prefix_value(b0: u32, b1: u32, b2: u32, b3: u32) -> u32
    recommends b0 <= 255, b1 <= 255, b2 <= 255, b3 <= 255
{
    ((b0 * 16777216 + b1 * 65536 + b2 * 256 + b3) as u32)
}

pub open spec fn prefix_byte_bounded(byte: u32) -> bool {
    byte <= 255
}

pub open spec fn read_action(prefix: PrefixReadOutcome, len: u32) -> FrameReadAction {
    match prefix {
        PrefixReadOutcome::Complete => if frame_len_admitted(len) {
            FrameReadAction::ReadBody
        } else {
            FrameReadAction::RejectOversize
        },
        PrefixReadOutcome::FinishedEarly => FrameReadAction::CleanEof,
        PrefixReadOutcome::IoError => FrameReadAction::PropagateError,
    }
}

pub open spec fn alloc_len_after_admission(len: u32) -> Option<u32> {
    if frame_len_admitted(len) { Some(len) } else { None::<u32> }
}

pub open spec fn connection_limit_positive(limit: u32) -> bool {
    limit > 0
}

pub fn frame_len_admitted_exec(len: u32) -> (admitted: bool)
    ensures admitted == frame_len_admitted(len)
{
    len <= MAX_EVENT_SIZE
}

pub fn write_len_admitted_exec(payload_len: u64) -> (admitted: bool)
    ensures admitted == write_len_admitted(payload_len)
{
    payload_len <= u32::MAX as u64 && payload_len <= MAX_EVENT_SIZE as u64
}

pub fn read_action_exec(prefix_complete: bool, finished_early: bool, len: u32) -> (action: FrameReadAction)
    requires !(prefix_complete && finished_early)
    ensures action == read_action(
        if prefix_complete {
            PrefixReadOutcome::Complete
        } else if finished_early {
            PrefixReadOutcome::FinishedEarly
        } else {
            PrefixReadOutcome::IoError
        },
        len,
    )
{
    if prefix_complete {
        if len <= MAX_EVENT_SIZE {
            FrameReadAction::ReadBody
        } else {
            FrameReadAction::RejectOversize
        }
    } else if finished_early {
        FrameReadAction::CleanEof
    } else {
        FrameReadAction::PropagateError
    }
}

pub fn alloc_len_after_admission_exec(len: u32) -> (alloc: Option<u32>)
    ensures alloc == alloc_len_after_admission(len)
{
    if len <= MAX_EVENT_SIZE { Some(len) } else { None }
}

pub proof fn max_event_size_is_positive_and_representable()
    ensures
        MAX_EVENT_SIZE > 0,
        MAX_EVENT_SIZE <= u32::MAX,
        write_len_representable(MAX_EVENT_SIZE as u64),
{
}

pub proof fn connection_limit_is_positive()
    ensures connection_limit_positive(MAX_IROH_CONNECTIONS)
{
}

pub proof fn admitted_read_lengths_allocate_exact_body(len: u32)
    requires frame_len_admitted(len)
    ensures alloc_len_after_admission(len) == Some(len)
{
}

pub proof fn oversize_read_lengths_do_not_allocate_body(len: u32)
    requires !frame_len_admitted(len)
    ensures alloc_len_after_admission(len) == None::<u32>
{
}

pub proof fn read_complete_admitted_reads_body(len: u32)
    requires frame_len_admitted(len)
    ensures read_action(PrefixReadOutcome::Complete, len) == FrameReadAction::ReadBody
{
}

pub proof fn read_complete_oversize_rejects_before_body(len: u32)
    requires !frame_len_admitted(len)
    ensures read_action(PrefixReadOutcome::Complete, len) == FrameReadAction::RejectOversize
{
}

pub proof fn finished_early_prefix_is_clean_eof(len: u32)
    ensures read_action(PrefixReadOutcome::FinishedEarly, len) == FrameReadAction::CleanEof
{
}

pub proof fn prefix_io_error_is_propagated(len: u32)
    ensures read_action(PrefixReadOutcome::IoError, len) == FrameReadAction::PropagateError
{
}

pub proof fn write_admitted_implies_read_admitted(payload_len: u64)
    requires write_len_admitted(payload_len)
    ensures payload_len <= MAX_EVENT_SIZE as u64
{
}

pub proof fn max_sized_payload_is_admitted_for_write()
    ensures write_len_admitted(MAX_EVENT_SIZE as u64)
{
}

pub proof fn one_byte_oversize_payload_is_rejected_for_write()
    ensures !write_len_admitted((MAX_EVENT_SIZE + 1) as u64)
{
}

pub proof fn one_byte_oversize_read_is_rejected()
    ensures !frame_len_admitted((MAX_EVENT_SIZE + 1) as u32)
{
}

pub proof fn encoded_prefix_is_always_four_bytes(payload_len: u32)
    ensures encoded_prefix_len(payload_len) == LENGTH_PREFIX_BYTES
{
}

pub proof fn admitted_frame_total_len_extends_payload(payload_len: u32)
    requires payload_len <= MAX_EVENT_SIZE
    ensures frame_total_len(payload_len) == payload_len + LENGTH_PREFIX_BYTES
{
}

pub proof fn zero_payload_frame_has_prefix_only()
    ensures frame_total_len(0) == LENGTH_PREFIX_BYTES
{
}

pub proof fn prefix_bytes_are_bounded(b0: u32, b1: u32, b2: u32, b3: u32)
    requires b0 <= 255, b1 <= 255, b2 <= 255, b3 <= 255
    ensures
        prefix_byte_bounded(b0),
        prefix_byte_bounded(b1),
        prefix_byte_bounded(b2),
        prefix_byte_bounded(b3),
{
}

pub proof fn decoded_prefix_from_bytes_is_representable(b0: u32, b1: u32, b2: u32, b3: u32)
    requires b0 <= 255, b1 <= 255, b2 <= 255, b3 <= 255
    ensures decode_prefix_value(b0, b1, b2, b3) <= u32::MAX
{
}

} // verus!
