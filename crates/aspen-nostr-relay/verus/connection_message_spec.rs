//! Verus specs for Nostr connection/message admission control flow.
//!
//! Production `src/connection.rs` owns WebSocket I/O, JSON parsing, Nostr
//! signature verification, rate limiter mutation, storage, and broadcast. This
//! module verifies the pure routing kernel around those shells: text-size gates,
//! WebSocket frame actions, client-message dispatch, EVENT first-failure
//! ordering, and store/broadcast response shape.

use vstd::prelude::*;

verus! {

pub const MAX_EVENT_SIZE: u32 = 64 * 1024;

pub enum WebSocketFrameSpec {
    Text,
    Close,
    Ping,
    Other,
    Error,
    EndOfStream,
}

pub enum FrameActionSpec {
    DispatchText,
    NoticeTooLarge,
    SendPong,
    Ignore,
    Break,
}

pub enum BroadcastRecvSpec {
    Event,
    Lagged,
    Closed,
}

pub enum BroadcastActionSpec {
    PushMatchingEvent,
    ContinueAfterLag,
    Break,
}

pub enum ClientMessageKindSpec {
    Event,
    Req,
    Close,
    Auth,
    Unsupported,
}

pub enum ClientDispatchSpec {
    HandleEvent,
    HandleReq,
    HandleClose,
    HandleAuth,
    NoticeUnsupported,
}

pub enum WritePolicySpec {
    Open,
    AuthRequired,
    ReadOnly,
}

pub enum EventAdmissionSpec {
    RejectIpRateLimited,
    RejectReadOnly,
    RejectAuthRequired,
    RejectInvalidSignature,
    RejectPubkeyRateLimited,
    StoreAndRespond,
}

pub enum StoreResponseSpec {
    OkBroadcast,
    OkDuplicateNoBroadcast,
    ErrorNoBroadcast,
}

pub open spec fn text_size_admitted(text_len: u32) -> bool {
    text_len <= MAX_EVENT_SIZE
}

pub open spec fn websocket_frame_action(frame: WebSocketFrameSpec, text_len: u32) -> FrameActionSpec {
    match frame {
        WebSocketFrameSpec::Text => if text_size_admitted(text_len) {
            FrameActionSpec::DispatchText
        } else {
            FrameActionSpec::NoticeTooLarge
        },
        WebSocketFrameSpec::Close => FrameActionSpec::Break,
        WebSocketFrameSpec::EndOfStream => FrameActionSpec::Break,
        WebSocketFrameSpec::Ping => FrameActionSpec::SendPong,
        WebSocketFrameSpec::Other => FrameActionSpec::Ignore,
        WebSocketFrameSpec::Error => FrameActionSpec::Break,
    }
}

pub open spec fn broadcast_recv_action(recv: BroadcastRecvSpec) -> BroadcastActionSpec {
    match recv {
        BroadcastRecvSpec::Event => BroadcastActionSpec::PushMatchingEvent,
        BroadcastRecvSpec::Lagged => BroadcastActionSpec::ContinueAfterLag,
        BroadcastRecvSpec::Closed => BroadcastActionSpec::Break,
    }
}

pub open spec fn client_message_dispatch(kind: ClientMessageKindSpec) -> ClientDispatchSpec {
    match kind {
        ClientMessageKindSpec::Event => ClientDispatchSpec::HandleEvent,
        ClientMessageKindSpec::Req => ClientDispatchSpec::HandleReq,
        ClientMessageKindSpec::Close => ClientDispatchSpec::HandleClose,
        ClientMessageKindSpec::Auth => ClientDispatchSpec::HandleAuth,
        ClientMessageKindSpec::Unsupported => ClientDispatchSpec::NoticeUnsupported,
    }
}

pub open spec fn write_policy_allows(policy: WritePolicySpec, authenticated: bool) -> bool {
    match policy {
        WritePolicySpec::Open => true,
        WritePolicySpec::AuthRequired => authenticated,
        WritePolicySpec::ReadOnly => false,
    }
}

pub open spec fn event_admission(
    ip_rate_limited: bool,
    policy: WritePolicySpec,
    authenticated: bool,
    signature_valid: bool,
    pubkey_rate_limited: bool,
) -> EventAdmissionSpec {
    if ip_rate_limited {
        EventAdmissionSpec::RejectIpRateLimited
    } else if policy == WritePolicySpec::ReadOnly {
        EventAdmissionSpec::RejectReadOnly
    } else if policy == WritePolicySpec::AuthRequired && !authenticated {
        EventAdmissionSpec::RejectAuthRequired
    } else if !signature_valid {
        EventAdmissionSpec::RejectInvalidSignature
    } else if pubkey_rate_limited {
        EventAdmissionSpec::RejectPubkeyRateLimited
    } else {
        EventAdmissionSpec::StoreAndRespond
    }
}

pub open spec fn store_response(store_ok: bool, is_new: bool) -> StoreResponseSpec {
    if !store_ok {
        StoreResponseSpec::ErrorNoBroadcast
    } else if is_new {
        StoreResponseSpec::OkBroadcast
    } else {
        StoreResponseSpec::OkDuplicateNoBroadcast
    }
}

pub open spec fn response_broadcasts(response: StoreResponseSpec) -> bool {
    response == StoreResponseSpec::OkBroadcast
}

pub fn text_size_admitted_exec(text_len: u32) -> (admitted: bool)
    ensures admitted == text_size_admitted(text_len)
{
    text_len <= MAX_EVENT_SIZE
}

pub fn websocket_frame_action_exec(frame: WebSocketFrameSpec, text_len: u32) -> (action: FrameActionSpec)
    ensures action == websocket_frame_action(frame, text_len)
{
    match frame {
        WebSocketFrameSpec::Text => if text_size_admitted_exec(text_len) {
            FrameActionSpec::DispatchText
        } else {
            FrameActionSpec::NoticeTooLarge
        },
        WebSocketFrameSpec::Close => FrameActionSpec::Break,
        WebSocketFrameSpec::EndOfStream => FrameActionSpec::Break,
        WebSocketFrameSpec::Ping => FrameActionSpec::SendPong,
        WebSocketFrameSpec::Other => FrameActionSpec::Ignore,
        WebSocketFrameSpec::Error => FrameActionSpec::Break,
    }
}

pub fn client_message_dispatch_exec(kind: ClientMessageKindSpec) -> (dispatch: ClientDispatchSpec)
    ensures dispatch == client_message_dispatch(kind)
{
    match kind {
        ClientMessageKindSpec::Event => ClientDispatchSpec::HandleEvent,
        ClientMessageKindSpec::Req => ClientDispatchSpec::HandleReq,
        ClientMessageKindSpec::Close => ClientDispatchSpec::HandleClose,
        ClientMessageKindSpec::Auth => ClientDispatchSpec::HandleAuth,
        ClientMessageKindSpec::Unsupported => ClientDispatchSpec::NoticeUnsupported,
    }
}

pub fn event_admission_exec(
    ip_rate_limited: bool,
    policy: WritePolicySpec,
    authenticated: bool,
    signature_valid: bool,
    pubkey_rate_limited: bool,
) -> (admission: EventAdmissionSpec)
    ensures admission == event_admission(ip_rate_limited, policy, authenticated, signature_valid, pubkey_rate_limited)
{
    if ip_rate_limited {
        EventAdmissionSpec::RejectIpRateLimited
    } else {
        match policy {
            WritePolicySpec::ReadOnly => EventAdmissionSpec::RejectReadOnly,
            WritePolicySpec::AuthRequired => if !authenticated {
                EventAdmissionSpec::RejectAuthRequired
            } else if !signature_valid {
                EventAdmissionSpec::RejectInvalidSignature
            } else if pubkey_rate_limited {
                EventAdmissionSpec::RejectPubkeyRateLimited
            } else {
                EventAdmissionSpec::StoreAndRespond
            },
            WritePolicySpec::Open => if !signature_valid {
                EventAdmissionSpec::RejectInvalidSignature
            } else if pubkey_rate_limited {
                EventAdmissionSpec::RejectPubkeyRateLimited
            } else {
                EventAdmissionSpec::StoreAndRespond
            },
        }
    }
}

pub proof fn text_at_max_size_is_dispatched()
    ensures websocket_frame_action(WebSocketFrameSpec::Text, MAX_EVENT_SIZE) == FrameActionSpec::DispatchText
{
}

pub proof fn text_one_byte_over_max_is_rejected()
    ensures websocket_frame_action(WebSocketFrameSpec::Text, (MAX_EVENT_SIZE + 1) as u32) == FrameActionSpec::NoticeTooLarge
{
}

pub proof fn oversized_text_does_not_dispatch(text_len: u32)
    requires text_len > MAX_EVENT_SIZE
    ensures websocket_frame_action(WebSocketFrameSpec::Text, text_len) == FrameActionSpec::NoticeTooLarge
{
}

pub proof fn admitted_text_dispatches(text_len: u32)
    requires text_size_admitted(text_len)
    ensures websocket_frame_action(WebSocketFrameSpec::Text, text_len) == FrameActionSpec::DispatchText
{
}

pub proof fn close_error_and_end_break(text_len: u32)
    ensures
        websocket_frame_action(WebSocketFrameSpec::Close, text_len) == FrameActionSpec::Break,
        websocket_frame_action(WebSocketFrameSpec::Error, text_len) == FrameActionSpec::Break,
        websocket_frame_action(WebSocketFrameSpec::EndOfStream, text_len) == FrameActionSpec::Break,
{
}

pub proof fn ping_and_other_do_not_dispatch(text_len: u32)
    ensures
        websocket_frame_action(WebSocketFrameSpec::Ping, text_len) == FrameActionSpec::SendPong,
        websocket_frame_action(WebSocketFrameSpec::Other, text_len) == FrameActionSpec::Ignore,
{
}

pub proof fn broadcast_lag_continues_while_closed_breaks()
    ensures
        broadcast_recv_action(BroadcastRecvSpec::Lagged) == BroadcastActionSpec::ContinueAfterLag,
        broadcast_recv_action(BroadcastRecvSpec::Closed) == BroadcastActionSpec::Break,
        broadcast_recv_action(BroadcastRecvSpec::Event) == BroadcastActionSpec::PushMatchingEvent,
{
}

pub proof fn known_client_messages_dispatch_to_handlers()
    ensures
        client_message_dispatch(ClientMessageKindSpec::Event) == ClientDispatchSpec::HandleEvent,
        client_message_dispatch(ClientMessageKindSpec::Req) == ClientDispatchSpec::HandleReq,
        client_message_dispatch(ClientMessageKindSpec::Close) == ClientDispatchSpec::HandleClose,
        client_message_dispatch(ClientMessageKindSpec::Auth) == ClientDispatchSpec::HandleAuth,
{
}

pub proof fn unsupported_client_message_gets_notice()
    ensures client_message_dispatch(ClientMessageKindSpec::Unsupported) == ClientDispatchSpec::NoticeUnsupported
{
}

pub proof fn ip_rate_limit_rejected_before_policy_and_signature(
    policy: WritePolicySpec,
    authenticated: bool,
    signature_valid: bool,
    pubkey_rate_limited: bool,
)
    ensures event_admission(true, policy, authenticated, signature_valid, pubkey_rate_limited)
        == EventAdmissionSpec::RejectIpRateLimited
{
}

pub proof fn read_only_rejected_before_auth_and_signature(
    authenticated: bool,
    signature_valid: bool,
    pubkey_rate_limited: bool,
)
    ensures event_admission(false, WritePolicySpec::ReadOnly, authenticated, signature_valid, pubkey_rate_limited)
        == EventAdmissionSpec::RejectReadOnly
{
}

pub proof fn auth_required_rejects_unauthenticated_before_signature(signature_valid: bool, pubkey_rate_limited: bool)
    ensures event_admission(false, WritePolicySpec::AuthRequired, false, signature_valid, pubkey_rate_limited)
        == EventAdmissionSpec::RejectAuthRequired
{
}

pub proof fn invalid_signature_rejected_before_pubkey_rate_limit(policy: WritePolicySpec, authenticated: bool, pubkey_rate_limited: bool)
    requires write_policy_allows(policy, authenticated)
    ensures event_admission(false, policy, authenticated, false, pubkey_rate_limited)
        == EventAdmissionSpec::RejectInvalidSignature
{
}

pub proof fn pubkey_rate_limit_rejected_after_valid_signature(policy: WritePolicySpec, authenticated: bool)
    requires write_policy_allows(policy, authenticated)
    ensures event_admission(false, policy, authenticated, true, true)
        == EventAdmissionSpec::RejectPubkeyRateLimited
{
}

pub proof fn fully_admitted_event_stores(policy: WritePolicySpec, authenticated: bool)
    requires write_policy_allows(policy, authenticated)
    ensures event_admission(false, policy, authenticated, true, false) == EventAdmissionSpec::StoreAndRespond
{
}

pub proof fn successful_new_store_broadcasts()
    ensures
        store_response(true, true) == StoreResponseSpec::OkBroadcast,
        response_broadcasts(store_response(true, true)),
{
}

pub proof fn duplicate_or_error_store_does_not_broadcast(store_ok: bool, is_new: bool)
    requires !store_ok || !is_new
    ensures !response_broadcasts(store_response(store_ok, is_new))
{
}

} // verus!
