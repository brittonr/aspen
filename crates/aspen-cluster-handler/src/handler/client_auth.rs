//! Client and docs ticket generation handlers.
//!
//! Handles: GetClientTicket, GetDocsTicket.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ClientTicketResponse;
use aspen_client_api::DocsTicketResponse;
use aspen_rpc_core::ClientProtocolContext;

pub(crate) async fn handle_get_client_ticket(
    ctx: &ClientProtocolContext,
    access: String,
    priority: u32,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client::AccessLevel;
    use aspen_client::AspenClientTicket;

    let endpoint_addr = ctx.endpoint_manager.node_addr().clone();
    let access_level = match access.to_lowercase().as_str() {
        "write" | "readwrite" | "read_write" | "rw" => AccessLevel::ReadWrite,
        _ => AccessLevel::ReadOnly,
    };

    // Tiger Style: saturate priority to u8 range
    let priority_u8 = priority.min(255) as u8;

    let ticket = AspenClientTicket::new(&ctx.cluster_cookie, vec![endpoint_addr])
        .with_access(access_level)
        .with_priority(priority_u8);

    let ticket_str = ticket.serialize();

    Ok(ClientRpcResponse::ClientTicket(ClientTicketResponse {
        ticket: ticket_str,
        cluster_id: ctx.cluster_cookie.clone(),
        access: match access_level {
            AccessLevel::ReadOnly => "read".to_string(),
            AccessLevel::ReadWrite => "write".to_string(),
        },
        priority,
        endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
        error: None,
    }))
}

#[cfg(feature = "docs")]
pub(crate) async fn handle_get_docs_ticket(
    ctx: &ClientProtocolContext,
    read_write: bool,
    priority: u8,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_docs::ticket::AspenDocsTicket;

    let endpoint_addr = ctx.endpoint_manager.node_addr().clone();

    // Derive namespace ID from cluster cookie
    let namespace_hash = blake3::hash(format!("aspen-docs-{}", ctx.cluster_cookie).as_bytes());
    let namespace_id_str = format!("{}", namespace_hash);

    let ticket = AspenDocsTicket::new(
        ctx.cluster_cookie.clone(),
        priority,
        namespace_id_str.clone(),
        vec![endpoint_addr],
        read_write,
    );

    let ticket_str = ticket.serialize();

    Ok(ClientRpcResponse::DocsTicket(DocsTicketResponse {
        ticket: ticket_str,
        cluster_id: ctx.cluster_cookie.clone(),
        namespace_id: namespace_id_str,
        read_write,
        priority,
        endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
        error: None,
    }))
}

#[cfg(not(feature = "docs"))]
pub(crate) async fn handle_get_docs_ticket(
    _ctx: &ClientProtocolContext,
    _read_write: bool,
    _priority: u8,
) -> anyhow::Result<ClientRpcResponse> {
    Ok(ClientRpcResponse::DocsTicket(DocsTicketResponse {
        ticket: String::new(),
        cluster_id: String::new(),
        namespace_id: String::new(),
        read_write: false,
        priority: 0,
        endpoint_id: String::new(),
        error: Some("docs feature not enabled".to_string()),
    }))
}
