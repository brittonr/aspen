//! Verus specs for Nostr relay HTTP request classification.
//!
//! Production classifier lives in `src/relay.rs::classify_http_request`: valid
//! UTF-8 GET/HEAD requests are classified as NIP-11 JSON, WebSocket upgrade, or
//! reject, while non-HTTP/binary input returns `None`. These specs model the
//! control-flow decision from precomputed parser predicates instead of pulling
//! string scanning or async TCP into Verus.

use vstd::prelude::*;

verus! {

pub const HTTP_METHOD_OTHER: u8 = 0;
pub const HTTP_METHOD_GET: u8 = 1;
pub const HTTP_METHOD_HEAD: u8 = 2;

pub enum HttpActionSpec {
    Nip11,
    WebSocket,
    Reject,
}

pub open spec fn is_supported_http_method(method: u8) -> bool {
    method == HTTP_METHOD_GET || method == HTTP_METHOD_HEAD
}

pub open spec fn classify_http_request_spec(
    utf8_ok: bool,
    method: u8,
    has_nostr_accept: bool,
    has_ws_upgrade: bool,
) -> Option<HttpActionSpec> {
    if !utf8_ok || !is_supported_http_method(method) {
        None::<HttpActionSpec>
    } else if has_nostr_accept && !has_ws_upgrade {
        Some(HttpActionSpec::Nip11)
    } else if has_ws_upgrade {
        Some(HttpActionSpec::WebSocket)
    } else {
        Some(HttpActionSpec::Reject)
    }
}

pub open spec fn is_nip11_request_spec(
    utf8_ok: bool,
    method: u8,
    has_nostr_accept: bool,
    has_ws_upgrade: bool,
) -> bool {
    classify_http_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)
        == Some(HttpActionSpec::Nip11)
}

pub open spec fn is_websocket_request_spec(
    utf8_ok: bool,
    method: u8,
    has_nostr_accept: bool,
    has_ws_upgrade: bool,
) -> bool {
    classify_http_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)
        == Some(HttpActionSpec::WebSocket)
}

pub open spec fn is_rejected_http_request_spec(
    utf8_ok: bool,
    method: u8,
    has_nostr_accept: bool,
    has_ws_upgrade: bool,
) -> bool {
    classify_http_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)
        == Some(HttpActionSpec::Reject)
}

pub fn is_supported_http_method_exec(method: u8) -> (supported: bool)
    ensures supported == is_supported_http_method(method)
{
    method == HTTP_METHOD_GET || method == HTTP_METHOD_HEAD
}

pub fn should_serve_nip11_exec(utf8_ok: bool, method: u8, has_nostr_accept: bool, has_ws_upgrade: bool) -> (serve: bool)
    ensures serve == is_nip11_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)
{
    utf8_ok && (method == HTTP_METHOD_GET || method == HTTP_METHOD_HEAD) && has_nostr_accept && !has_ws_upgrade
}

pub fn should_upgrade_websocket_exec(utf8_ok: bool, method: u8, has_ws_upgrade: bool) -> (upgrade: bool)
    ensures upgrade == is_websocket_request_spec(utf8_ok, method, false, has_ws_upgrade)
{
    utf8_ok && (method == HTTP_METHOD_GET || method == HTTP_METHOD_HEAD) && has_ws_upgrade
}

pub proof fn unsupported_or_binary_requests_are_unclassified(
    utf8_ok: bool,
    method: u8,
    has_nostr_accept: bool,
    has_ws_upgrade: bool,
)
    requires !utf8_ok || !is_supported_http_method(method)
    ensures classify_http_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade) == None::<HttpActionSpec>
{
}

pub proof fn get_and_head_are_supported()
    ensures
        is_supported_http_method(HTTP_METHOD_GET),
        is_supported_http_method(HTTP_METHOD_HEAD),
        !is_supported_http_method(HTTP_METHOD_OTHER),
{
}

pub proof fn nip11_requires_supported_text_request(
    utf8_ok: bool,
    method: u8,
    has_nostr_accept: bool,
    has_ws_upgrade: bool,
)
    requires is_nip11_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)
    ensures
        utf8_ok,
        is_supported_http_method(method),
        has_nostr_accept,
        !has_ws_upgrade,
{
}

pub proof fn websocket_upgrade_takes_precedence_over_nip11_accept(method: u8)
    requires is_supported_http_method(method)
    ensures classify_http_request_spec(true, method, true, true) == Some(HttpActionSpec::WebSocket)
{
}

pub proof fn nip11_accept_without_upgrade_serves_json(method: u8)
    requires is_supported_http_method(method)
    ensures classify_http_request_spec(true, method, true, false) == Some(HttpActionSpec::Nip11)
{
}

pub proof fn plain_supported_http_is_rejected(method: u8)
    requires is_supported_http_method(method)
    ensures classify_http_request_spec(true, method, false, false) == Some(HttpActionSpec::Reject)
{
}

pub proof fn classified_http_actions_are_total_for_supported_text_requests(
    method: u8,
    has_nostr_accept: bool,
    has_ws_upgrade: bool,
)
    requires is_supported_http_method(method)
    ensures classify_http_request_spec(true, method, has_nostr_accept, has_ws_upgrade).is_some()
{
}

pub proof fn action_classes_are_disjoint(
    utf8_ok: bool,
    method: u8,
    has_nostr_accept: bool,
    has_ws_upgrade: bool,
)
    ensures
        !(is_nip11_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)
            && is_websocket_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)),
        !(is_nip11_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)
            && is_rejected_http_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)),
        !(is_websocket_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)
            && is_rejected_http_request_spec(utf8_ok, method, has_nostr_accept, has_ws_upgrade)),
{
}

pub proof fn websocket_classification_ignores_accept_header_when_upgrade_present(method: u8, has_nostr_accept: bool)
    requires is_supported_http_method(method)
    ensures classify_http_request_spec(true, method, has_nostr_accept, true) == Some(HttpActionSpec::WebSocket)
{
}

} // verus!
